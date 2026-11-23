pub mod http;
pub mod metadata;
pub mod transfer;
use crate::{
    model::{Config, Rating, Wallpaper},
    store,
};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use url::Url;
pub struct SearchBatch {
    pub rejected: usize,
    pub items: Vec<Wallpaper>,
    pub has_more: bool,
}

pub struct Network {
    agent: ureq::Agent,
    wallhaven_at: Mutex<Instant>,
    assets: Mutex<std::collections::BTreeSet<String>>,
    asset_ready: std::sync::Condvar,
    cooldowns: Mutex<std::collections::BTreeMap<String, u64>>,
}
impl Network {
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .https_only(true)
                .timeout_global(Some(Duration::from_secs(120)))
                .timeout_resolve(Some(Duration::from_secs(4)))
                .timeout_connect(Some(Duration::from_secs(5)))
                .timeout_recv_response(Some(Duration::from_secs(12)))
                .timeout_per_read(Some(Duration::from_secs(10)))
                .max_idle_connections(32)
                .max_idle_connections_per_host(8)
                .max_redirects(3)
                .build()
                .into(),
            assets: Mutex::new(std::collections::BTreeSet::new()),
            asset_ready: std::sync::Condvar::new(),
            cooldowns: Mutex::new(std::collections::BTreeMap::new()),
            wallhaven_at: Mutex::new(Instant::now() - Duration::from_secs(5)),
        }
    }
    pub fn bytes(&self, url: &str, limit: u64, cancel: &AtomicBool) -> Result<Vec<u8>> {
        check(cancel)?;
        valid_url(url)?;
        let mut response = self.request(
            url,
            if url.starts_with("https://api.openverse.org/") {
                25
            } else {
                20
            },
            cancel,
        )?;
        if response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .is_some_and(|n| n > limit)
        {
            return Err(crate::errors::Failure::new(
                crate::errors::Code::TooLarge,
                "Kaynak boyut sınırını aşıyor",
            )
            .into());
        }
        let mut reader = response.body_mut().as_reader().take(limit + 1);
        let mut data = Vec::new();
        let mut buffer = [0u8; 32768];
        loop {
            check(cancel)?;
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..n]);
            if data.len() as u64 > limit {
                bail!("Kaynak boyut sınırını aşıyor")
            }
        }
        Ok(data)
    }
    pub(crate) fn json(&self, url: &str, cancel: &AtomicBool) -> Result<Value> {
        let bytes = self.bytes(url, 4 * 1024 * 1024, cancel)?;
        if bytes.is_empty() {
            return Err(crate::errors::Failure::new(
                crate::errors::Code::EmptyBody,
                "Kaynak boş yanıt döndürdü",
            )
            .into());
        }
        if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'<') {
            return Err(crate::errors::Failure::new(
                crate::errors::Code::InvalidMetadata,
                "Kaynak JSON yerine HTML/erişim sayfası döndürdü",
            )
            .into());
        }
        serde_json::from_slice(&bytes).map_err(|_| {
            crate::errors::Failure::new(
                crate::errors::Code::InvalidMetadata,
                "Kaynak JSON yanıtı geçersiz",
            )
            .into()
        })
    }
    fn wallhaven(&self, url: &str, cancel: &AtomicBool) -> Result<Value> {
        {
            let mut last = self
                .wallhaven_at
                .lock()
                .map_err(|_| anyhow::anyhow!("İstek sırası kullanılamıyor"))?;
            while last.elapsed() < Duration::from_millis(1450) {
                check(cancel)?;
                std::thread::sleep(Duration::from_millis(50))
            }
            *last = Instant::now();
        }
        self.json(url, cancel)
    }
    pub fn search(
        &self,
        provider: &str,
        config: &Config,
        page: u32,
        key: &str,
        cancel: &AtomicBool,
        mut progress: Option<&mut dyn FnMut(Wallpaper)>,
    ) -> Result<SearchBatch> {
        match provider {
            "Wallhaven" => {
                if !config.categories.iter().any(|x| *x)
                    || config.policy.wallhaven_purity() == "000"
                {
                    return Ok(SearchBatch {
                        rejected: 0,
                        items: vec![],
                        has_more: false,
                    });
                }
                if config.policy.adult && key.is_empty() {
                    bail!(
                        "Wallhaven yetişkin içeriği için API anahtarı gerekiyor; anahtarı Kaynaklar bölümüne ekle"
                    )
                }
                let mut url = Url::parse("https://wallhaven.cc/api/v1/search")?;
                let wh_sort = match config.gallery_order.as_str() {
                    "newest" => "date_added",
                    "random" => "random",
                    _ => config.sort.as_str(),
                };
                let mut query_parts = Vec::new();
                if !config.query.trim().is_empty() {
                    query_parts.push(config.query.trim().to_string());
                }
                if config.policy.include_enabled && config.policy.all_included {
                    for term in crate::model::terms(&config.policy.include_tags) {
                        query_parts.push(format!("+\"{term}\""));
                    }
                }
                if config.policy.exclude_enabled {
                    for term in crate::model::terms(&config.policy.exclude_tags) {
                        query_parts.push(format!("-\"{term}\""));
                    }
                }
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("purity", &config.policy.wallhaven_purity())
                        .append_pair(
                            "categories",
                            &config
                                .categories
                                .map(|x| if x { '1' } else { '0' })
                                .iter()
                                .collect::<String>(),
                        )
                        .append_pair("sorting", wh_sort)
                        .append_pair("page", &page.to_string());
                    if config.policy.minimum_width > 0 || config.policy.minimum_height > 0 {
                        let w = config.policy.minimum_width.max(1);
                        let h = config.policy.minimum_height.max(1);
                        q.append_pair("atleast", &format!("{w}x{h}"));
                    }
                    match config.policy.orientation.as_str() {
                        "landscape" => {
                            q.append_pair("ratios", "landscape");
                        }
                        "portrait" => {
                            q.append_pair("ratios", "portrait");
                        }
                        "square" => {
                            q.append_pair("ratios", "1x1");
                        }
                        _ => {}
                    }
                    if !query_parts.is_empty() {
                        q.append_pair("q", &query_parts.join(" "));
                    }
                    if !key.is_empty() {
                        q.append_pair("apikey", key);
                    }
                }
                let data = self.wallhaven(url.as_str(), cancel)?;
                let entries = data["data"]
                    .as_array()
                    .context("Wallhaven yanıtı tanınmadı")?;
                let mut found = Vec::new();
                let mut rejected = 0;
                for entry in entries.iter().take(24) {
                    check(cancel)?;
                    let Ok(mut item) = parse_wallhaven(entry) else {
                        continue;
                    };
                    let category = match text(entry, "category").as_str() {
                        "general" => 0,
                        "anime" => 1,
                        "people" => 2,
                        _ => continue,
                    };
                    if !config.categories[category] {
                        continue;
                    }

                    if !config.policy.rating_allowed(item.rating) {
                        continue;
                    }
                    if config.policy.needs_tags() {
                        let mut url = Url::parse(&format!(
                            "https://wallhaven.cc/api/v1/w/{}",
                            item.id.trim_start_matches("wh:")
                        ))?;
                        if !key.is_empty() {
                            url.query_pairs_mut().append_pair("apikey", key);
                        }
                        let detail = match self.wallhaven(url.as_str(), cancel) {
                            Ok(v) => v,
                            Err(_) if !cancel.load(Ordering::Relaxed) => continue,
                            Err(e) => return Err(e),
                        };
                        item.tags = tags(&detail["data"]["tags"], "name");
                    }
                    if config.policy.accepts(&item) {
                        if let Some(emit) = progress.as_mut() {
                            emit(item.clone());
                        }
                        found.push(item)
                    } else {
                        rejected += 1;
                    }
                }
                Ok(SearchBatch {
                    rejected,
                    items: found,
                    has_more: data["meta"]["last_page"]
                        .as_u64()
                        .map_or(entries.len() == 24, |last| (page as u64) < last),
                })
            }
            "Openverse" => {
                if !config.policy.safe {
                    return Ok(SearchBatch {
                        rejected: 0,
                        items: vec![],
                        has_more: false,
                    });
                }
                let mut url = Url::parse("https://api.openverse.org/v1/images/")?;
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair(
                        "q",
                        if config.query.is_empty() {
                            "landscape"
                        } else {
                            &config.query
                        },
                    )
                    .append_pair("mature", "false")
                    .append_pair("page_size", "20")
                    .append_pair("page", &page.to_string());
                    match config.policy.orientation.as_str() {
                        "landscape" => {
                            q.append_pair("aspect_ratio", "wide");
                        }
                        "portrait" => {
                            q.append_pair("aspect_ratio", "tall");
                        }
                        "square" => {
                            q.append_pair("aspect_ratio", "square");
                        }
                        _ => {}
                    }
                    if !config.policy.formats.is_empty() {
                        q.append_pair("extension", &config.policy.formats.join(","));
                    }
                    if config.policy.minimum_width >= 1920 || config.policy.minimum_height >= 1080 {
                        q.append_pair("size", "large");
                    }
                }
                let data = self.json(url.as_str(), cancel)?;
                let list = data["results"]
                    .as_array()
                    .context("Openverse yanıtı tanınmadı")?;
                let items = list
                    .iter()
                    .take(24)
                    .filter_map(|v| parse_openverse(v).ok())
                    .collect::<Vec<_>>();
                let count = items.len();
                let items = items
                    .into_iter()
                    .filter(|w| w.rating != Rating::Adult && config.policy.accepts(w))
                    .collect::<Vec<_>>();
                let rejected = count - items.len();
                let has_more = data["page_count"]
                    .as_u64()
                    .map_or(list.len() == 20, |last| (page as u64) < last);
                Ok(SearchBatch {
                    rejected,
                    items,
                    has_more,
                })
            }
            "Yerel" => {
                let mut out = Vec::new();
                let mut rejected = 0;
                let root = Path::new(&config.local_directory);
                if !root.is_dir() {
                    bail!("Önce yerel resim klasörünü seç")
                }
                let start = (page.saturating_sub(1) as usize) * 24;
                let needle = config.query.trim().to_lowercase();
                let mut files = fs::read_dir(root)?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_file()
                            && p.extension().is_some_and(|s| {
                                ["jpg", "jpeg", "png", "webp"]
                                    .contains(&s.to_string_lossy().to_lowercase().as_str())
                            })
                            && (needle.is_empty()
                                || p.file_name().is_some_and(|n| {
                                    n.to_string_lossy().to_lowercase().contains(&needle)
                                }))
                    })
                    .take(2048)
                    .collect::<Vec<_>>();
                files.sort();
                let has_more = files.len() > start + 24;
                for path in files.into_iter().skip(start).take(24) {
                    check(cancel)?;
                    let (width, height) = image::image_dimensions(&path).unwrap_or((0, 0));
                    let created_at = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let uri = Url::from_file_path(&path)
                        .map_err(|_| anyhow::anyhow!("Dosya yolu geçersiz"))?
                        .to_string();
                    let item = Wallpaper {
                        id: format!("local:{}", store::key(&uri)),
                        provider: "Yerel".into(),
                        title: path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into(),
                        thumbnail: uri.clone(),
                        preview_url: uri.clone(),
                        image: uri.clone(),
                        landing: uri,
                        rating: Rating::Unknown,
                        tags: None,
                        width,
                        height,
                        author: String::new(),
                        license: "Kişisel dosya".into(),
                        local: true,
                        created_at,
                    };
                    if config.policy.accepts(&item) {
                        out.push(item)
                    } else {
                        rejected += 1;
                    }
                }
                Ok(SearchBatch {
                    rejected,
                    items: out,
                    has_more,
                })
            }
            _ => crate::sources::search(self, provider, config, page, key, cancel),
        }
    }
    pub fn thumbnail(&self, item: &Wallpaper, cancel: &AtomicBool) -> Result<store::Pixels> {
        let p = store::thumbnail_path(&item.id);
        if let Ok(pixels) = store::read_pixels(&p) {
            return Ok(pixels);
        }
        let data = if item.local {
            read_local(&item.thumbnail, 32 * 1024 * 1024)?
        } else {
            self.bytes(&item.thumbnail, 6 * 1024 * 1024, cancel)?
        };
        check(cancel)?;
        let pixels = store::decode(&data, 384)?;
        if store::prepare_cache().is_ok() {
            if let Err(e) = store::write_pixels(&p, &pixels) {
                crate::diagnostics::log(None, "cache", "thumbnail_write", &e.to_string(), None);
            } else {
                crate::cache::mark(&p, item.rating);
                let _ = crate::cache::trim(256 * 1024 * 1024, false);
            }
        }

        Ok(pixels)
    }
    #[cfg(test)]
    fn download(
        &self,
        item: &Wallpaper,
        directory: &Path,
        cancel: &AtomicBool,
    ) -> Result<std::path::PathBuf> {
        self.download_progress(item, directory, cancel, &mut |_, _, _, _, _| {})
            .map(|v| v.0)
    }
    pub fn preview(&self, item: &Wallpaper, cancel: &AtomicBool) -> Result<store::Pixels> {
        let raw = self.original(item, cancel, &mut |_, _, _, _, _| {})?;
        check(cancel)?;
        let data = read_local(
            url::Url::from_file_path(&raw.lease.path)
                .map_err(|_| anyhow::anyhow!("Dosya yolu geçersiz"))?
                .as_ref(),
            crate::files::MAX_FILE,
        )?;
        let pixels = store::decode_original(&data)?;
        check(cancel)?;
        Ok(pixels)
    }
}

fn read_local(uri: &str, limit: u64) -> Result<Vec<u8>> {
    let path = Url::parse(uri)?
        .to_file_path()
        .map_err(|_| anyhow::anyhow!("Dosya yolu geçersiz"))?;
    let meta = fs::metadata(&path)?;
    if !meta.is_file() || meta.len() > limit {
        bail!("Dosya boyutu uygun değil")
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("Dosya boyut sınırını aşıyor")
    }
    Ok(bytes)
}
pub fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        bail!("İptal edildi")
    }
    Ok(())
}
pub(crate) fn valid_url(value: &str) -> Result<()> {
    let u = Url::parse(value)?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.host_str().is_none()
    {
        bail!("Yalnızca HTTPS görsel adresleri destekleniyor")
    }
    let host = u.host_str().unwrap();
    if host == "localhost" || host.ends_with(".local") || host.parse::<std::net::IpAddr>().is_ok() {
        bail!("Yerel ağ adresi kaynak olarak kullanılamaz")
    }
    Ok(())
}
fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").chars().take(1024).collect()
}
fn tags(v: &Value, key: &str) -> Option<Vec<String>> {
    let values = v.as_array()?;
    if values.len() > 128 {
        return None;
    }
    values
        .iter()
        .map(|v| {
            let s = v[key].as_str()?;
            if s.chars().count() > 128 {
                return None;
            }
            Some(s.to_string())
        })
        .collect()
}
pub fn parse_wallhaven(v: &Value) -> Result<Wallpaper> {
    let raw = text(v, "id");
    if raw.is_empty() || raw.len() > 16 || !raw.bytes().all(|b| b.is_ascii_alphanumeric()) {
        bail!("Geçersiz görsel kimliği")
    };
    let thumbnail = text(&v["thumbs"], "small");
    let image = text(v, "path");
    valid_url(&thumbnail)?;
    valid_url(&image)?;
    Ok(Wallpaper {
        id: format!("wh:{raw}"),
        provider: "Wallhaven".into(),
        title: format!("Wallhaven · {raw}"),
        thumbnail,
        preview_url: if v.get("thumbs").is_some() {
            text(&v["thumbs"], "large")
        } else {
            String::new()
        },
        image,
        landing: format!("https://wallhaven.cc/w/{raw}"),
        rating: match text(v, "purity").as_str() {
            "sfw" => Rating::Safe,
            "sketchy" => Rating::Sketchy,
            "nsfw" => Rating::Adult,
            _ => Rating::Unknown,
        },
        tags: tags(&v["tags"], "name"),
        width: crate::model::dimension(&v["dimension_x"]),
        height: crate::model::dimension(&v["dimension_y"]),
        author: String::new(),
        license: "Kaynak görselin haklarını kontrol et".into(),
        local: false,
        created_at: parse_date(&text(v, "created_at")),
    })
}
pub fn parse_openverse(v: &Value) -> Result<Wallpaper> {
    let id = text(v, "id");
    if id.is_empty() || id.len() > 80 {
        bail!("Geçersiz görsel kimliği")
    };
    let image = text(v, "url");
    let thumbnail = text(v, "thumbnail");
    valid_url(&image)?;
    valid_url(&thumbnail)?;
    let rating = match v["mature"].as_bool() {
        Some(false) => Rating::Safe,
        Some(true) => Rating::Adult,
        None => Rating::Unknown,
    };
    Ok(Wallpaper {
        id: format!("ov:{id}"),
        provider: format!("Openverse / {}", text(v, "source")),
        title: text(v, "title"),
        thumbnail,
        preview_url: String::new(),
        image,
        landing: text(v, "foreign_landing_url"),
        rating,
        tags: tags(&v["tags"], "name"),
        width: crate::model::dimension(&v["width"]),
        height: crate::model::dimension(&v["height"]),
        author: text(v, "creator"),
        license: format!("{} {}", text(v, "license"), text(v, "license_version")),
        local: false,
        created_at: 0,
    })
}
fn secret_command() -> std::process::Command {
    use std::os::unix::process::CommandExt;
    let mut command = std::process::Command::new("/usr/bin/secret-tool");
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() == 1 {
                libc::_exit(1)
            }
            Ok(())
        });
    }
    command
}
fn wait_secret(
    mut child: std::process::Child,
    cancel: &AtomicBool,
) -> Result<std::process::Output> {
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
        if cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(45) {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Anahtarlık işlemi iptal edildi veya zaman aşımına uğradı")
        }
        std::thread::sleep(Duration::from_millis(40))
    }
}
pub fn save_key_for(provider: &str, key: &str, cancel: &AtomicBool) -> Result<()> {
    if key.len() > 2048 || key.contains('\n') {
        bail!("API anahtarı biçimi geçersiz")
    }
    // Refuse automatic collection creation: only an existing default keyring is used.
    // secret-tool store updates an item; no collection/password mutation API is called.
    use gio::prelude::*;
    let service = gio::DBusProxy::for_bus_sync(
        gio::BusType::Session,
        gio::DBusProxyFlags::NONE,
        None,
        "org.freedesktop.secrets",
        "/org/freedesktop/secrets",
        "org.freedesktop.Secret.Service",
        None::<&gio::Cancellable>,
    )?;
    let alias = service.call_sync(
        "ReadAlias",
        Some(&("default",).to_variant()),
        gio::DBusCallFlags::NONE,
        5000,
        None::<&gio::Cancellable>,
    )?;
    if alias.child_value(0).str().is_none_or(|path| path == "/") {
        bail!(
            "Varsayılan anahtarlık bulunamadı. Mevcut anahtarlığını varsayılan seç; Duvar anahtarlık oluşturmaz veya parolasını değiştirmez."
        );
    }
    if load_key_for(provider, cancel)? == key {
        return Ok(());
    }
    let mut command = secret_command();
    let mut child = command
        .args([
            "store",
            &format!("--label=Duvar · {provider} API"),
            "application",
            "duvar",
            "provider",
            provider,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .context("Anahtarlık açılamadı")?
        .write_all(key.as_bytes())?;
    if !wait_secret(child, cancel)?.status.success() {
        bail!("Anahtarlığa kaydedilemedi")
    };
    Ok(())
}
pub fn load_key_for(provider: &str, cancel: &AtomicBool) -> Result<String> {
    let mut command = secret_command();
    let child = command
        .args(["lookup", "application", "duvar", "provider", provider])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let output = wait_secret(child, cancel)?;
    if !output.status.success() {
        return Ok(String::new());
    }
    let key = String::from_utf8(output.stdout)?.trim().to_string();
    if key.len() > 2048 {
        bail!("Anahtar uzunluğu geçersiz")
    }
    Ok(key)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn unknown_rating_stays_unknown() {
        let w=parse_openverse(&json!({"id":"a","url":"https://example.org/a.jpg","thumbnail":"https://example.org/a.jpg","source":"fixture"})).unwrap();
        assert_eq!(w.rating, Rating::Unknown);
        assert!(!Config::default().policy.accepts(&w))
    }
    #[test]
    fn reject_credentials_and_non_https() {
        for u in [
            "http://example.org/a",
            "https://user:pass@example.org/a",
            "https://localhost/a",
            "file:///tmp/a",
        ] {
            assert!(valid_url(u).is_err())
        }
    }
}

#[cfg(test)]
mod metadata_integrity_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn incomplete_or_truncated_tag_data_is_not_treated_as_verified() {
        assert!(tags(&json!([{"name":"nature"},{}]), "name").is_none());
        assert!(
            tags(
                &serde_json::Value::Array((0..129).map(|_| json!({"name":"nature"})).collect()),
                "name"
            )
            .is_none()
        );
    }
}

#[cfg(test)]
mod download_tests {
    use super::*;
    #[test]
    fn real_image_download_uses_destination_and_preserves_conflicting_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([30, 60, 90, 255]))
            .save(&source)
            .unwrap();
        let uri = Url::from_file_path(&source).unwrap().to_string();
        let item = Wallpaper {
            id: "local:fixture".into(),
            provider: "Yerel".into(),
            title: "Fixture".into(),
            thumbnail: uri.clone(),
            preview_url: uri.clone(),
            image: uri.clone(),
            landing: uri,
            rating: Rating::Unknown,
            tags: None,
            width: 8,
            height: 8,
            author: String::new(),
            license: String::new(),
            local: true,
            created_at: 0,
        };
        let cancel = AtomicBool::new(false);
        let network = Network::new();
        let destination = temp.path().join("selected-folder");
        let path = network.download(&item, &destination, &cancel).unwrap();
        assert_eq!(path.parent().unwrap(), destination);
        assert_eq!(fs::read(&path).unwrap(), fs::read(&source).unwrap());
        assert!(!path.with_extension("png.json").exists());
        assert_eq!(
            network.download(&item, &destination, &cancel).unwrap(),
            path
        );
        fs::write(&path, b"user file").unwrap();
        assert!(network.download(&item, &destination, &cancel).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"user file");
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    #[test]
    #[ignore = "Explicit network diagnostic; downloads one SFW result per provider into a temporary directory"]
    fn safe_remote_downloads() {
        let temp = tempfile::tempdir().unwrap();
        let network = Network::new();
        let config = Config::default();
        let cancel = AtomicBool::new(false);
        for provider in ["Wallhaven", "Openverse"] {
            match network.search(provider, &config, 1, "", &cancel, None) {
                Ok(batch) => {
                    let Some(item) = batch.items.first() else {
                        println!("{provider}: no safe results");
                        continue;
                    };
                    assert_eq!(item.rating, Rating::Safe);
                    match network.download(item, temp.path(), &cancel) {
                        Ok(path) => println!(
                            "{provider}: validated download {} bytes",
                            fs::metadata(path).unwrap().len()
                        ),
                        Err(e) => panic!("{provider}: download failed: {e}"),
                    }
                }
                Err(e) => println!("{provider}: source unavailable: {e}"),
            }
        }
    }
}

pub(crate) fn parse_date(text: &str) -> i64 {
    glib::DateTime::from_iso8601(&text.replace(' ', "T"), Some(&glib::TimeZone::utc()))
        .map(|d| d.to_unix())
        .unwrap_or(0)
}

#[cfg(test)]
mod date_tests {
    use super::*;
    #[test]
    fn unzoned_source_dates_sort_chronologically() {
        assert_eq!(
            parse_date("2026-11-13 14:30:00"),
            parse_date("2026-11-13T14:30:00Z")
        );
        assert!(parse_date("2026-11-13 14:30:00") > 0);
        assert_eq!(parse_date("bad date"), 0);
    }
}

pub fn remove_key_for(provider: &str, cancel: &AtomicBool) -> Result<()> {
    if provider != "wallhaven" {
        bail!("Bilinmeyen anahtar sağlayıcısı");
    }
    let child = secret_command()
        .args(["clear", "application", "duvar", "provider", provider])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let output = wait_secret(child, cancel)?;
    if output.status.success() || output.status.code() == Some(1) {
        Ok(())
    } else {
        bail!("Anahtar kaldırılamadı; mevcut anahtarlık değişmedi")
    }
}
