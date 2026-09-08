use gpui::{
    Decorations, Div, InteractiveElement, MouseButton, MouseDownEvent, ParentElement, ResizeEdge,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use reveal::icons::{Icon, icon};
use reveal::ui::{self, Palette};

pub const TRAFFIC_LIGHT_INSET: f32 = 20.0;
const TRAFFIC_LIGHT_RESERVE: f32 = 78.0;
const RESIZE_EDGE: f32 = 6.0;

pub fn traffic_light_reserve() -> Option<Div> {
    cfg!(target_os = "macos").then(|| div().flex_shrink_0().w(ui::rem(TRAFFIC_LIGHT_RESERVE)))
}

pub fn header(p: Palette, title: &'static str, window: &Window) -> Div {
    ui::toolbar(p)
        .children(traffic_light_reserve())
        .child(drag_region().flex().items_center().child(title))
        .children(window_controls(p, window))
}

pub fn titlebar_options(title: &str) -> gpui::TitlebarOptions {
    gpui::TitlebarOptions {
        title: Some(title.to_owned().into()),
        appears_transparent: true,
        traffic_light_position: Some(gpui::point(
            px(TRAFFIC_LIGHT_INSET),
            px((ui::TOOLBAR_HEIGHT - 16.0) / 2.0),
        )),
    }
}

pub fn drag_region() -> Div {
    div()
        .flex_grow(1.)
        .h_full()
        .on_mouse_down(MouseButton::Left, |event: &MouseDownEvent, window, _cx| {
            if event.click_count == 2 {
                window.zoom_window();
            } else {
                window.start_window_move();
            }
        })
        .on_mouse_down(MouseButton::Right, |event: &MouseDownEvent, window, _cx| {
            window.show_window_menu(event.position);
        })
}

pub fn window_controls(p: Palette, window: &Window) -> Option<Div> {
    if cfg!(target_os = "macos") {
        return None;
    }
    let controls = window.window_controls();
    let maximized = window.is_maximized();

    Some(
        div()
            .ml_2()
            .flex()
            .items_center()
            .gap_px()
            .children(controls.minimize.then(|| {
                control_button("wc-minimize", Icon::Minus, p, false).on_click(|_e, window, _cx| {
                    window.minimize_window();
                })
            }))
            .children(controls.maximize.then(|| {
                control_button(
                    "wc-maximize",
                    if maximized { Icon::Copy } else { Icon::Square },
                    p,
                    false,
                )
                .on_click(|_e, window, _cx| {
                    window.zoom_window();
                })
            }))
            .child(control_button("wc-close", Icon::Close, p, true).on_click(|_e, window, _cx| {
                window.remove_window();
            })),
    )
}

fn control_button(id: &'static str, kind: Icon, p: Palette, danger: bool) -> gpui::Stateful<Div> {
    let tint = ui::color(p.text_muted);
    div()
        .id(id)
        .size(ui::rem(ui::BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui::rem(6.))
        .hover(|s| s.bg(ui::color(if danger { p.danger } else { p.element_hover })))
        .child(icon(kind, tint))
}

pub fn resize_handles(window: &Window) -> Option<Div> {
    let Decorations::Client { .. } = window.window_decorations() else {
        return None;
    };

    Some(div().absolute().inset_0().children(RESIZE_EDGES.iter().map(|(edge, place)| {
        let edge = *edge;
        place(div()).absolute().on_mouse_down(MouseButton::Left, move |_e, window, _cx| {
            window.start_window_resize(edge);
        })
    })))
}

type Placement = fn(Div) -> Div;

const RESIZE_EDGES: [(ResizeEdge, Placement); 8] = [
    (ResizeEdge::Top, |d| d.top_0().left_0().right_0().h(px(RESIZE_EDGE))),
    (ResizeEdge::Bottom, |d| d.bottom_0().left_0().right_0().h(px(RESIZE_EDGE))),
    (ResizeEdge::Left, |d| d.top_0().bottom_0().left_0().w(px(RESIZE_EDGE))),
    (ResizeEdge::Right, |d| d.top_0().bottom_0().right_0().w(px(RESIZE_EDGE))),
    (ResizeEdge::TopLeft, |d| d.top_0().left_0().size(px(RESIZE_EDGE * 2.0))),
    (ResizeEdge::TopRight, |d| d.top_0().right_0().size(px(RESIZE_EDGE * 2.0))),
    (ResizeEdge::BottomLeft, |d| d.bottom_0().left_0().size(px(RESIZE_EDGE * 2.0))),
    (ResizeEdge::BottomRight, |d| d.bottom_0().right_0().size(px(RESIZE_EDGE * 2.0))),
];
