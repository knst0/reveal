use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::{AnyElement, Context, Div, IntoElement, ParentElement, Styled, Window, div, px};
use reveal::compare::{self, Align, Layout, Rect};
use reveal::ui::{self, Palette};
use reveal::viewer::Viewer;

use super::RevealApp;

fn pane_label(viewer: &Viewer) -> String {
    if let Some(status) = viewer.status() {
        return status.to_owned();
    }
    match viewer.display_name() {
        Some(name) => {
            let (index, len) = viewer.position().unwrap_or((1, 1));
            format!("{name}  {index}/{len}")
        }
        None => "Drop or open an image here".to_owned(),
    }
}

fn placed(rect: Rect) -> Div {
    div().absolute().left(px(rect.x)).top(px(rect.y)).w(px(rect.w)).h(px(rect.h))
}

impl RevealApp {
    pub fn layout(&self) -> Option<Layout> {
        self.panes.layout()
    }

    /// Viewers on screen: the main one, plus the compared one in compare mode.
    pub fn visible_panes(&self) -> usize {
        self.panes.visible_count()
    }

    pub fn pane(&self, index: usize) -> &Viewer {
        self.panes.viewer(index)
    }

    pub fn pane_mut(&mut self, index: usize) -> &mut Viewer {
        self.panes.viewer_mut(index)
    }

    /// Every existing viewer, shown or not; background results still go to it.
    pub fn existing_panes(&self) -> usize {
        self.panes.existing_count()
    }

    pub fn active_pane(&self) -> usize {
        self.panes.active_index()
    }

    pub fn active_viewer(&self) -> &Viewer {
        self.panes.active()
    }

    pub fn active_viewer_mut(&mut self) -> &mut Viewer {
        self.panes.active_mut()
    }

    pub fn targets(&self, only: Option<usize>) -> Vec<usize> {
        self.panes.targets(only)
    }

    pub fn pane_rects(&self) -> [Rect; 2] {
        self.panes.pane_rects()
    }

    pub fn pane_at(&self, point: (f32, f32)) -> usize {
        self.panes.pane_at(point)
    }

    /// `point` (image-area coordinates) relative to the pane it falls in.
    pub fn pane_local(&self, pane: usize, point: (f32, f32)) -> (f32, f32) {
        self.panes.pane_local(pane, point)
    }

    /// Makes the pane under `point` the active one; true when it changed.
    pub fn hover_pane(&mut self, point: (f32, f32)) -> bool {
        self.panes.hover_pane(point)
    }

    pub fn near_split(&self, point: (f32, f32)) -> bool {
        self.panes.near_split(point)
    }

    pub fn move_split(&mut self, point: (f32, f32)) {
        self.panes.move_split(point);
    }

    /// Turns `layout` on, or off when it is already shown.
    pub fn toggle_compare(&mut self, layout: Layout) {
        if self.panes.has_second() {
            self.panes.toggle_compare(layout);
            return;
        }

        let mut viewer = Viewer::new();
        let main = self.panes.main();
        viewer.inherit_view(main);
        // Start on the same image so a second folder can be dropped next to it.
        if let Some(path) = main.directory_current().map(PathBuf::from) {
            viewer.open(&path);
        }
        self.panes.install_second(viewer, layout);
        self.forward_loader(1);
    }

    pub fn flip_compare(&mut self) {
        if self.panes.is_comparing() {
            self.panes.flip();
        }
    }

    /// Aligns the other pane to the active one.
    pub fn align_compare(&mut self, align: Align) {
        if self.panes.is_comparing() {
            self.panes.align_active_to_other(align);
        }
    }

    pub fn render_compare_area(
        &mut self,
        p: Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let Some(compare) = self.panes.compare() else {
            return Vec::new();
        };
        let Some(layout) = compare.layout else {
            return Vec::new();
        };
        let (vertical, split) = (compare.vertical, compare.split);
        let first_element = self.panes.image_element(0, window, cx);
        let second_element = self.panes.image_element(1, window, cx);
        let active = self.panes.active_index();
        let area = self.panes.area();
        let rects = self.pane_rects();
        let labels = [pane_label(self.panes.viewer(0)), pane_label(self.panes.viewer(1))];
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
                let tile = |pane: usize,
                            rect: Rect,
                            element: reveal::render::ImageElement,
                            text: String| {
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
