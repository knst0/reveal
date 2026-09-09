fn main() {
    emit_build_info();
    emit_dependencies();

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=resource/reveal.ico");
        let icon = std::path::Path::new("resource/reveal.ico");
        let mut res = winres::WindowsResource::new();
        if icon.exists() {
            res.set_icon("resource/reveal.ico");
        }
        res.set("ProductName", "Reveal");
        res.set("FileDescription", "Reveal");
        res.set("InternalName", "Reveal");
        res.set("OriginalFilename", "reveal.exe");
        if let Err(e) = res.compile() {
            println!("cargo:warning=winres failed: {e}");
        }
    }
}

fn emit_build_info() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();
    println!("cargo:rustc-env=REVEAL_TARGET={target}");
    println!("cargo:rustc-env=REVEAL_PROFILE={profile}");
    println!("cargo:rustc-env=REVEAL_RUSTC={}", rustc_version());
    println!("cargo:rustc-env=REVEAL_COMMIT={}", git_commit());
}

fn rustc_version() -> String {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

fn git_commit() -> String {
    println!("cargo:rerun-if-changed=.git/HEAD");
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

fn emit_dependencies() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let manifest_dir = std::path::Path::new(&manifest_dir);

    let manifest: toml::Value = std::fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("read Cargo.toml")
        .parse()
        .expect("parse Cargo.toml");
    let lock: toml::Value = std::fs::read_to_string(manifest_dir.join("Cargo.lock"))
        .expect("read Cargo.lock")
        .parse()
        .expect("parse Cargo.lock");

    let mut entries: Vec<Dep> = Vec::new();
    for (key, spec) in direct_dependencies(&manifest) {
        let crate_name =
            spec.get("package").and_then(toml::Value::as_str).unwrap_or(&key).to_owned();
        if entries.iter().any(|dep| dep.crate_name == crate_name) {
            continue;
        }
        let feature = spec
            .get("optional")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
            .then(|| enabling_feature(&manifest, &key))
            .flatten();
        let requirement = spec.get("version").and_then(toml::Value::as_str).unwrap_or_default();
        let Some(version) = locked_version(&lock, &crate_name, requirement) else {
            println!("cargo:warning=no lockfile entry for {crate_name}");
            continue;
        };
        let metadata = crate_metadata(&crate_name, &version);
        entries.push(Dep {
            crate_name,
            version,
            license: metadata.license,
            repository: metadata.repository,
            feature,
        });
    }
    entries.sort_by(|a, b| a.crate_name.cmp(&b.crate_name));

    let mut generated = String::from("pub const DEPENDENCIES: &[Dependency] = &[\n");
    for dep in &entries {
        if let Some(feature) = &dep.feature {
            generated.push_str(&format!("    #[cfg(feature = {})]\n", escape(feature)));
        }
        generated.push_str("    Dependency {\n");
        generated.push_str(&format!("        name: {},\n", escape(&dep.crate_name)));
        generated
            .push_str(&format!("        version: {},\n", escape(&minor_version(&dep.version))));
        generated.push_str(&format!("        license: {},\n", escape(&dep.license)));
        generated.push_str(&format!("        repository: {},\n", escape(&dep.repository)));
        generated.push_str("    },\n");
    }
    generated.push_str("];\n");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    std::fs::write(std::path::Path::new(&out_dir).join("dependencies.rs"), generated)
        .expect("write dependencies.rs");
}

struct Dep {
    crate_name: String,
    version: String,
    license: String,
    repository: String,
    feature: Option<String>,
}

struct Metadata {
    license: String,
    repository: String,
}

fn direct_dependencies(manifest: &toml::Value) -> Vec<(String, toml::Value)> {
    let mut tables = Vec::new();
    if let Some(table) = manifest.get("dependencies") {
        tables.push(table);
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            if let Some(table) = target.get("dependencies") {
                tables.push(table);
            }
        }
    }

    let mut deps = Vec::new();
    for table in tables {
        let Some(table) = table.as_table() else { continue };
        for (name, spec) in table {
            let spec = match spec {
                toml::Value::String(version) => {
                    let mut table = toml::map::Map::new();
                    table.insert("version".to_owned(), toml::Value::String(version.clone()));
                    toml::Value::Table(table)
                }
                other => other.clone(),
            };
            deps.push((name.clone(), spec));
        }
    }
    deps
}

fn enabling_feature(manifest: &toml::Value, dep_key: &str) -> Option<String> {
    let features = manifest.get("features")?.as_table()?;
    let target = format!("dep:{dep_key}");
    features
        .iter()
        .find(|(name, enables)| {
            name.as_str() != "default"
                && enables.as_array().is_some_and(|enables| {
                    enables.iter().filter_map(toml::Value::as_str).any(|entry| entry == target)
                })
        })
        .map(|(name, _)| name.clone())
}

fn locked_version(lock: &toml::Value, crate_name: &str, requirement: &str) -> Option<String> {
    let versions: Vec<&str> = lock
        .get("package")?
        .as_array()?
        .iter()
        .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some(crate_name))
        .filter_map(|package| package.get("version").and_then(toml::Value::as_str))
        .collect();

    versions
        .iter()
        .find(|version| satisfies(version, requirement))
        .or_else(|| versions.first())
        .map(|version| (*version).to_owned())
}

fn satisfies(version: &str, requirement: &str) -> bool {
    let requirement = requirement.trim_start_matches(['^', '=', ' ']);
    if requirement.is_empty() {
        return true;
    }
    fn significant(value: &str) -> Vec<&str> {
        value
            .split(['+', '-'])
            .next()
            .unwrap_or(value)
            .split('.')
            .skip_while(|part| *part == "0")
            .collect()
    }
    let required = significant(requirement);
    let actual = significant(version);
    match (required.first(), actual.first()) {
        (Some(required), Some(actual)) => required == actual,
        _ => version.starts_with(requirement),
    }
}

fn minor_version(version: &str) -> String {
    let version = version.split(['+', '-']).next().unwrap_or(version);
    let mut parts = version.split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => format!("{major}.{minor}"),
        _ => version.to_owned(),
    }
}

fn crate_metadata(crate_name: &str, version: &str) -> Metadata {
    let fallback = Metadata { license: String::new(), repository: String::new() };
    let Some(manifest) = registry_manifest(crate_name, version) else {
        println!("cargo:warning=no registry manifest for {crate_name} {version}");
        return fallback;
    };
    let Ok(manifest) = manifest.parse::<toml::Value>() else {
        return fallback;
    };
    let Some(package) = manifest.get("package") else {
        return fallback;
    };
    let field =
        |key: &str| package.get(key).and_then(toml::Value::as_str).unwrap_or_default().to_owned();
    Metadata { license: field("license"), repository: field("repository") }
}

fn registry_manifest(crate_name: &str, version: &str) -> Option<String> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| std::path::Path::new(&home).join(".cargo")))
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|home| std::path::Path::new(&home).join(".cargo"))
        })?;

    let registries = std::fs::read_dir(cargo_home.join("registry").join("src")).ok()?;
    for registry in registries.flatten() {
        let path = registry.path().join(format!("{crate_name}-{version}")).join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Some(text);
        }
    }
    None
}

fn escape(value: &str) -> String {
    format!("{value:?}")
}
