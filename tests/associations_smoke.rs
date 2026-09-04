#![cfg(target_os = "windows")]

#[test]
#[ignore = "writes to the real HKCU registry; run manually with --ignored"]
fn registering_writes_progids_for_every_format() {
    let outcome = reveal::associations::register().expect("registration succeeds");
    assert_eq!(outcome.registered, reveal::formats::FORMATS.len());

    for key in [
        r"HKCU\Software\Classes\Reveal.png\shell\open\command",
        r"HKCU\Software\Classes\.png\OpenWithProgids",
        r"HKCU\Software\Reveal\Capabilities\FileAssociations",
    ] {
        let out = std::process::Command::new("reg")
            .args(["query", key])
            .output()
            .expect("reg query runs");
        assert!(out.status.success(), "missing registry key: {key}");
    }

    let out = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\RegisteredApplications", "/v", "Reveal"])
        .output()
        .expect("reg query runs");
    assert!(out.status.success(), "RegisteredApplications entry missing");
}
