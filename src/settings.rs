use crate::config::{Channel, Configuration};
use crate::input::{Action, Binding, Bindings, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Keys,
}

impl SettingsTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Keys => "Keyboard",
        }
    }
}

pub const SETTINGS_TABS: &[SettingsTab] = &[SettingsTab::General, SettingsTab::Keys];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleField {
    Dark,
    ShowBottomBar,
    Antialias,
    StartFullscreen,
    UpdateCheck,
    UpdateAutoInstall,
}

impl ToggleField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark theme",
            Self::ShowBottomBar => "Show status bar",
            Self::Antialias => "Smooth scaling",
            Self::StartFullscreen => "Start fullscreen",
            Self::UpdateCheck => "Check for updates",
            Self::UpdateAutoInstall => "Install updates automatically",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Dark => "Use the dark palette for the interface.",
            Self::ShowBottomBar => "Display the status bar along the bottom edge.",
            Self::Antialias => "Filter images when they are scaled.",
            Self::StartFullscreen => "Open new windows in fullscreen.",
            Self::UpdateCheck => "Look for new releases on startup.",
            Self::UpdateAutoInstall => "Download and apply updates without asking.",
        }
    }

    pub fn get(self, config: &Configuration) -> bool {
        match self {
            Self::Dark => config.window.dark,
            Self::ShowBottomBar => config.window.show_bottom_bar,
            Self::Antialias => config.window.antialias,
            Self::StartFullscreen => config.window.start_fullscreen,
            Self::UpdateCheck => config.updates.check,
            Self::UpdateAutoInstall => config.updates.auto_install,
        }
    }

    pub fn set(self, config: &mut Configuration, value: bool) {
        match self {
            Self::Dark => config.window.dark = value,
            Self::ShowBottomBar => config.window.show_bottom_bar = value,
            Self::Antialias => config.window.antialias = value,
            Self::StartFullscreen => config.window.start_fullscreen = value,
            Self::UpdateCheck => config.updates.check = value,
            Self::UpdateAutoInstall => config.updates.auto_install = value,
        }
    }

    pub fn enabled(self, config: &Configuration) -> bool {
        match self {
            Self::UpdateAutoInstall => config.updates.check,
            _ => true,
        }
    }
}

pub const APPEARANCE_FIELDS: &[ToggleField] = &[
    ToggleField::Dark,
    ToggleField::ShowBottomBar,
    ToggleField::Antialias,
    ToggleField::StartFullscreen,
];

pub const UPDATE_FIELDS: &[ToggleField] =
    &[ToggleField::UpdateCheck, ToggleField::UpdateAutoInstall];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureTarget {
    pub action: Action,
    pub replacing: Option<Binding>,
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub tab: SettingsTab,
    pub config: Configuration,
    pub bindings: Bindings,
    pub capturing: Option<CaptureTarget>,
    pub notice: Option<String>,
    pub displaced: Option<Action>,
    dirty: bool,
}

impl SettingsState {
    pub fn new(config: Configuration, bindings: Bindings) -> Self {
        Self {
            tab: SettingsTab::General,
            config,
            bindings,
            capturing: None,
            notice: None,
            displaced: None,
            dirty: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn select_tab(&mut self, tab: SettingsTab) {
        self.tab = tab;
        self.capturing = None;
        self.notice = None;
        self.displaced = None;
    }

    pub fn toggle(&mut self, field: ToggleField) {
        if !field.enabled(&self.config) {
            return;
        }
        let value = !field.get(&self.config);
        field.set(&mut self.config, value);
        self.notice = None;
        if field == ToggleField::UpdateCheck && !value {
            self.config.updates.auto_install = false;
        }
        self.dirty = true;
    }

    pub fn set_channel(&mut self, channel: Channel) {
        if self.config.updates.channel != channel {
            self.config.updates.channel = channel;
            self.dirty = true;
        }
    }

    pub fn begin_capture(&mut self, action: Action) {
        self.capturing = Some(CaptureTarget { action, replacing: None });
        self.notice = None;
        self.displaced = None;
    }

    pub fn begin_recapture(&mut self, action: Action, existing: Binding) {
        self.capturing = Some(CaptureTarget { action, replacing: Some(existing) });
        self.notice = None;
        self.displaced = None;
    }

    pub fn cancel_capture(&mut self) -> bool {
        self.capturing.take().is_some()
    }

    pub fn capture_key(&mut self, key: &str, modifiers: Modifiers) -> bool {
        let Some(target) = self.capturing.clone() else {
            return false;
        };
        if key.eq_ignore_ascii_case("escape") {
            self.capturing = None;
            return true;
        }
        if is_modifier_key(key) {
            return true;
        }
        let binding = Binding { key: key.to_ascii_lowercase(), modifiers };
        if let Some(previous) = target.replacing {
            self.bindings.remove_binding(&previous);
        }
        self.displaced = self.bindings.set_binding(target.action, binding);
        self.capturing = None;
        self.notice = None;
        self.dirty = true;
        true
    }

    pub fn clear_binding(&mut self, action: Action) {
        self.bindings.clear_action(action);
        self.capturing = None;
        self.dirty = true;
    }

    pub fn remove_binding(&mut self, binding: &Binding) {
        self.bindings.remove_binding(binding);
        self.capturing = None;
        self.dirty = true;
    }

    pub fn reset_bindings(&mut self) {
        self.bindings.reset_to_defaults();
        self.capturing = None;
        self.notice = Some("Shortcuts restored to defaults.".to_owned());
        self.dirty = true;
    }

    pub fn register_associations(&mut self) {
        self.notice = Some(association_notice());
    }
}

pub fn association_notice() -> String {
    match crate::associations::register() {
        Ok(outcome) => {
            let mut text = format!("Registered {} formats.", outcome.registered);
            if let Some(extra) = outcome.needs_user_action {
                text.push(' ');
                text.push_str(&extra);
            }
            text
        }
        Err(e) => format!("Could not set defaults: {e}"),
    }
}

impl SettingsState {
    pub fn persist(&mut self) -> std::io::Result<()> {
        self.config.bindings =
            self.bindings.differs_from_defaults().then(|| self.bindings.to_overrides());
        self.config.save()?;
        self.dirty = false;
        Ok(())
    }
}

fn is_modifier_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "shift" | "control" | "ctrl" | "alt" | "cmd" | "platform" | "function"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> SettingsState {
        SettingsState::new(Configuration::default(), Bindings::default())
    }

    #[test]
    fn toggling_a_field_marks_state_dirty() {
        let mut s = state();
        assert!(!s.is_dirty());
        s.toggle(ToggleField::Dark);
        assert!(!s.config.window.dark);
        assert!(s.is_dirty());
    }

    #[test]
    fn disabling_update_check_also_clears_auto_install() {
        let mut s = state();
        s.config.updates.auto_install = true;
        s.toggle(ToggleField::UpdateCheck);
        assert!(!s.config.updates.check);
        assert!(!s.config.updates.auto_install);
    }

    #[test]
    fn auto_install_is_locked_while_checks_are_off() {
        let mut s = state();
        s.config.updates.check = false;
        s.toggle(ToggleField::UpdateAutoInstall);
        assert!(!s.config.updates.auto_install);
    }

    #[test]
    fn capturing_a_key_adds_a_binding_without_dropping_the_others() {
        let mut s = state();
        s.begin_capture(Action::ImgNext);
        assert!(s.capture_key("k", Modifiers::default()));
        assert_eq!(s.bindings.action_for("k", Modifiers::default()), Some(Action::ImgNext));
        assert_eq!(s.bindings.action_for("d", Modifiers::default()), Some(Action::ImgNext));
    }

    #[test]
    fn recapturing_replaces_only_the_selected_binding() {
        let mut s = state();
        s.begin_capture(Action::ImgNext);
        s.capture_key("k", Modifiers::default());
        let existing = Binding { key: "d".to_owned(), modifiers: Modifiers::default() };
        s.begin_recapture(Action::ImgNext, existing);
        s.capture_key("m", Modifiers::default());
        assert_eq!(s.bindings.action_for("d", Modifiers::default()), None);
        assert_eq!(s.bindings.action_for("m", Modifiers::default()), Some(Action::ImgNext));
        assert_eq!(s.bindings.action_for("k", Modifiers::default()), Some(Action::ImgNext));
    }

    #[test]
    fn removing_one_binding_keeps_the_rest() {
        let mut s = state();
        s.begin_capture(Action::ImgNext);
        s.capture_key("k", Modifiers::default());
        s.remove_binding(&Binding { key: "k".to_owned(), modifiers: Modifiers::default() });
        assert_eq!(s.bindings.action_for("k", Modifiers::default()), None);
        assert_eq!(s.bindings.action_for("d", Modifiers::default()), Some(Action::ImgNext));
    }

    #[test]
    fn capture_reports_when_another_action_is_displaced() {
        let mut s = state();
        s.begin_capture(Action::ImgNext);
        s.capture_key("f", Modifiers::default());
        assert_eq!(s.displaced, Some(Action::ImgFit));
        assert_eq!(s.bindings.action_for("f", Modifiers::default()), Some(Action::ImgNext));
    }

    #[test]
    fn escape_cancels_capture_without_rebinding() {
        let mut s = state();
        s.begin_capture(Action::ImgNext);
        s.capture_key("escape", Modifiers::default());
        assert!(s.capturing.is_none());
        assert_eq!(s.bindings.action_for("d", Modifiers::default()), Some(Action::ImgNext));
    }

    #[test]
    fn modifier_only_presses_do_not_end_capture() {
        let mut s = state();
        s.begin_capture(Action::ImgNext);
        s.capture_key("shift", Modifiers { shift: true, ..Default::default() });
        assert_eq!(s.capturing.map(|t| t.action), Some(Action::ImgNext));
    }

    #[test]
    fn overrides_are_only_written_when_bindings_changed() {
        let mut s = state();
        assert!(!s.bindings.differs_from_defaults());
        s.begin_capture(Action::ImgNext);
        s.capture_key("k", Modifiers::default());
        assert!(s.bindings.differs_from_defaults());
    }

    #[test]
    fn resetting_bindings_restores_defaults() {
        let mut s = state();
        s.begin_capture(Action::ImgNext);
        s.capture_key("k", Modifiers::default());
        s.reset_bindings();
        assert!(!s.bindings.differs_from_defaults());
    }
}
