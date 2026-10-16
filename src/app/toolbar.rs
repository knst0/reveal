use gpui::{Context, IntoElement, ParentElement, StatefulInteractiveElement, Window};
use reveal::icons::{Icon, icon};
use reveal::input::Action;
use reveal::ui::{self, Palette};

use super::RevealApp;
use super::titlebar;

impl RevealApp {
    pub fn render_toolbar(
        &self,
        p: Palette,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        ui::toolbar(p)
            .children(titlebar::traffic_light_reserve())
            .child(self.toolbar_button("tb-prev", Icon::ArrowLeft, Action::ImgPrev, false, p, cx))
            .child(self.toolbar_button("tb-next", Icon::ArrowRight, Action::ImgNext, false, p, cx))
            .child(titlebar::drag_region())
            .child(self.toolbar_button(
                "tb-settings",
                Icon::Settings,
                Action::Settings,
                self.settings_window.is_some(),
                p,
                cx,
            ))
            .children(titlebar::window_controls(p, window))
    }

    pub fn toolbar_button(
        &self,
        id: &'static str,
        kind: Icon,
        action: Action,
        active: bool,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tint = ui::color(if active { p.text_accent } else { p.text });
        ui::tool_button(id, p, active).child(icon(kind, tint)).on_click(cx.listener(
            move |this, _e, window, cx| {
                this.run(action, window, cx);
            },
        ))
    }
}
