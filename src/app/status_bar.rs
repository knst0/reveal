use gpui::prelude::FluentBuilder;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, px,
};
use reveal::render::FitMode;
use reveal::ui::{self, MenuItem, Palette};

use super::RevealApp;

impl RevealApp {
    pub fn render_status_bar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let zoom_pct = format!("{:.0}%", self.viewer.transform.zoom * 100.0);
        let danger = self.confirm_delete;

        ui::status_bar(p)
            .child(
                div().when(danger, |s| s.text_color(ui::color(p.danger))).child(self.left_status()),
            )
            .children(
                self.position_label()
                    .map(|label| div().text_color(ui::color(p.text_muted)).child(label)),
            )
            .child(div().flex_grow(1.))
            .children(
                self.dimensions_label()
                    .map(|label| div().text_color(ui::color(p.text_muted)).child(label)),
            )
            .child(
                div()
                    .id("zoom")
                    .relative()
                    .child(
                        ui::tool_button("zoom-btn", p, self.zoom_menu_open)
                            .child(zoom_pct)
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.zoom_menu_open = !this.zoom_menu_open;
                                cx.notify();
                            })),
                    )
                    .children(self.zoom_menu_open.then(|| {
                        div().absolute().bottom(px(28.)).right_0().child(
                            ui::menu_surface(p)
                                .occlude()
                                .min_w(px(160.))
                                .child(zoom_menu_item(cx, "Fit to Window", FitMode::Fit, p))
                                .child(zoom_menu_item(cx, "Fit Best", FitMode::FitBest, p))
                                .child(zoom_menu_item(cx, "Original Size", FitMode::Original, p)),
                        )
                    })),
            )
    }
}

fn zoom_menu_item(
    cx: &mut Context<RevealApp>,
    label: &'static str,
    fit: FitMode,
    p: Palette,
) -> impl IntoElement {
    MenuItem::new(label, label).render(p).on_click(cx.listener(move |this, _e, _w, cx| {
        this.set_fit_and_close_menu(fit);
        cx.notify();
    }))
}
