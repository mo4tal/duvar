pub mod cache;
mod catalog;
pub mod diagnostics;
pub mod distribution;
pub mod doctor;
mod engine;
pub mod errors;
pub mod files;
pub mod i18n;
pub mod library;
mod model;
mod providers;
pub mod queue;
pub mod registry;
pub mod runtime;
mod sources;
mod store;
mod thumbnail;
mod ui;

/// Attach the Rust UI to GNOME's actual extension preferences window.
///
/// # Safety
/// The host calls this on GTK's main thread with a live AdwPreferencesWindow
/// and a valid, NUL-terminated UTF-8 extension directory for the call duration.
#[allow(
    deprecated,
    reason = "GNOME 51 preferences API passes an AdwPreferencesWindow"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duvar_populate(
    window: *mut adw::ffi::AdwPreferencesWindow,
    directory: *const libc::c_char,
) {
    let result = std::panic::catch_unwind(|| {
        use glib::translate::from_glib_none;
        if window.is_null() || directory.is_null() {
            return;
        }
        if !gtk::is_initialized() {
            unsafe {
                gtk::set_initialized();
            }
        }
        let window: adw::PreferencesWindow = unsafe { from_glib_none(window) };
        let directory = unsafe { std::ffi::CStr::from_ptr(directory) }.to_string_lossy();
        ui::populate(&window, std::path::Path::new(directory.as_ref()));
    });
    if result.is_err() {
        eprintln!("Duvar: preferences initialization failed");
    }
}

/// Developer diagnostic: metadata only, with the same fail-closed default policy.
pub fn check_sources() -> serde_json::Value {
    let network = providers::Network::new();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let config = model::Config {
        query: "nature".into(),
        sort: "random".into(),
        ..Default::default()
    };
    let mut reports = serde_json::Map::new();
    for source in ["Wallhaven", "Openverse"] {
        let report = match network.search(source, &config, 1, "", &cancel, None) {
            Ok(batch) => {
                let items = batch.items;
                serde_json::json!({"ok":true,"accepted":items.len(),"all_sfw":items.iter().all(|w|w.rating==model::Rating::Safe)})
            }
            Err(e) => serde_json::json!({"ok":false,"error":e.to_string()}),
        };
        reports.insert(source.into(), report);
    }
    serde_json::Value::Object(reports)
}

/// Local maintenance diagnostic. Never reads a credential.
pub fn migrate_downloads() -> anyhow::Result<usize> {
    let config = store::load_config()?.unwrap_or_default();
    store::migrate_sidecars(&config)
}
/// Safe metadata and transfer timings; credentials and adult content are never fetched.
pub fn check_new_sources() -> serde_json::Value {
    let mut reports = serde_json::Map::new();
    let mut config = model::Config::default();
    config.policy.unknown = true;
    let network = providers::Network::new();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for provider in [
        "NASA",
        "Bing",
        "Yande.re",
        "AIBooru",
        "NekosAPI",
        "Cleveland",
        "The Met",
        "V&A",
    ] {
        if let Ok(only) = std::env::var("DUVAR_CHECK_SOURCE")
            && only != provider
        {
            continue;
        }
        let now = std::time::Instant::now();
        let report = match network.search(provider, &config, 1, "", &cancel, None) {
            Ok(batch) => {
                let query_ms = now.elapsed().as_millis();
                let mut report = serde_json::json!({"ok":true,"accepted":batch.items.len(),"query_ms":query_ms,"all_sfw":batch.items.iter().all(|i|i.rating==model::Rating::Safe)});
                if let Some(item) = batch.items.first() {
                    let now = std::time::Instant::now();
                    report["thumbnail_ok"] =
                        serde_json::json!(network.thumbnail(item, &cancel).is_ok());
                    report["thumbnail_ms"] = serde_json::json!(now.elapsed().as_millis());
                    let now = std::time::Instant::now();
                    report["preview_ok"] =
                        serde_json::json!(network.preview(item, &cancel).is_ok());
                    report["preview_ms"] = serde_json::json!(now.elapsed().as_millis());
                }
                report
            }
            Err(e) => serde_json::json!({"ok":false,"error":e.to_string()}),
        };
        reports.insert(provider.into(), report);
    }
    serde_json::Value::Object(reports)
}

#[unsafe(no_mangle)]
pub extern "C" fn duvar_version() -> *const libc::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}
