use reveal::input::{Action, Binding};

pub const APP_NAME: &str = "Reveal";

pub fn title_for(path: Option<&std::path::Path>) -> String {
    match path {
        Some(path) => {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            format!("{name} \u{2014} {APP_NAME}")
        }
        None => APP_NAME.to_owned(),
    }
}

pub fn format_binding(binding: &Binding) -> String {
    let mut s = String::new();
    if binding.modifiers.cmd_ctrl {
        s.push_str("Ctrl+");
    }
    if binding.modifiers.alt {
        s.push_str("Alt+");
    }
    if binding.modifiers.shift {
        s.push_str("Shift+");
    }
    s.push_str(&pretty_key(&binding.key));
    s
}

pub fn pretty_key(key: &str) -> String {
    match key {
        "left" => "\u{2190}".to_owned(),
        "right" => "\u{2192}".to_owned(),
        "up" => "\u{2191}".to_owned(),
        "down" => "\u{2193}".to_owned(),
        "return" => "Enter".to_owned(),
        "escape" => "Esc".to_owned(),
        "pagedown" => "PgDn".to_owned(),
        "pageup" => "PgUp".to_owned(),
        "delete" => "Del".to_owned(),
        "plus" => "+".to_owned(),
        "minus" => "-".to_owned(),
        "space" => "Space".to_owned(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

pub fn action_label(action: Action) -> &'static str {
    match action {
        Action::ImgNext => "Next Image",
        Action::ImgPrev => "Previous Image",
        Action::ImgOrig => "Original Size",
        Action::ImgFit => "Fit to Window",
        Action::ImgFitBest => "Fit Best",
        Action::ImgDel => "Move to Trash",
        Action::ImgCopy => "Copy Image",
        Action::ImgPaste => "Paste Image",
        Action::PanUp => "Pan Up",
        Action::PanDown => "Pan Down",
        Action::PanLeft => "Pan Left",
        Action::PanRight => "Pan Right",
        Action::ZoomIn => "Zoom In",
        Action::ZoomOut => "Zoom Out",
        Action::PlayAnim => "Play / Pause Animation",
        Action::PlayPresent => "Start Slideshow",
        Action::PlayPresentRandom => "Start Random Slideshow",
        Action::ToggleFullscreen => "Toggle Fullscreen",
        Action::ToggleAntialias => "Toggle Smoothing",
        Action::ToggleTheme => "Toggle Theme",
        Action::ToggleBottomBar => "Toggle Status Bar",
        Action::Settings => "Settings",
        Action::Escape => "Cancel / Close",
    }
}

#[cfg(test)]
mod tests {
    use super::{action_label, format_binding, pretty_key, title_for};
    use reveal::input::{Action, Binding};
    use std::path::Path;

    #[test]
    fn title_uses_filename_with_app_suffix() {
        assert_eq!(title_for(Some(Path::new("/a/b/photo.jpg"))), "photo.jpg \u{2014} Reveal");
    }

    #[test]
    fn title_falls_back_to_app_name_when_empty() {
        assert_eq!(title_for(None), "Reveal");
    }

    #[test]
    fn bindings_render_with_modifier_prefixes() {
        let binding = Binding::parse("CmdCtrl+c").unwrap();
        assert_eq!(format_binding(&binding), "Ctrl+C");
    }

    #[test]
    fn arrow_keys_render_as_glyphs() {
        assert_eq!(pretty_key("left"), "\u{2190}");
        assert_eq!(pretty_key("f11"), "F11");
    }

    #[test]
    fn every_action_has_a_menu_label() {
        for action in reveal::input::ALL_ACTIONS {
            assert!(!action_label(*action).is_empty());
        }
        assert_eq!(action_label(Action::ImgDel), "Move to Trash");
    }
}
