//! Read-only local compatibility report. Network probes require an explicit opt-in.
use serde::Serialize;
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};
#[derive(Serialize)]
pub struct Check {
    pub name: String,
    pub status: String,
    pub detail: Value,
}
fn check(name: &str, status: &str, detail: Value) -> Check {
    Check {
        name: name.into(),
        status: status.into(),
        detail,
    }
}
fn hash(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn json_file(path: &Path, max: u64) -> anyhow::Result<Value> {
    if fs::metadata(path)?.len() > max {
        anyhow::bail!("Boyut sınırı");
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
pub fn report(extension: &Path, network: bool, full: bool) -> Value {
    let mut checks = Vec::new();
    checks.push(check("runtime","ok",json!({"gtk":format!("{}.{}.{}",gtk::major_version(),gtk::minor_version(),gtk::micro_version()),"adwaita":format!("{}.{}.{}",unsafe{adw::ffi::adw_get_major_version()},unsafe{adw::ffi::adw_get_minor_version()},unsafe{adw::ffi::adw_get_micro_version()}),"duvar":env!("CARGO_PKG_VERSION"),"architecture":std::env::consts::ARCH})));
    for (name, rel) in [
        ("library", "native/libduvar_native.so"),
        ("gir", "native/Duvar-1.0.gir"),
        ("typelib", "native/Duvar-1.0.typelib"),
        ("schema", "schemas/gschemas.compiled"),
        ("preferences", "prefs.js"),
    ] {
        let p = extension.join(rel);
        let result = fs::metadata(&p).is_ok_and(|m| m.is_file() && m.len() > 0);
        checks.push(check(
            name,
            if result { "ok" } else { "error" },
            json!({"present":result,"sha256":hash(&p).ok()}),
        ));
    }
    let manifest = extension.join(crate::distribution::BUILD_MANIFEST);
    match json_file(&manifest, 128 * 1024) {
        Ok(manifest) => {
            let verified = crate::distribution::verify_manifest(extension, &manifest);
            checks.push(check("package_integrity",if verified.is_ok(){"ok"}else{"error"},json!({"version":manifest["version"],"verified":verified.is_ok(),"error":verified.err().map(|e|crate::errors::redact(&e.to_string(),true))})));
        }
        Err(_) => checks.push(check(
            "package_integrity",
            "warning",
            json!("Eski paket: taşınabilir hash manifesti yok"),
        )),
    }
    let gir = fs::read_to_string(extension.join("native/Duvar-1.0.gir")).unwrap_or_default();
    let native = crate::distribution::preflight(extension);
    checks.push(match native {
        Ok(c) => check(
            "native_gir_contract",
            if gir.contains("duvar_populate") && gir.contains("duvar_version") {
                "ok"
            } else {
                "error"
            },
            json!(c),
        ),
        Err(e) => check(
            "native_gir_contract",
            "error",
            json!(crate::errors::redact(&e.to_string(), true)),
        ),
    });
    let metadata = json_file(&extension.join("metadata.json"), 65536);
    checks.push(match metadata {
        Ok(v) => check(
            "extension_metadata",
            if v["version-name"]
                .as_str()
                .is_some_and(|s| s.starts_with(env!("CARGO_PKG_VERSION")))
            {
                "ok"
            } else {
                "error"
            },
            json!({"version":v["version-name"],"shell":v["shell-version"]}),
        ),
        Err(_) => check("extension_metadata", "error", json!("Metadata okunamadı")),
    });
    let config = glib::user_config_dir().join("duvar/preferences.json");
    if config.exists() {
        checks.push(match json_file(&config, 1024 * 1024) {
            Ok(v) => {
                let valid = v["schema_version"].as_u64().unwrap_or(0) <= 2
                    && serde_json::from_value::<crate::model::Config>(v.clone()).is_ok();
                check(
                    "config_schema",
                    if valid { "ok" } else { "error" },
                    json!({"schema":v["schema_version"],"valid":valid}),
                )
            }
            Err(_) => check(
                "config_schema",
                "error",
                json!("Ayar JSON dosyası okunamıyor"),
            ),
        });
    } else {
        checks.push(check(
            "config_schema",
            "ok",
            json!("İlk kullanım; dosya yok"),
        ));
    }
    let queue = crate::store::queue_path();
    if queue.exists() {
        let result = fs::read(&queue)
            .ok()
            .filter(|v| v.len() <= 4 * 1024 * 1024)
            .and_then(|b| crate::queue::decode(&b).ok());
        checks.push(match result {
            Some((e, w)) => check(
                "queue_schema",
                if w.is_empty() { "ok" } else { "warning" },
                json!({"schema":e.schema_version,"jobs":e.jobs.len(),"recovery_notes":w}),
            ),
            None => check(
                "queue_schema",
                "error",
                json!("Kuyruk okunamıyor; doktor dosyayı değiştirmedi"),
            ),
        });
    }
    let mut good = 0;
    let mut missing = 0;
    let mut changed = 0;
    let mut legacy = 0;
    let mut sampled = 0;
    match crate::store::downloaded() {
        Ok(records) => {
            for r in &records {
                if !Path::new(&r.path).exists() {
                    missing += 1;
                } else if crate::files::managed_name(r) {
                    if !full && sampled >= 32 {
                        continue;
                    }
                    sampled += 1;
                    if crate::files::owns(r).is_ok() {
                        good += 1;
                    } else {
                        changed += 1;
                    }
                } else {
                    legacy += 1;
                }
            }
            checks.push(check(
                "receipts",
                if changed > 0 || crate::store::bad_receipts()>0 { "warning" } else { "ok" },
                json!({"verified":good,"missing":missing,"changed":changed,"legacy":legacy,"hash_sampled":sampled,"full_hash_audit":full,"bad_records":crate::store::bad_receipts()}),
            ));
        }
        Err(_) => checks.push(check(
            "receipts",
            "error",
            json!("Makbuz dizini okunamıyor"),
        )),
    }
    checks.push(match crate::cache::stats() {
        Ok(s) => check(
            "cache",
            if s.corrupt > 0 { "warning" } else { "ok" },
            json!(s),
        ),
        Err(e) => check(
            "cache",
            "error",
            json!(crate::errors::redact(&e.to_string(), true)),
        ),
    });
    let mut ids = std::collections::BTreeSet::new();
    let valid = crate::registry::SOURCES.iter().all(|s| ids.insert(s.id));
    checks.push(check(
        "provider_contracts",
        if valid { "ok" } else { "error" },
        json!({"count":ids.len(),"contracts":crate::registry::SOURCES}),
    ));
    if network {
        let sources = crate::check_sources();
        let ok = sources
            .as_object()
            .is_some_and(|m| m.values().all(|v| v["ok"] == true));
        checks.push(check("network", if ok { "ok" } else { "error" }, sources));
    }
    let ok = !checks.iter().any(|c| c.status == "error");
    json!({"schema":1,"version":env!("CARGO_PKG_VERSION"),"at":crate::runtime::now(),"ok":ok,"read_only":true,"network_opt_in":network,"checks":checks})
}
