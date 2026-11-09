use std::sync::Arc;

use gpui::{App, RenderImage, Window};
use reveal::compare::{self, Align, Layout, PaneView, Rect};
use reveal::playback::PlaybackState;
use reveal::render::ImageElement;
use reveal::viewer::Viewer;

/// The compared pane's geometry. The second viewer is kept after compare is
/// turned off, so its folder, position and zoom come back when it is re-entered.
#[derive(Debug, Clone, Copy)]
pub struct CompareState {
    /// `None` while compare is hidden; the rest of the state is retained so
    /// re-entering compare restores the split and orientation.
    pub layout: Option<Layout>,
    pub vertical: bool,
    /// Slider cut position as a fraction of the image area.
    pub split: f32,
    pub dragging_split: bool,
}

impl CompareState {
    fn new(layout: Layout) -> Self {
        Self { layout: Some(layout), vertical: false, split: 0.5, dragging_split: false }
    }
}

pub const MAX_PANES: usize = 2;

/// Owns every viewer and the single active-pane index. Pane 0 is the main
/// viewer; pane 1 exists only after compare has been entered once.
pub struct Panes {
    main: Viewer,
    second: Option<Viewer>,
    compare: Option<CompareState>,
    active: usize,
    /// Area below the toolbar that holds the panes, in logical pixels.
    area: (f32, f32),
    painted: [Option<Arc<RenderImage>>; MAX_PANES],
}

fn pane_view(viewer: &Viewer) -> PaneView {
    PaneView {
        transform: viewer.transform(),
        intrinsic: viewer.intrinsic(),
        source: viewer.source_size(),
    }
}

impl Panes {
    pub fn new(main: Viewer) -> Self {
        Self {
            main,
            second: None,
            compare: None,
            active: 0,
            area: (0.0, 0.0),
            painted: [const { None }; MAX_PANES],
        }
    }

    pub fn main(&self) -> &Viewer {
        &self.main
    }

    pub fn viewer(&self, pane: usize) -> &Viewer {
        match pane {
            0 => &self.main,
            1 => self.second.as_ref().expect("pane 1 exists only after compare is opened"),
            _ => panic!("pane index {pane} is out of range"),
        }
    }

    pub fn viewer_mut(&mut self, pane: usize) -> &mut Viewer {
        match pane {
            0 => &mut self.main,
            1 => self.second.as_mut().expect("pane 1 exists only after compare is opened"),
            _ => panic!("pane index {pane} is out of range"),
        }
    }

    pub fn set_area(&mut self, width: f32, height: f32) {
        self.area = (width, height);
    }

    pub fn area(&self) -> (f32, f32) {
        self.area
    }

    pub fn existing_count(&self) -> usize {
        if self.second.is_some() { 2 } else { 1 }
    }

    pub fn visible_count(&self) -> usize {
        if self.is_comparing() { 2 } else { 1 }
    }

    pub fn is_comparing(&self) -> bool {
        self.layout().is_some()
    }

    pub fn has_second(&self) -> bool {
        self.second.is_some()
    }

    pub fn layout(&self) -> Option<Layout> {
        self.compare.and_then(|c| c.layout)
    }

    pub fn active_index(&self) -> usize {
        if self.visible_count() == MAX_PANES { self.active } else { 0 }
    }

    pub fn set_active(&mut self, pane: usize) -> bool {
        if pane >= self.visible_count() || pane == self.active_index() {
            return false;
        }
        self.active = pane;
        true
    }

    pub fn active(&self) -> &Viewer {
        self.viewer(self.active_index())
    }

    pub fn active_mut(&mut self) -> &mut Viewer {
        let index = self.active_index();
        self.viewer_mut(index)
    }

    /// Panes an action applies to: `only` one, or every visible pane.
    pub fn targets(&self, only: Option<usize>) -> Vec<usize> {
        match only {
            Some(pane) if pane < self.visible_count() => vec![pane],
            _ => (0..self.visible_count()).collect(),
        }
    }

    pub fn install_second(&mut self, viewer: Viewer, layout: Layout) {
        self.second = Some(viewer);
        self.compare = Some(CompareState::new(layout));
        self.active = 0;
    }

    pub fn compare(&self) -> Option<&CompareState> {
        self.compare.as_ref()
    }

    /// Turns a compare layout on, or off when it is already the shown one.
    /// The second viewer is never discarded by turning compare off.
    pub fn toggle_compare(&mut self, layout: Layout) {
        match self.compare.as_mut() {
            Some(compare) if compare.layout == Some(layout) => {
                compare.layout = None;
                compare.dragging_split = false;
                self.active = 0;
                if let Some(second) = self.second.as_mut() {
                    second.set_playback(PlaybackState::Paused);
                }
            }
            Some(compare) => {
                compare.layout = Some(layout);
                compare.dragging_split = false;
                self.active = 0;
            }
            None => {
                self.compare = Some(CompareState::new(layout));
                self.active = 0;
            }
        }
    }

    pub fn flip(&mut self) {
        if let Some(compare) = self.compare.as_mut() {
            compare.vertical = !compare.vertical;
        }
    }

    pub fn pane_rects(&self) -> [Rect; 2] {
        match self
            .compare
            .and_then(|compare| compare.layout.map(|layout| (layout, compare.vertical)))
        {
            Some((layout, vertical)) => compare::pane_rects(layout, vertical, self.area),
            None => {
                let rect =
                    Rect { x: 0.0, y: 0.0, w: self.area.0.max(1.0), h: self.area.1.max(1.0) };
                [rect; 2]
            }
        }
    }

    pub fn pane_at(&self, point: (f32, f32)) -> usize {
        match self.compare.and_then(|compare| {
            compare.layout.map(|layout| (layout, compare.vertical, compare.split))
        }) {
            Some((layout, vertical, split)) => {
                compare::pane_at(layout, vertical, split, self.area, point)
            }
            None => 0,
        }
    }

    pub fn pane_local(&self, pane: usize, point: (f32, f32)) -> (f32, f32) {
        if !self.is_comparing() {
            return point;
        }
        self.pane_rects().get(pane).map_or(point, |rect| rect.local(point))
    }

    pub fn in_image_area(&self, point: (f32, f32)) -> bool {
        point.0 >= 0.0 && point.1 >= 0.0 && point.0 < self.area.0 && point.1 < self.area.1
    }

    /// Makes the pane under `point` the active one; true when it changed.
    pub fn hover_pane(&mut self, point: (f32, f32)) -> bool {
        if !self.is_comparing() || !self.in_image_area(point) {
            return false;
        }
        self.set_active(self.pane_at(point))
    }

    pub fn near_split(&self, point: (f32, f32)) -> bool {
        match self.compare.filter(|compare| compare.layout == Some(Layout::Slider)) {
            Some(compare) => compare::near_split(point, compare.split, compare.vertical, self.area),
            None => false,
        }
    }

    pub fn move_split(&mut self, point: (f32, f32)) {
        let vertical = self.compare.is_some_and(|c| c.vertical);
        if let Some(compare) = self.compare.as_mut()
            && compare.layout.is_some()
        {
            compare.split = compare::split_at(point, vertical, self.area);
        }
    }

    pub fn recentre_split(&mut self) {
        if let Some(compare) = self.compare.as_mut() {
            compare.split = 0.5;
        }
    }

    pub fn begin_split_drag(&mut self) {
        if let Some(compare) = self.compare.as_mut() {
            compare.dragging_split = true;
        }
    }

    pub fn end_split_drag(&mut self) -> bool {
        self.compare.as_mut().is_some_and(|compare| std::mem::take(&mut compare.dragging_split))
    }

    /// Aligns the other pane to the active one.
    pub fn align_active_to_other(&mut self, align: Align) {
        let Some(_) = self.compare else {
            return;
        };
        let active = self.active_index();
        let (reference, target) = if active == 0 {
            (&self.main, self.second.as_mut().expect("compare has a second viewer"))
        } else {
            (self.second.as_ref().expect("compare has a second viewer"), &mut self.main)
        };
        if let Some(transform) = compare::aligned(pane_view(reference), pane_view(target), align) {
            target.set_transform(transform);
        }
    }

    /// Renders a pane and evicts the previously painted texture for that pane.
    pub fn image_element(
        &mut self,
        pane: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> ImageElement {
        let (image, element) = self.viewer_mut(pane).image_element();
        self.track_painted(pane, &image, window, cx);
        element
    }

    pub fn drop_painted(&mut self, pane: usize, window: &mut Window, cx: &mut App) {
        if let Some(old) = self.painted.get_mut(pane).and_then(Option::take) {
            cx.drop_image(old, Some(window));
        }
    }

    fn track_painted(
        &mut self,
        pane: usize,
        image: &Option<Arc<RenderImage>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let slot = &mut self.painted[pane];
        let replaced = match (&*slot, image) {
            (Some(old), Some(new)) => !Arc::ptr_eq(old, new),
            (old, _) => old.is_some(),
        };
        if replaced && let Some(old) = slot.take() {
            cx.drop_image(old, Some(window));
        }
        slot.clone_from(image);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_compare_off_keeps_the_second_viewer_for_the_next_entry() {
        let mut panes = Panes::new(Viewer::new());
        panes.install_second(Viewer::new(), Layout::Tiles);
        assert_eq!(panes.visible_count(), 2);
        assert_eq!(panes.targets(None), vec![0, 1]);

        panes.toggle_compare(Layout::Tiles);
        assert!(!panes.is_comparing());
        assert!(panes.has_second(), "closing compare must not discard the second viewer");
        assert_eq!(panes.visible_count(), 1);
        assert_eq!(panes.targets(None), vec![0]);

        panes.toggle_compare(Layout::Slider);
        assert_eq!(panes.layout(), Some(Layout::Slider));
        assert_eq!(panes.visible_count(), 2);

        panes.set_area(1000.0, 500.0);
        panes.move_split((250.0, 0.0));
        panes.flip();
        panes.toggle_compare(Layout::Slider);
        assert!(!panes.is_comparing());
        panes.toggle_compare(Layout::Slider);
        let compare = panes.compare().expect("compare state is retained");
        assert_eq!(compare.split, 0.25, "the slider cut survives a close/reopen");
        assert!(compare.vertical, "the flip survives a close/reopen");
    }

    #[test]
    fn active_pane_is_only_used_while_compare_is_visible() {
        let mut panes = Panes::new(Viewer::new());
        panes.install_second(Viewer::new(), Layout::Tiles);
        assert!(panes.set_active(1));
        assert_eq!(panes.active_index(), 1);
        panes.toggle_compare(Layout::Tiles);
        assert_eq!(panes.active_index(), 0);
        assert!(!panes.set_active(1), "a hidden pane cannot become active");
    }
}
