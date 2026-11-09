use std::collections::BTreeMap;

fn canonical_key(key: &str) -> String {
    match key {
        "return" => "enter".to_owned(),
        "plus" => "+".to_owned(),
        "minus" => "-".to_owned(),
        "esc" => "escape".to_owned(),
        other => other.to_owned(),
    }
}

macro_rules! actions {
    ($($variant:ident => $name:literal),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Action {
            $($variant),*
        }

        impl Action {
            pub fn name(self) -> &'static str {
                match self {
                    $(Action::$variant => $name),*
                }
            }

            pub fn from_name(name: &str) -> Option<Action> {
                match name {
                    $($name => Some(Action::$variant),)*
                    _ => None,
                }
            }
        }

        pub const ALL_ACTIONS: &[Action] = &[$(Action::$variant),*];
    };
}

actions! {
    FileOpen => "file_open",
    FolderOpen => "folder_open",
    ImgNext => "img_next",
    ImgPrev => "img_prev",
    ImgOrig => "img_orig",
    ImgFit => "img_fit",
    ImgFitBest => "img_fit_best",
    ImgDel => "img_del",
    ImgCopy => "img_copy",
    ImgPaste => "img_paste",
    PanUp => "pan_up",
    PanDown => "pan_down",
    PanLeft => "pan_left",
    PanRight => "pan_right",
    ZoomIn => "zoom_in",
    ZoomOut => "zoom_out",
    PlayAnim => "play_anim",
    PlayPresent => "play_present",
    PlayPresentRandom => "play_present_random",
    ToggleFullscreen => "toggle_fullscreen",
    ToggleAntialias => "toggle_antialias",
    ToggleTheme => "toggle_theme",
    Settings => "settings",
    CompareTiles => "compare_tiles",
    CompareSlider => "compare_slider",
    CompareFlip => "compare_flip",
    CompareAlign => "compare_align",
    CompareAlignPixels => "compare_align_pixels",
    Escape => "escape",
}

/// Actions that a viewer can perform on its own. Keeping the mapping next to
/// `Action` means adding a pane action cannot half-land in dispatch code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneAction {
    Next,
    Prev,
    Original,
    Fit,
    FitBest,
    PanUp,
    PanDown,
    PanLeft,
    PanRight,
    ZoomIn,
    ZoomOut,
    TogglePlay,
    Present,
    PresentRandom,
}

impl Action {
    pub fn pane_action(self) -> Option<PaneAction> {
        use PaneAction as P;
        Some(match self {
            Action::ImgNext => P::Next,
            Action::ImgPrev => P::Prev,
            Action::ImgOrig => P::Original,
            Action::ImgFit => P::Fit,
            Action::ImgFitBest => P::FitBest,
            Action::PanUp => P::PanUp,
            Action::PanDown => P::PanDown,
            Action::PanLeft => P::PanLeft,
            Action::PanRight => P::PanRight,
            Action::ZoomIn => P::ZoomIn,
            Action::ZoomOut => P::ZoomOut,
            Action::PlayAnim => P::TogglePlay,
            Action::PlayPresent => P::Present,
            Action::PlayPresentRandom => P::PresentRandom,
            Action::FileOpen
            | Action::FolderOpen
            | Action::ImgDel
            | Action::ImgCopy
            | Action::ImgPaste
            | Action::ToggleFullscreen
            | Action::ToggleAntialias
            | Action::ToggleTheme
            | Action::Settings
            | Action::CompareTiles
            | Action::CompareSlider
            | Action::CompareFlip
            | Action::CompareAlign
            | Action::CompareAlignPixels
            | Action::Escape => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Modifiers {
    pub alt: bool,
    pub cmd_ctrl: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Binding {
    pub key: String,
    pub modifiers: Modifiers,
}

impl Binding {
    /// Canonicalises aliases such as `return`/`enter` once, so captured keys
    /// and configured keys agree with key dispatch.
    pub fn new(key: &str, modifiers: Modifiers) -> Binding {
        Binding { key: canonical_key(&key.to_ascii_lowercase()), modifiers }
    }

    pub fn parse(text: &str) -> Option<Binding> {
        let mut modifiers = Modifiers::default();
        let mut key = None;

        let trimmed = text.trim();
        let literal_plus = trimmed.ends_with('+');
        let head = if literal_plus { &trimmed[..trimmed.len() - 1] } else { trimmed };

        for part in head.split('+') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match part.to_ascii_lowercase().as_str() {
                "alt" => modifiers.alt = true,
                "cmdctrl" | "ctrl" | "cmd" => modifiers.cmd_ctrl = true,
                "shift" => modifiers.shift = true,
                other => key = Some(canonical_key(other)),
            }
        }

        if literal_plus {
            key = Some("+".to_owned());
        }

        key.map(|key| Binding::new(&key, modifiers))
    }

    pub fn to_config_string(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.cmd_ctrl {
            parts.push("CmdCtrl");
        }
        if self.modifiers.alt {
            parts.push("alt");
        }
        if self.modifiers.shift {
            parts.push("shift");
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

#[derive(Debug, Clone)]
pub struct Bindings {
    map: BTreeMap<Binding, Action>,
}

impl Default for Bindings {
    fn default() -> Self {
        let mut map = BTreeMap::new();
        for (action, keys) in DEFAULT_BINDINGS {
            for key in *keys {
                if let Some(binding) = Binding::parse(key) {
                    map.insert(binding, *action);
                }
            }
        }
        Self { map }
    }
}

impl Bindings {
    pub fn action_for(&self, key: &str, modifiers: Modifiers) -> Option<Action> {
        self.map.get(&Binding::new(key, modifiers)).copied()
    }

    pub fn keys_for(&self, action: Action) -> Vec<&Binding> {
        self.map.iter().filter(|(_, a)| **a == action).map(|(b, _)| b).collect()
    }

    pub fn set_binding(&mut self, action: Action, binding: Binding) -> Option<Action> {
        let previous = self.map.insert(binding, action);
        previous.filter(|a| *a != action)
    }

    pub fn remove_binding(&mut self, binding: &Binding) {
        self.map.remove(binding);
    }

    pub fn clear_action(&mut self, action: Action) {
        self.map.retain(|_, a| *a != action);
    }

    pub fn rebind(&mut self, action: Action, binding: Binding) -> Option<Action> {
        self.clear_action(action);
        self.set_binding(action, binding)
    }

    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
    }

    pub fn to_overrides(&self) -> BTreeMap<String, Vec<String>> {
        let mut overrides: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for action in ALL_ACTIONS {
            let keys: Vec<String> =
                self.keys_for(*action).into_iter().map(Binding::to_config_string).collect();
            overrides.insert(action.name().to_owned(), keys);
        }
        overrides
    }

    pub fn differs_from_defaults(&self) -> bool {
        self.map != Self::default().map
    }

    pub fn apply_overrides(&mut self, overrides: &BTreeMap<String, Vec<String>>) {
        for (name, keys) in overrides {
            let Some(action) = Action::from_name(name) else {
                log::warn!("unknown action in config: {name}");
                continue;
            };
            self.map.retain(|_, a| *a != action);
            for key in keys {
                match Binding::parse(key) {
                    Some(binding) => {
                        self.map.insert(binding, action);
                    }
                    None => log::warn!("unparsable binding for {name}: {key}"),
                }
            }
        }
    }
}

pub const DEFAULT_BINDINGS: &[(Action, &[&str])] = &[
    (Action::FileOpen, &["CmdCtrl+o"]),
    (Action::FolderOpen, &["CmdCtrl+shift+o"]),
    (Action::ImgNext, &["d", "right", "pagedown"]),
    (Action::ImgPrev, &["a", "left", "pageup"]),
    (Action::ImgOrig, &["q", "1"]),
    (Action::ImgFit, &["f"]),
    (Action::ImgFitBest, &["e"]),
    (Action::ImgDel, &["delete"]),
    (Action::ImgCopy, &["CmdCtrl+c"]),
    (Action::ImgPaste, &["CmdCtrl+v"]),
    (Action::PanUp, &["up"]),
    (Action::PanDown, &["down"]),
    (Action::PanLeft, &["shift+left"]),
    (Action::PanRight, &["shift+right"]),
    (Action::ZoomIn, &["+", "="]),
    (Action::ZoomOut, &["-"]),
    (Action::PlayAnim, &["alt+a", "space"]),
    (Action::PlayPresent, &["p"]),
    (Action::PlayPresentRandom, &["alt+p"]),
    (Action::ToggleFullscreen, &["f11", "enter"]),
    (Action::ToggleAntialias, &["s"]),
    (Action::ToggleTheme, &["t"]),
    (Action::Settings, &["h", "f1"]),
    (Action::CompareTiles, &["b"]),
    (Action::CompareSlider, &["v"]),
    (Action::CompareFlip, &["x"]),
    (Action::CompareAlign, &["g"]),
    (Action::CompareAlignPixels, &["shift+g"]),
    (Action::Escape, &["escape"]),
];
