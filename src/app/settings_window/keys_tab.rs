use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, px,
};
use reveal::input::{ALL_ACTIONS, Action, Binding};
use reveal::ui::{self, Palette};

use super::SettingsWindow;
use crate::app::labels::{action_label, format_binding};

impl SettingsWindow {
    pub(super) fn render_keys_tab(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let capturing = self.state.capturing.clone();
        let rows: Vec<(Action, Vec<Binding>)> = ALL_ACTIONS
            .iter()
            .map(|action| {
                let keys: Vec<Binding> =
                    self.state.bindings.keys_for(*action).into_iter().cloned().collect();
                (*action, keys)
            })
            .collect();

        let hint = match capturing.as_ref() {
            Some(target) => {
                format!(
                    "Press a key for \u{201c}{}\u{201d}, or Esc to cancel.",
                    action_label(target.action)
                )
            }
            None => match self.state.displaced {
                Some(other) => {
                    format!("Rebound. \u{201c}{}\u{201d} lost that shortcut.", action_label(other))
                }
                None => self
                    .state
                    .notice
                    .clone()
                    .unwrap_or_else(|| "Click a shortcut to change it.".to_owned()),
            },
        };

        self.settings_body().gap_0().child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .pb_2()
                        .text_color(ui::color(if capturing.is_some() {
                            p.text_accent
                        } else {
                            p.text_muted
                        }))
                        .child(hint),
                )
                .children(rows.into_iter().map(|(action, keys)| {
                    let adding = capturing
                        .as_ref()
                        .is_some_and(|t| t.action == action && t.replacing.is_none());
                    let mut chips: Vec<gpui::AnyElement> = Vec::new();
                    for (index, binding) in keys.iter().enumerate() {
                        let recapturing = capturing
                            .as_ref()
                            .is_some_and(|t| t.replacing.as_ref() == Some(binding));
                        let for_click = binding.clone();
                        let for_remove = binding.clone();
                        chips.push(
                            ui::chip(("bind", action as usize * 16 + index), p, recapturing)
                                .gap_1()
                                .child(if recapturing {
                                    "Press a key\u{2026}".to_owned()
                                } else {
                                    format_binding(binding)
                                })
                                .child(
                                    div()
                                        .id(("unbind", action as usize * 16 + index))
                                        .text_color(ui::color(p.text_muted))
                                        .hover(|s| s.text_color(ui::color(p.danger)))
                                        .child("\u{00d7}")
                                        .occlude()
                                        .on_click(cx.listener(move |this, _e, _w, cx| {
                                            this.state.remove_binding(&for_remove);
                                            this.preview(cx);
                                        })),
                                )
                                .on_click(cx.listener(move |this, _e, _w, cx| {
                                    this.state.begin_recapture(action, for_click.clone());
                                    cx.notify();
                                }))
                                .into_any_element(),
                        );
                    }
                    if keys.is_empty() && !adding {
                        chips.push(
                            div()
                                .h(px(24.))
                                .flex()
                                .items_center()
                                .text_color(ui::color(p.text_muted))
                                .child("Unassigned")
                                .into_any_element(),
                        );
                    }
                    chips.push(
                        ui::chip(("add-bind", action as usize), p, adding)
                            .child(if adding { "Press a key\u{2026}" } else { "+" })
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.state.begin_capture(action);
                                cx.notify();
                            }))
                            .into_any_element(),
                    );

                    div()
                        .py_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_grow(1.).child(action_label(action)))
                        .child(div().flex().items_center().gap_1().flex_wrap().children(chips))
                })),
        )
    }
}
