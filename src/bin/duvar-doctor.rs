use std::{
    path::PathBuf,
    process::{Command, ExitCode},
};
fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("Duvar doctor: {e}");
            ExitCode::from(2)
        }
    }
}
fn run() -> anyhow::Result<bool> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|a| a == "--help") {
        println!(
            "duvar-doctor [--json] [--output PATH] [--extension PATH] [--source PATH] [--network] [--full] [--verify] [--gui]\nVarsayılan salt okunur, çevrimdışı. --verify: biçim, clippy ve yalıtılmış Rust testleri; --gui: özel D-Bus/HOME ile GTK senaryoları. Çıkış: 0 başarılı, 1 sorun, 2 kullanım/çalıştırma hatası."
        );
        return Ok(true);
    }
    let value = |key: &str| {
        args.iter()
            .position(|v| v == key)
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
    };
    let extension = value("--extension")
        .unwrap_or_else(|| glib::user_data_dir().join("gnome-shell/extensions/duvar@mo4tal.local"));
    let mut report = duvar_native::doctor::report(
        &extension,
        args.iter().any(|a| a == "--network"),
        args.iter().any(|a| a == "--full"),
    );
    let needs_source = args.iter().any(|a| a == "--verify" || a == "--gui");
    let source = value("--source").or_else(|| {
        std::env::current_dir().ok().and_then(|p| {
            p.ancestors()
                .find(|p| {
                    std::fs::read_to_string(p.join("Cargo.toml"))
                        .is_ok_and(|s| s.contains(r#"name = "duvar""#))
                })
                .map(PathBuf::from)
        })
    });
    if needs_source && source.is_none() {
        anyhow::bail!(
            "Bu denetim kaynak kod gerektirir. Paylaşım paketindeki kaynak arşivini açıp --source KLASÖR belirt."
        );
    }
    let root = source.unwrap_or_default();
    if args.iter().any(|a| a == "--verify") {
        let temp = tempfile::Builder::new()
            .prefix("duvar-doctor-tests-")
            .tempdir()?;
        let mut results = Vec::new();
        for command in [
            vec!["fmt", "--all", "--check"],
            vec![
                "clippy",
                "--locked",
                "--offline",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            vec!["test", "--locked", "--offline", "--lib"],
        ] {
            let output = Command::new("cargo")
                .args(&command)
                .current_dir(&root)
                .env(
                    "CARGO_TARGET_DIR",
                    std::env::var_os("CARGO_TARGET_DIR")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| root.join("target")),
                )
                .env("XDG_CONFIG_HOME", temp.path().join("config"))
                .env("XDG_DATA_HOME", temp.path().join("data"))
                .env("XDG_CACHE_HOME", temp.path().join("cache"))
                .env("XDG_STATE_HOME", temp.path().join("state"))
                .output()?;
            results.push(serde_json::json!({"command":command,"ok":output.status.success(),"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr)}));
        }
        report["verification"] = serde_json::json!(results);
    }
    if args.iter().any(|a| a == "--gui") {
        let out = Command::new("python3")
            .arg(root.join("tests/run-preferences-smoke.py"))
            .current_dir(&root)
            .output()?;
        report["gui"] = serde_json::json!({"ok":out.status.success(),"stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)});
    }
    let ok = report["ok"] == true
        && report["verification"]
            .as_array()
            .is_none_or(|v| v.iter().all(|v| v["ok"] == true))
        && (report["gui"].is_null() || report["gui"]["ok"] == true);
    report["ok"] = serde_json::json!(ok);
    let text = serde_json::to_string_pretty(&report)?;
    if let Some(path) = value("--output") {
        std::fs::write(path, &text)?;
    }
    if args.iter().any(|a| a == "--json") {
        println!("{text}");
    } else {
        println!(
            "Duvar {} — {}",
            env!("CARGO_PKG_VERSION"),
            if ok {
                "SAĞLIKLI"
            } else {
                "İNCELEME GEREKİYOR"
            }
        );
        if let Some(list) = report["checks"].as_array() {
            for c in list {
                println!(
                    "{}: {} {}",
                    c["name"].as_str().unwrap_or(""),
                    c["status"].as_str().unwrap_or(""),
                    c["detail"]
                );
            }
        }
        if !report["verification"].is_null() {
            println!("{}", report["verification"]);
        }
        if !report["gui"].is_null() {
            println!("{}", report["gui"]);
        }
    }
    Ok(ok)
}
