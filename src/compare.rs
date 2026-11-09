//! Geometry of the two-pane compare view. Pane 0 is the main viewer, pane 1
//! the compared one; every rect is relative to the image area.

use crate::render::{FitMode, ViewTransform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Two viewers side by side (or stacked when `vertical`).
    Tiles,
    /// Both viewers share the whole area; a movable cut shows pane 0 before
    /// the cut and pane 1 after it.
    Slider,
}

/// Gap between tiles, in logical pixels.
pub const TILE_GAP: f32 = 2.0;
/// Half-width of the slider's grab zone around the cut line.
pub const SPLIT_GRAB: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn local(&self, point: (f32, f32)) -> (f32, f32) {
        (point.0 - self.x, point.1 - self.y)
    }
}

/// Viewport of each pane. `vertical` stacks tiles top/bottom, or makes the
/// slider cut horizontal.
pub fn pane_rects(layout: Layout, vertical: bool, area: (f32, f32)) -> [Rect; 2] {
    let (w, h) = (area.0.max(1.0), area.1.max(1.0));
    match layout {
        Layout::Slider => [Rect { x: 0.0, y: 0.0, w, h }; 2],
        Layout::Tiles if vertical => {
            let first = ((h - TILE_GAP) / 2.0).floor().max(1.0);
            let second_y = first + TILE_GAP;
            [
                Rect { x: 0.0, y: 0.0, w, h: first },
                Rect { x: 0.0, y: second_y, w, h: (h - second_y).max(1.0) },
            ]
        }
        Layout::Tiles => {
            let first = ((w - TILE_GAP) / 2.0).floor().max(1.0);
            let second_x = first + TILE_GAP;
            [
                Rect { x: 0.0, y: 0.0, w: first, h },
                Rect { x: second_x, y: 0.0, w: (w - second_x).max(1.0), h },
            ]
        }
    }
}

/// Position of the slider cut along its axis, in logical pixels.
pub fn split_px(split: f32, vertical: bool, area: (f32, f32)) -> f32 {
    let len = if vertical { area.1 } else { area.0 };
    (split.clamp(0.0, 1.0) * len).round()
}

/// Split fraction that puts the cut under `point`.
pub fn split_at(point: (f32, f32), vertical: bool, area: (f32, f32)) -> f32 {
    let (pos, len) = if vertical { (point.1, area.1) } else { (point.0, area.0) };
    if len <= 0.0 { 0.5 } else { (pos / len).clamp(0.0, 1.0) }
}

pub fn near_split(point: (f32, f32), split: f32, vertical: bool, area: (f32, f32)) -> bool {
    let pos = if vertical { point.1 } else { point.0 };
    (pos - split_px(split, vertical, area)).abs() <= SPLIT_GRAB
}

/// Pane under `point`.
pub fn pane_at(
    layout: Layout,
    vertical: bool,
    split: f32,
    area: (f32, f32),
    point: (f32, f32),
) -> usize {
    let pos = if vertical { point.1 } else { point.0 };
    let boundary = match layout {
        Layout::Slider => split_px(split, vertical, area),
        Layout::Tiles => {
            let second = pane_rects(layout, vertical, area)[1];
            if vertical { second.y } else { second.x }
        }
    };
    usize::from(pos >= boundary)
}

/// What a pane is aligned to the other pane by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Same on-screen size and placement: frames of different resolutions overlay.
    Frame,
    /// Same scale of source pixels, centred on the same relative point.
    Pixels,
}

/// One pane's state as needed to align another pane to it.
#[derive(Debug, Clone, Copy)]
pub struct PaneView {
    pub transform: ViewTransform,
    /// Size the transform's zoom is relative to (logical px at zoom 1).
    pub intrinsic: (f32, f32),
    /// Decoded size in source pixels.
    pub source: (u32, u32),
}

/// Transform for `target` so it matches `reference` under `align`.
/// Returns `None` when either pane has no image.
pub fn aligned(reference: PaneView, target: PaneView, align: Align) -> Option<ViewTransform> {
    let (ri, ti) = (reference.intrinsic, target.intrinsic);
    if ri.0 <= 0.0 || ri.1 <= 0.0 || ti.0 <= 0.0 || ti.1 <= 0.0 {
        return None;
    }
    let zoom = match align {
        Align::Frame => reference.transform.zoom * ri.0 / ti.0,
        Align::Pixels => {
            if reference.source.0 == 0 || target.source.0 == 0 {
                return None;
            }
            // Displayed px per source px must match on both panes.
            let per_source = reference.transform.zoom * ri.0 / reference.source.0 as f32;
            per_source * target.source.0 as f32 / ti.0
        }
    };
    let zoom = zoom.clamp(0.01, 100.0);
    // Keep the same relative image point at the viewport centre.
    let scale = (
        (ti.0 * zoom) / (ri.0 * reference.transform.zoom),
        (ti.1 * zoom) / (ri.1 * reference.transform.zoom),
    );
    let offset = (reference.transform.offset.0 * scale.0, reference.transform.offset.1 * scale.1);
    Some(ViewTransform { zoom, offset, fit: FitMode::Free })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(zoom: f32, offset: (f32, f32), intrinsic: (f32, f32), source: (u32, u32)) -> PaneView {
        PaneView {
            transform: ViewTransform { zoom, offset, fit: FitMode::Free },
            intrinsic,
            source,
        }
    }

    #[test]
    fn tiles_split_the_area_without_overlap() {
        let [a, b] = pane_rects(Layout::Tiles, false, (1001.0, 600.0));
        assert_eq!(a, Rect { x: 0.0, y: 0.0, w: 499.0, h: 600.0 });
        assert_eq!(b, Rect { x: 501.0, y: 0.0, w: 500.0, h: 600.0 });
        let [a, b] = pane_rects(Layout::Tiles, true, (800.0, 600.0));
        assert_eq!((a.h, b.y, b.h), (299.0, 301.0, 299.0));
    }

    #[test]
    fn slider_panes_share_the_area_and_split_by_the_cut() {
        let area = (1000.0, 500.0);
        let [a, b] = pane_rects(Layout::Slider, false, area);
        assert_eq!(a, b);
        assert_eq!(pane_at(Layout::Slider, false, 0.3, area, (299.0, 10.0)), 0);
        assert_eq!(pane_at(Layout::Slider, false, 0.3, area, (300.0, 10.0)), 1);
        assert_eq!(pane_at(Layout::Slider, true, 0.3, area, (900.0, 140.0)), 0);
        assert!(near_split((305.0, 400.0), 0.3, false, area));
        assert!(!near_split((320.0, 400.0), 0.3, false, area));
        assert!((split_at((250.0, 0.0), false, area) - 0.25).abs() < 1e-6);
        assert_eq!(split_at((-50.0, 0.0), false, area), 0.0);
    }

    #[test]
    fn tiles_hit_test_uses_the_second_rect() {
        let area = (1001.0, 600.0);
        assert_eq!(pane_at(Layout::Tiles, false, 0.5, area, (498.0, 1.0)), 0);
        assert_eq!(pane_at(Layout::Tiles, false, 0.5, area, (501.0, 1.0)), 1);
    }

    #[test]
    fn frame_align_matches_displayed_size_and_offset() {
        let reference = view(2.0, (40.0, -10.0), (400.0, 300.0), (1920, 1440));
        let target = view(1.0, (0.0, 0.0), (200.0, 150.0), (1280, 960));
        let t = aligned(reference, target, Align::Frame).unwrap();
        assert!((t.zoom * 200.0 - 2.0 * 400.0).abs() < 1e-3);
        assert_eq!(t.offset, (40.0, -10.0));
        assert_eq!(t.fit, FitMode::Free);
    }

    #[test]
    fn pixel_align_matches_source_pixel_scale() {
        let reference = view(1.0, (30.0, 0.0), (400.0, 300.0), (800, 600));
        let target = view(1.0, (0.0, 0.0), (400.0, 300.0), (1600, 1200));
        let t = aligned(reference, target, Align::Pixels).unwrap();
        // 0.5 displayed px per source px on both sides.
        assert!((t.zoom * 400.0 / 1600.0 - 400.0 / 800.0).abs() < 1e-4);
        // Target shows twice as big, so the same relative point needs twice the offset.
        assert!((t.offset.0 - 60.0).abs() < 1e-3);
    }

    #[test]
    fn align_without_an_image_is_a_no_op() {
        let empty = view(1.0, (0.0, 0.0), (0.0, 0.0), (0, 0));
        let some = view(1.0, (0.0, 0.0), (10.0, 10.0), (10, 10));
        assert!(aligned(empty, some, Align::Frame).is_none());
        assert!(aligned(some, empty, Align::Pixels).is_none());
    }
}
