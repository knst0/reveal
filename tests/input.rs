use std::collections::BTreeMap;

use reveal::config::CommandDefinition;
use reveal::input::{
    Action, Bindings, Modifiers, TemplateContext, expand_template, resolve_command,
};

fn plain() -> Modifiers {
    Modifiers::default()
}

#[test]
fn default_bindings_match_upstream() {
    let b = Bindings::default();
    for (key, expected) in [
        ("d", Action::ImgNext),
        ("right", Action::ImgNext),
        ("pagedown", Action::ImgNext),
        ("a", Action::ImgPrev),
        ("left", Action::ImgPrev),
        ("pageup", Action::ImgPrev),
        ("q", Action::ImgOrig),
        ("1", Action::ImgOrig),
        ("f", Action::ImgFit),
        ("e", Action::ImgFitBest),
        ("f11", Action::ToggleFullscreen),
        ("enter", Action::ToggleFullscreen),
        ("+", Action::ZoomIn),
        ("=", Action::ZoomIn),
        ("-", Action::ZoomOut),
        ("p", Action::PlayPresent),
    ] {
        assert_eq!(b.action_for(key, plain()), Some(expected), "binding for {key}");
    }
}

#[test]
fn legacy_key_names_from_saved_configs_are_migrated() {
    let mut b = Bindings::default();
    let mut overrides = BTreeMap::new();
    overrides.insert("toggle_fullscreen".to_owned(), vec!["return".to_owned()]);
    overrides.insert("zoom_in".to_owned(), vec!["plus".to_owned()]);
    overrides.insert("zoom_out".to_owned(), vec!["minus".to_owned()]);
    b.apply_overrides(&overrides);

    assert_eq!(b.action_for("enter", plain()), Some(Action::ToggleFullscreen));
    assert_eq!(b.action_for("+", plain()), Some(Action::ZoomIn));
    assert_eq!(b.action_for("-", plain()), Some(Action::ZoomOut));
}

#[test]
fn cmdctrl_and_alt_modifiers_are_parsed() {
    let b = Bindings::default();

    let ctrl = Modifiers { cmd_ctrl: true, ..Default::default() };
    assert_eq!(b.action_for("c", ctrl), Some(Action::ImgCopy));
    assert_eq!(b.action_for("c", plain()), None, "plain C is not copy");

    let alt = Modifiers { alt: true, ..Default::default() };
    assert_eq!(b.action_for("p", alt), Some(Action::PlayPresentRandom));
    assert_eq!(b.action_for("p", plain()), Some(Action::PlayPresent));
}

#[test]
fn action_names_round_trip() {
    for action in reveal::input::ALL_ACTIONS {
        assert_eq!(Action::from_name(action.name()), Some(*action));
    }
}

#[test]
fn user_rebinding_replaces_the_default_keys() {
    let mut b = Bindings::default();
    let mut overrides = BTreeMap::new();
    overrides.insert("img_next".to_owned(), vec!["n".to_owned()]);
    b.apply_overrides(&overrides);

    assert_eq!(b.action_for("n", plain()), Some(Action::ImgNext));
    assert_eq!(b.action_for("d", plain()), None, "old default is dropped");
    assert_eq!(b.action_for("a", plain()), Some(Action::ImgPrev), "others intact");
}

#[test]
fn unknown_action_in_config_is_ignored() {
    let mut b = Bindings::default();
    let mut overrides = BTreeMap::new();
    overrides.insert("not_a_real_action".to_owned(), vec!["z".to_owned()]);
    b.apply_overrides(&overrides);
    assert_eq!(b.action_for("z", plain()), None);
    assert_eq!(b.action_for("d", plain()), Some(Action::ImgNext));
}

#[test]
fn command_templates_expand_paths_and_envs() {
    let ctx = TemplateContext {
        file_path: "/photos/cat.png",
        folder_path: "/photos",
        file_name: "cat.png",
    };
    assert_eq!(expand_template("open ${file}", &ctx), "open /photos/cat.png");

    let mut envs = BTreeMap::new();
    envs.insert("DIR".to_owned(), "${folder}".to_owned());

    let def = CommandDefinition {
        input: vec!["ctrl+e".to_owned()],
        program: "editor".to_owned(),
        args: vec!["--open".to_owned(), "${file}".to_owned(), "${name}".to_owned()],
        envs,
    };

    let resolved = resolve_command(&def, &ctx);
    assert_eq!(resolved.program, "editor");
    assert_eq!(resolved.args, vec!["--open", "/photos/cat.png", "cat.png"]);
    assert_eq!(resolved.envs.get("DIR").unwrap(), "/photos");
}

#[test]
fn bindings_round_trip_through_config_strings() {
    for (_, keys) in reveal::input::DEFAULT_BINDINGS {
        for key in *keys {
            let binding = reveal::input::Binding::parse(key).unwrap();
            let text = binding.to_config_string();
            assert_eq!(
                reveal::input::Binding::parse(&text),
                Some(binding.clone()),
                "{key} serialised to {text} did not round-trip"
            );
        }
    }
}

#[test]
fn exported_overrides_reproduce_the_same_bindings() {
    let mut edited = reveal::input::Bindings::default();
    edited.rebind(reveal::input::Action::ImgNext, reveal::input::Binding::parse("ctrl+k").unwrap());

    let mut restored = reveal::input::Bindings::default();
    restored.apply_overrides(&edited.to_overrides());

    for action in reveal::input::ALL_ACTIONS {
        assert_eq!(
            restored.keys_for(*action),
            edited.keys_for(*action),
            "{} did not survive the round trip",
            action.name()
        );
    }
}
