use std::path::Path;
use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, Context, Div, IntoElement, ParentElement, RenderImage, Styled, Window, div, px,
};
use reveal::compare::{self, Align, Layout, PaneView, Rect};
use reveal::input::Action;
use reveal::playback::PlaybackState;
use reveal::render::ImageElement;
use reveal::ui::{self, Palette};
use reveal::viewer::Viewer;

use super::RevealApp;

/// Second viewer of the compare view. It stays alive after compare is turned
/// off so its folder, position and zoom come back when compare is re-entered.
pub struct Compare {
    pub viewer: Viewer,
    pub layout: Option<Layout>,
    pub vertical: bool,
    /// Slider cut position as a fraction of the image area.
    pub split: f32,
    /// Pane that keyboard Ctrl-actions, drops and dialogs go to.
    pub active: usize,
    pub dragging_split: bool,
    pub painted: Option<Arc<RenderImage>>,
}

/// Actions that act on each pane independently; Ctrl limits them to one pane.
pub fn per_pane(action: Action) -> bool {
    matches!(
        action,
        Action::ImgNext
            | Action::ImgPrev
            | Action::ImgOrig
            | Action::ImgFit
            | Action::ImgFitBest
            | Action::PanUp
            | Action::PanDown
            | Action::PanLeft
            | Action::PanRight
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::PlayAnim
            | Action::PlayPresent
            | Action::PlayPresentRandom
    )
}

/// Evicts the previously painted image from gpui's sprite atlas once replaced.
pub fn track_painted(
    slot: &mut Option<Arc<RenderImage>>,
    image: &Option<Arc<RenderImage>>,
    window: &mut Window,
    cx: &mut App,
) {
    let replaced = match (&*slot, image) {
        (Some(old), Some(new)) => !Arc::ptr_eq(old, new),
        (old, _) => old.is_some(),
    };
    if replaced && let Some(old) = slot.take() {
        cx.drop_image(old, Some(window));
    }
    slot.clone_from(image);
}

fn pane_view(viewer: &Viewer) -> PaneView {
    PaneView {
        transform: viewer.view.transform,
        intrinsic: viewer.presentation.current_intrinsic(),
        source: viewer.presentation.current_source_size(),
    }
}

fn image_of(viewer: &mut Viewer) -> (Option<Arc<RenderImage>>, ImageElement) {
    let image = viewer.render_image();
    let element = ImageElement::new(
        image.clone(),
        viewer.presentation.current_intrinsic(),
        viewer.view.transform,
        viewer.playback.frame_index(),
        viewer.presentation.render_crop(),
    );
    (image, element)
}

fn pane_label(viewer: &Viewer) -> String {
    if let Some(status) = viewer.session.status() {
        return status.to_owned();
    }
    match viewer.presentation.current_path() {
        Some(path) => format!(
            "{}  {}/{}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            viewer.session.directory.current_index() + 1,
            viewer.session.directory.len().max(1)
        ),
        None => "Drop or open an image here".to_owned(),
    }
}

fn placed(rect: Rect) -> Div {
    div().absolute().left(px(rect.x)).top(px(rect.y)).w(px(rect.w)).h(px(rect.h))
}

impl RevealApp {
    pub fn layout(&self) -> Option<Layout> {
        self.compare.as_ref().and_then(|c| c.layout)
    }

    /// Viewers on screen: the main one, plus the compared one in compare mode.
    pub fn visible_panes(&self) -> usize {
        if self.layout().is_some() { 2 } else { 1 }
    }

    pub fn pane(&self, index: usize) -> &Viewer {
        match (index, self.compare.as_ref()) {
            (1, Some(c)) => &c.viewer,
            _ => &self.viewer,
        }
    }

    pub fn pane_mut(&mut self, index: usize) -> &mut Viewer {
        match (index, self.compare.as_mut()) {
            (1, Some(c)) => &mut c.viewer,
            _ => &mut self.viewer,
        }
    }

    /// Every existing viewer, shown or not; background results still go to it.
    pub fn existing_panes(&self) -> usize {
        if self.compare.is_some() { 2 } else { 1 }
    }

    pub fn active_pane(&self) -> usize {
        match (self.layout(), self.compare.as_ref()) {
            (Some(_), Some(c)) => c.active,
            _ => 0,
        }
    }

    pub fn active_viewer(&self) -> &Viewer {
        self.pane(self.active_pane())
    }

    pub fn active_viewer_mut(&mut self) -> &mut Viewer {
        self.pane_mut(self.active_pane())
    }

    /// Panes an action applies to: `only` one, or every visible pane.
    pub fn targets(&self, only: Option<usize>) -> Vec<usize> {
        match only {
            Some(pane) if pane < self.visible_panes() => vec![pane],
            _ => (0..self.visible_panes()).collect(),
        }
    }

    pub fn pane_rects(&self) -> [Rect; 2] {
        let layout = self.layout().unwrap_or(Layout::Slider);
        let vertical = self.compare.as_ref().is_some_and(|c| c.vertical);
        compare::pane_rects(layout, vertical, self.image_area)
    }

    pub fn pane_at(&self, point: (f32, f32)) -> usize {
        match (self.layout(), self.compare.as_ref()) {
            (Some(layout), Some(c)) => {
                compare::pane_at(layout, c.vertical, c.split, self.image_area, point)
            }
            _ => 0,
        }
    }

    /// `point` (image-area coordinates) relative to the pane it falls in.
    pub fn pane_local(&self, pane: usize, point: (f32, f32)) -> (f32, f32) {
        if self.layout().is_none() {
            return point;
        }
        self.pane_rects()[pane.min(1)].local(point)
    }

    pub fn in_image_area(&self, point: (f32, f32)) -> bool {
        point.0 >= 0.0
            && point.1 >= 0.0
            && point.0 < self.image_area.0
            && point.1 < self.image_area.1
    }

    /// Makes the pane under `point` the active one; true when it changed.
    pub fn hover_pane(&mut self, point: (f32, f32)) -> bool {
        if self.layout().is_none() || !self.in_image_area(point) {
            return false;
        }
        let pane = self.pane_at(point);
        match self.compare.as_mut() {
            Some(c) if c.active != pane => {
                c.active = pane;
                true
            }
            _ => false,
        }
    }

    pub fn near_split(&self, point: (f32, f32)) -> bool {
        match (self.layout(), self.compare.as_ref()) {
            (Some(Layout::Slider), Some(c)) => {
                compare::near_split(point, c.split, c.vertical, self.image_area)
            }
            _ => false,
        }
    }

    pub fn move_split(&mut self, point: (f32, f32)) {
        let area = self.image_area;
        if let Some(c) = self.compare.as_mut() {
            c.split = compare::split_at(point, c.vertical, area);
        }
    }

    /// Turns `layout` on, or off when it is already shown.
    pub fn toggle_compare(&mut self, layout: Layout, cx: &mut Context<Self>) {
        if let Some(c) = self.compare.as_mut() {
            c.layout = (c.layout != Some(layout)).then_some(layout);
            c.dragging_split = false;
            if c.layout.is_none() {
                c.active = 0;
                c.viewer.playback.set_state(PlaybackState::Paused);
            }
            return;
        }
        let mut viewer = Viewer::new();
        viewer.set_antialias(self.viewer.view.antialias());
        viewer.set_scale_factor(self.viewer.view.scale_factor);
        let (w, h) = self.viewer.view.viewport;
        viewer.set_viewport(w, h);
        // Start on the same image so a second folder can be dropped next to it.
        if let Some(path) = self.viewer.session.directory.current().map(Path::to_path_buf)
            && let Err(e) = viewer.open(&path)
        {
            log::warn!("failed to open {} for compare: {e}", path.display());
        }
        self.compare = Some(Compare {
            viewer,
            layout: Some(layout),
            vertical: false,
            split: 0.5,
            active: 0,
            dragging_split: false,
            painted: None,
        });
        self.forward_loader(1, cx);
    }

    pub fn flip_compare(&mut self) {
        if let Some(c) = self.compare.as_mut()
            && c.layout.is_some()
        {
            c.vertical = !c.vertical;
        }
    }

    /// Aligns the other pane to the active one.
    pub fn align_compare(&mut self, align: Align) {
        let Some(c) = self.compare.as_mut().filter(|c| c.layout.is_some()) else {
            return;
        };
        let (reference, target) = if c.active == 0 {
            (&self.viewer, &mut c.viewer)
        } else {
            (&c.viewer, &mut self.viewer)
        };
        if let Some(transform) = compare::aligned(pane_view(reference), pane_view(target), align) {
            target.view.transform = transform;
        }
    }

    pub fn render_compare_area(
        &mut self,
        layout: Layout,
        p: Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let (first, first_element) = image_of(&mut self.viewer);
        track_painted(&mut self.painted, &first, window, cx);
        let Some(c) = self.compare.as_mut() else {
            return Vec::new();
        };
        let (second, second_element) = image_of(&mut c.viewer);
        track_painted(&mut c.painted, &second, window, cx);

        let (vertical, split, active) = (c.vertical, c.split, c.active);
        let area = self.image_area;
        let rects = self.pane_rects();
        let labels = [pane_label(&self.viewer), pane_label(self.pane(1))];
        let label = |pane: usize, text: String| {
            let mut backdrop = ui::color(p.background);
            backdrop.a = 0.72;
            div()
                .px_2()
                .py_0p5()
                .rounded(ui::rem(4.))
                .bg(backdrop)
                .text_size(ui::rem(12.))
                .text_color(ui::color(if pane == active { p.text_accent } else { p.text_muted }))
                .child(text)
        };

        match layout {
            Layout::Tiles => {
                let [first_label, second_label] = labels;
                let tile = |pane: usize, rect: Rect, element: ImageElement, text: String| {
                    placed(rect)
                        .overflow_hidden()
                        .bg(ui::color(p.background))
                        .child(element)
                        .child(div().absolute().top_2().left_2().child(label(pane, text)))
                        .when(pane == active, |s| {
                            s.child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .border_1()
                                    .border_color(ui::color(p.text_accent)),
                            )
                        })
                        .into_any_element()
                };
                vec![
                    tile(0, rects[0], first_element, first_label),
                    tile(1, rects[1], second_element, second_label),
                ]
            }
            Layout::Slider => {
                let cut = compare::split_px(split, vertical, area);
                let (clip, inner, line, grab) = if vertical {
                    (
                        Rect { x: 0.0, y: cut, w: area.0, h: (area.1 - cut).max(0.0) },
                        Rect { x: 0.0, y: -cut, w: area.0, h: area.1 },
                        Rect { x: 0.0, y: cut - 1.0, w: area.0, h: 2.0 },
                        Rect {
                            x: 0.0,
                            y: cut - compare::SPLIT_GRAB,
                            w: area.0,
                            h: compare::SPLIT_GRAB * 2.0,
                        },
                    )
                } else {
                    (
                        Rect { x: cut, y: 0.0, w: (area.0 - cut).max(0.0), h: area.1 },
                        Rect { x: -cut, y: 0.0, w: area.0, h: area.1 },
                        Rect { x: cut - 1.0, y: 0.0, w: 2.0, h: area.1 },
                        Rect {
                            x: cut - compare::SPLIT_GRAB,
                            y: 0.0,
                            w: compare::SPLIT_GRAB * 2.0,
                            h: area.1,
                        },
                    )
                };
                let [first_label, second_label] = labels;
                let second_corner = if vertical {
                    div().absolute().bottom_2().left_2()
                } else {
                    div().absolute().top_2().right_2()
                };
                vec![
                    placed(rects[0]).child(first_element).into_any_element(),
                    placed(clip)
                        .overflow_hidden()
                        .child(placed(inner).child(second_element))
                        .into_any_element(),
                    placed(line).bg(ui::color(p.text_accent)).into_any_element(),
                    placed(grab)
                        .map(
                            |s| {
                                if vertical { s.cursor_row_resize() } else { s.cursor_col_resize() }
                            },
                        )
                        .into_any_element(),
                    div()
                        .absolute()
                        .top_2()
                        .left_2()
                        .child(label(0, first_label))
                        .into_any_element(),
                    second_corner.child(label(1, second_label)).into_any_element(),
                ]
            }
        }
    }
}
