use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    FileOpen,
    FolderOpen,
    ImgNext,
    ImgPrev,
    ImgOrig,
    ImgFit,
    ImgFitBest,
    ImgDel,
    ImgCopy,
    ImgPaste,
    PanUp,
    PanDown,
    PanLeft,
    PanRight,
    ZoomIn,
    ZoomOut,
    PlayAnim,
    PlayPresent,
    PlayPresentRandom,
    ToggleFullscreen,
    ToggleAntialias,
    ToggleTheme,
    ToggleBottomBar,
    Settings,
    Escape,
}

impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Action::FileOpen => "file_open",
            Action::FolderOpen => "folder_open",
            Action::ImgNext => "img_next",
            Action::ImgPrev => "img_prev",
            Action::ImgOrig => "img_orig",
            Action::ImgFit => "img_fit",
            Action::ImgFitBest => "img_fit_best",
            Action::ImgDel => "img_del",
            Action::ImgCopy => "img_copy",
            Action::ImgPaste => "img_paste",
            Action::PanUp => "pan_up",
            Action::PanDown => "pan_down",
            Action::PanLeft => "pan_left",
            Action::PanRight => "pan_right",
            Action::ZoomIn => "zoom_in",
            Action::ZoomOut => "zoom_out",
            Action::PlayAnim => "play_anim",
            Action::PlayPresent => "play_present",
            Action::PlayPresentRandom => "play_present_random",
            Action::ToggleFullscreen => "toggle_fullscreen",
            Action::ToggleAntialias => "toggle_antialias",
            Action::ToggleTheme => "toggle_theme",
            Action::ToggleBottomBar => "toggle_bottom_bar",
            Action::Settings => "settings",
            Action::Escape => "escape",
        }
    }

    pub fn from_name(name: &str) -> Option<Action> {
        ALL_ACTIONS.iter().copied().find(|a| a.name() == name)
    }
}

pub const ALL_ACTIONS: &[Action] = &[
    Action::FileOpen,
    Action::FolderOpen,
    Action::ImgNext,
    Action::ImgPrev,
    Action::ImgOrig,
    Action::ImgFit,
    Action::ImgFitBest,
    Action::ImgDel,
    Action::ImgCopy,
    Action::ImgPaste,
    Action::PanUp,
    Action::PanDown,
    Action::PanLeft,
    Action::PanRight,
    Action::ZoomIn,
    Action::ZoomOut,
    Action::PlayAnim,
    Action::PlayPresent,
    Action::PlayPresentRandom,
    Action::ToggleFullscreen,
    Action::ToggleAntialias,
    Action::ToggleTheme,
    Action::ToggleBottomBar,
    Action::Settings,
    Action::Escape,
];

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
    pub fn parse(text: &str) -> Option<Binding> {
        let mut modifiers = Modifiers::default();
        let mut key = None;

        for part in text.split('+') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match part.to_ascii_lowercase().as_str() {
                "alt" => modifiers.alt = true,
                "cmdctrl" | "ctrl" | "cmd" => modifiers.cmd_ctrl = true,
                "shift" => modifiers.shift = true,
                other => key = Some(other.to_owned()),
            }
        }

        key.map(|key| Binding { key, modifiers })
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
        self.map.get(&Binding { key: key.to_ascii_lowercase(), modifiers }).copied()
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
    (Action::ZoomIn, &["plus", "="]),
    (Action::ZoomOut, &["minus", "-"]),
    (Action::PlayAnim, &["alt+a", "space"]),
    (Action::PlayPresent, &["p"]),
    (Action::PlayPresentRandom, &["alt+p"]),
    (Action::ToggleFullscreen, &["f11", "return"]),
    (Action::ToggleAntialias, &["s"]),
    (Action::ToggleTheme, &["t"]),
    (Action::ToggleBottomBar, &["b"]),
    (Action::Settings, &["h", "f1"]),
    (Action::Escape, &["escape"]),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCommand {
    pub program: String,
    pub args: Vec<String>,
    pub envs: BTreeMap<String, String>,
}

pub struct TemplateContext<'a> {
    pub file_path: &'a str,
    pub folder_path: &'a str,
    pub file_name: &'a str,
}

pub fn expand_template(template: &str, ctx: &TemplateContext<'_>) -> String {
    template
        .replace("${file}", ctx.file_path)
        .replace("${folder}", ctx.folder_path)
        .replace("${name}", ctx.file_name)
}

pub fn resolve_command(
    definition: &crate::config::CommandDefinition,
    ctx: &TemplateContext<'_>,
) -> ResolvedCommand {
    ResolvedCommand {
        program: expand_template(&definition.program, ctx),
        args: definition.args.iter().map(|a| expand_template(a, ctx)).collect(),
        envs: definition.envs.iter().map(|(k, v)| (k.clone(), expand_template(v, ctx))).collect(),
    }
}
