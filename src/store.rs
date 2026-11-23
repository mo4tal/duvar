use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
pub fn key(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
pub fn cache() -> PathBuf {
    glib::user_cache_dir().join("duvar")
}
pub fn downloads() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Pictures)
        .unwrap_or_else(|| glib::home_dir().join("Pictures"))
        .join("Duvar")
}
pub fn atomic(p: &Path, bytes: &[u8]) -> Result<()> {
    let parent = p.parent().context("Dosya yolu eksik")?;
    fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(bytes)?;
    f.as_file().sync_all()?;
    f.persist(p).map_err(|e| e.error)?;
    Ok(())
}
/// Atomic rename without fsync for regenerable cache files (thumbnails, cache markers).
pub fn atomic_fast(p: &Path, bytes: &[u8]) -> Result<()> {
    let parent = p.parent().context("Dosya yolu eksik")?;
    fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(bytes)?;
    f.persist(p).map_err(|e| e.error)?;
    Ok(())
}
pub fn prepare_cache() -> Result<()> {
    private_dir(&cache())
}
#[derive(Debug)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub fn thumbnail_path(id: &str) -> PathBuf {
    cache().join(format!("{}.rgba", key(id)))
}
pub fn read_pixels(p: &Path) -> Result<Pixels> {
    if fs::metadata(p)?.len() > 1280 * 1280 * 4 + 8 {
        bail!("Önizleme boyutu geçersiz")
    }
    let mut b = Vec::new();
    fs::File::open(p)?
        .take(1280 * 1280 * 4 + 9)
        .read_to_end(&mut b)?;
    if b.len() < 8 {
        bail!("Eksik önizleme")
    }
    let width = u32::from_le_bytes(b[..4].try_into()?);
    let height = u32::from_le_bytes(b[4..8].try_into()?);
    if width == 0
        || height == 0
        || width > 1280
        || height > 1280
        || b.len() != 8 + (width as usize * height as usize * 4)
    {
        bail!("Geçersiz önizleme")
    }
    Ok(Pixels {
        width,
        height,
        rgba: b[8..].to_vec(),
    })
}
pub fn write_pixels(p: &Path, pixels: &Pixels) -> Result<()> {
    let mut bytes = Vec::with_capacity(pixels.rgba.len() + 8);
    bytes.extend(pixels.width.to_le_bytes());
    bytes.extend(pixels.height.to_le_bytes());
    bytes.extend(&pixels.rgba);
    atomic_fast(p, &bytes)
}
// Gate only full-resolution original decodes (~150-256 MiB spikes); thumbnails
// decode concurrently across worker threads with bounded allocation limits.
static DECODE_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub fn decode(bytes: &[u8], max: u32) -> Result<Pixels> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    use image::ImageDecoder;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut decoded = image::DynamicImage::from_decoder(decoder)?;
    decoded.apply_orientation(orientation);
    let image = decoded.thumbnail(max, max).to_rgba8();
    Ok(Pixels {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broken_cache_is_rejected() {
        let t = tempfile::NamedTempFile::new().unwrap();
        fs::write(t.path(), [255; 12]).unwrap();
        assert!(read_pixels(t.path()).is_err())
    }
    #[test]
    fn pixels_round_trip() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("a");
        write_pixels(
            &p,
            &Pixels {
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3, 255],
            },
        )
        .unwrap();
        assert_eq!(read_pixels(&p).unwrap().rgba, [1, 2, 3, 255])
    }
}

// A private atomic copy survives immediate prefs-process exit and dconf backend issues.
pub fn config_path() -> PathBuf {
    glib::user_config_dir().join("duvar/preferences.json")
}
static CONFIG_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static CONFIG_WRITTEN: std::sync::Mutex<u64> = std::sync::Mutex::new(0);
pub fn config_revision() -> u64 {
    CONFIG_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
pub fn save_config(config: &crate::model::Config) -> Result<()> {
    save_config_revision(config_revision(), config)
}
pub fn save_config_revision(revision: u64, config: &crate::model::Config) -> Result<()> {
    let mut written = CONFIG_WRITTEN
        .lock()
        .map_err(|_| anyhow::anyhow!("Ayar kayıt kilidi kullanılamıyor"))?;
    if revision <= *written {
        return Ok(());
    }
    if !config_writable() {
        bail!("Ayar dosyası daha yeni şemada; salt okunur korunuyor");
    }
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap())?;
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o700))?;
    atomic(&path, &serde_json::to_vec(&config.persistent())?)?;
    *written = revision;
    Ok(())
}
pub fn load_config() -> Result<Option<crate::model::Config>> {
    let path = config_path();
    match fs::metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(m) if m.len() > 4 * 1024 * 1024 => bail!("Ayar dosyası boyut sınırını aşıyor"),
        _ => {}
    }
    let bytes = fs::read(&path)?;
    let raw: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            let saved = path.with_extension(format!("invalid-{}.json", crate::runtime::now()));
            fs::rename(&path, &saved)?;
            crate::diagnostics::log(
                None,
                "settings",
                "recovery",
                "Bozuk ayar korunarak GNOME kaydına dönüldü",
                None,
            );
            return Err(e.into());
        }
    };
    if raw["schema_version"].as_u64().is_some_and(|n| n > 2) {
        CONFIG_BLOCKED.store(true, std::sync::atomic::Ordering::Relaxed);
        bail!("Ayar şeması daha yeni; dosya salt okunur korunuyor");
    }
    let mut merged = serde_json::to_value(crate::model::Config::default())?;
    merge_valid(&mut merged, &raw, "settings");
    let mut config: crate::model::Config = serde_json::from_value(merged)?;
    config.schema_version = 2;
    config.validate();
    Ok(Some(config))
}

pub fn metadata_dir() -> PathBuf {
    glib::user_data_dir().join("duvar/metadata")
}
fn private_dir(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        bail!("Özel veri klasörü sembolik bağlantı olamaz");
    }
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
pub fn receipt(item: &crate::model::Wallpaper, path: &Path) -> Result<()> {
    let facts = crate::files::facts(path, false)?;
    receipt_with_facts(item, path, facts)
}
pub fn receipt_with_facts(
    item: &crate::model::Wallpaper,
    path: &Path,
    facts: crate::runtime::Facts,
) -> Result<()> {
    private_dir(&metadata_dir())?;
    let mut item = item.clone();
    item.width = facts.width;
    item.height = facts.height;
    let record = crate::model::DownloadedImage {
        item,
        path: path.to_string_lossy().into(),
        schema_version: 2,
        facts,
        downloaded_at: crate::runtime::now(),
    };
    atomic(
        &metadata_dir().join(format!("{}.json", key(&record.path))),
        &serde_json::to_vec(&record)?,
    )
}
static BAD_RECEIPTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub fn bad_receipts() -> usize {
    BAD_RECEIPTS.load(std::sync::atomic::Ordering::Relaxed)
}
pub fn downloaded() -> Result<Vec<crate::model::DownloadedImage>> {
    if !metadata_dir().is_dir() {
        return Ok(vec![]);
    }
    let mut files = fs::read_dir(metadata_dir())?
        .filter_map(Result::ok)
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect::<Vec<_>>();
    files.sort_by_key(|a| std::cmp::Reverse(a.0));
    let mut out = Vec::new();
    let mut bad = 0;
    for (_, path) in files.into_iter().take(5000) {
        let record = (|| -> Result<crate::model::DownloadedImage> {
            let m = fs::symlink_metadata(&path)?;
            if !m.is_file() || m.len() > 256 * 1024 {
                bail!("Geçersiz makbuz");
            }
            let record: crate::model::DownloadedImage = serde_json::from_slice(&fs::read(&path)?)?;
            if !Path::new(&record.path).is_absolute()
                || !path
                    .file_stem()
                    .is_some_and(|s| s.to_string_lossy() == key(&record.path))
            {
                bail!("Makbuz yol kimliği uyuşmuyor");
            }
            Ok(record)
        })();
        match record {
            Ok(record) => out.push(record),
            Err(_) => bad += 1,
        }
    }
    BAD_RECEIPTS.store(bad, std::sync::atomic::Ordering::Relaxed);
    if bad > 0 {
        crate::diagnostics::log(
            None,
            "receipts",
            "invalid_records",
            &format!("{bad} geçersiz kayıt atlandı; sağlam kayıtlar korundu"),
            None,
        );
    }
    Ok(out)
}
pub fn migrate_sidecars(config: &crate::model::Config) -> Result<usize> {
    let mut roots = vec![downloads(), glib::user_data_dir().join("duvar/wallpapers")];
    if !config.download_directory.is_empty() {
        roots.push(PathBuf::from(&config.download_directory));
    }
    roots.sort();
    roots.dedup();
    let mut count = 0;
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for e in fs::read_dir(root)?.take(5000).filter_map(Result::ok) {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with("duvar-")
                || !name.ends_with(".json")
                || !e.metadata().is_ok_and(|m| m.len() < 256 * 1024)
            {
                continue;
            }
            let image = p.with_file_name(name.trim_end_matches(".json"));
            if !image.is_file() {
                continue;
            }
            if let Ok(item) = serde_json::from_slice::<crate::model::Wallpaper>(&fs::read(&p)?) {
                if !name.starts_with(&format!("duvar-{}-", &key(&item.id)[..12])) {
                    continue;
                }
                receipt(&item, &image)?;
                fs::remove_file(p)?;
                count += 1;
            }
        }
    }
    Ok(count)
}
pub fn queue_path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local/state"))
        .join("duvar/download-queue.json")
}
static QUEUE_META: std::sync::LazyLock<std::sync::Mutex<(crate::queue::ApplyOrder, bool)>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new((Default::default(), false)));
pub fn queue_metadata() -> (crate::queue::ApplyOrder, bool) {
    *QUEUE_META.lock().unwrap_or_else(|e| e.into_inner())
}
static QUEUE_SAVE: std::sync::Mutex<u64> = std::sync::Mutex::new(0);
static QUEUE_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static QUEUE_BLOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn queue_revision() -> u64 {
    QUEUE_REVISION.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
pub fn save_queue_state(
    entries: &[crate::model::DownloadRecord],
    latest: crate::queue::ApplyOrder,
    paused: bool,
) -> Result<()> {
    save_queue_revision(queue_revision(), entries, latest, paused)
}
pub fn save_queue_revision(
    revision: u64,
    entries: &[crate::model::DownloadRecord],
    latest: crate::queue::ApplyOrder,
    paused: bool,
) -> Result<()> {
    let mut applied = QUEUE_SAVE.lock().unwrap_or_else(|e| e.into_inner());
    if revision <= *applied {
        return Ok(());
    }
    if QUEUE_BLOCKED.load(std::sync::atomic::Ordering::Acquire) {
        bail!("Okunamayan kuyruk korunuyor; tanı raporuyla kurtarma gerekli");
    }
    private_dir(queue_path().parent().unwrap())?;
    if queue_path().exists()
        && let Ok(data) = fs::read(queue_path())
        && let Ok(v) = serde_json::from_slice::<serde_json::Value>(&data)
        && v["schema_version"].as_u64().is_some_and(|n| n > 2)
    {
        bail!("Daha yeni kuyruk şeması korunuyor");
    }
    let value = crate::queue::Envelope {
        schema_version: 2,
        latest_apply: latest.latest(),
        apply_order: latest,
        paused,
        jobs: entries.to_vec(),
    };
    atomic(&queue_path(), &serde_json::to_vec(&value)?)?;
    *applied = revision;
    *QUEUE_META.lock().unwrap_or_else(|e| e.into_inner()) = (latest, paused);
    Ok(())
}
pub fn load_queue() -> Result<Vec<crate::model::DownloadRecord>> {
    let path = queue_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    if fs::metadata(&path)?.len() > 4 * 1024 * 1024 {
        QUEUE_BLOCKED.store(true, std::sync::atomic::Ordering::Release);
        bail!("İndirme kuyruğu boyutu geçersiz");
    }
    let (value, warnings) = match crate::queue::decode(&fs::read(path)?) {
        Ok(v) => v,
        Err(e) => {
            QUEUE_BLOCKED.store(true, std::sync::atomic::Ordering::Release);
            return Err(e);
        }
    };
    *QUEUE_META.lock().unwrap_or_else(|e| e.into_inner()) = (value.apply_order, value.paused);
    for note in warnings {
        crate::diagnostics::log(None, "queue", "recovery", &note, None);
    }
    Ok(value.jobs)
}
pub fn state_dir() -> PathBuf {
    queue_path().parent().unwrap().to_path_buf()
}
pub fn decode_permit() -> Result<std::sync::MutexGuard<'static, ()>> {
    DECODE_GATE
        .lock()
        .map_err(|_| anyhow::anyhow!("Görsel çözücü kilidi kullanılamıyor"))
}
// Fast header validation avoids resizing a multi-megapixel original just to save it.
pub fn image_info(data: &[u8]) -> Result<(u32, u32, image::ImageFormat)> {
    let reader = image::ImageReader::new(std::io::Cursor::new(data)).with_guessed_format()?;
    let format = reader.format().context("Görsel biçimi tanınmadı")?;
    let (w, h) = reader.into_dimensions()?;
    if w == 0 || h == 0 || w > 16384 || h > 16384 || (w as u64 * h as u64) > 64 * 1024 * 1024 {
        bail!("Görsel boyut sınırını aşıyor");
    }
    Ok((w, h, format))
}

pub fn decode_original(bytes: &[u8]) -> Result<Pixels> {
    let _span = crate::diagnostics::Span::new(None, "decode_original");
    let _permit = DECODE_GATE
        .lock()
        .map_err(|_| anyhow::anyhow!("Görsel çözücü kullanılamıyor"))?;
    let (w, h, _) = image_info(bytes)?;
    if w as u64 * h as u64 > 48 * 1024 * 1024 {
        bail!(
            "Bu özgün görsel önizleme için fazla büyük (48 megapiksel sınırı). Dosyayı indirip görüntüleyicide açabilirsin."
        );
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    use image::ImageDecoder;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut decoded = image::DynamicImage::from_decoder(decoder)?;
    decoded.apply_orientation(orientation);
    let image = decoded.into_rgba8();
    Ok(Pixels {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

#[cfg(test)]
mod original_tests {
    use super::*;
    #[test]
    fn original_preview_retains_native_pixels() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("wide.png");
        image::RgbaImage::from_pixel(1800, 1000, image::Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        let bytes = fs::read(&path).unwrap();
        let small = decode(&bytes, 384).unwrap();
        let original = decode_original(&bytes).unwrap();
        assert_eq!(original.width, 1800);
        assert_eq!(original.height, 1000);
        assert!(small.width <= 384);
        assert_eq!(original.rgba.len(), 1800 * 1000 * 4);
    }
}

static CONFIG_BLOCKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn config_writable() -> bool {
    !CONFIG_BLOCKED.load(std::sync::atomic::Ordering::Relaxed)
}
fn merge_valid(default: &mut serde_json::Value, raw: &serde_json::Value, field: &str) {
    if let Some(array) = raw.as_array() {
        if field.ends_with(".favorites") {
            *default = serde_json::Value::Array(
                array
                    .iter()
                    .filter(|v| {
                        serde_json::from_value::<crate::model::Wallpaper>((*v).clone()).is_ok()
                    })
                    .take(128)
                    .cloned()
                    .collect(),
            );
            return;
        }
        if field.ends_with(".categories") {
            if let Some(base) = default.as_array_mut() {
                for (i, v) in array.iter().take(3).enumerate() {
                    if v.is_boolean() {
                        base[i] = v.clone();
                    }
                }
            }
            return;
        }
        if default.is_array() {
            *default = serde_json::Value::Array(
                array
                    .iter()
                    .filter(|v| v.is_string())
                    .take(200)
                    .cloned()
                    .collect(),
            );
            return;
        }
    }
    if default.is_number()
        && (!raw.is_u64() || (field != ".cache_mib" && raw.as_u64().unwrap_or(0) > u32::MAX as u64))
    {
        crate::diagnostics::log(None, "settings", "field_recovered", field, None);
        return;
    }

    if let (Some(base), Some(incoming)) = (default.as_object_mut(), raw.as_object()) {
        for (key, value) in incoming {
            if let Some(target) = base.get_mut(key) {
                merge_valid(target, value, &format!("{field}.{key}"));
            }
        }
        return;
    }
    let same = matches!(
        (&*default, raw),
        (serde_json::Value::Bool(_), serde_json::Value::Bool(_))
            | (serde_json::Value::String(_), serde_json::Value::String(_))
            | (serde_json::Value::Number(_), serde_json::Value::Number(_))
            | (serde_json::Value::Array(_), serde_json::Value::Array(_))
            | (serde_json::Value::Null, _)
    );
    if same {
        *default = raw.clone();
    } else {
        crate::diagnostics::log(None, "settings", "field_recovered", field, None);
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn malformed_fields_do_not_destroy_good_settings() {
        let mut v = serde_json::to_value(crate::model::Config::default()).unwrap();
        merge_valid(
            &mut v,
            &serde_json::json!({"wallhaven":false,"policy":{"safe":"invalid","adult":false,"minimum_width":-4,"general_topics":["g000",12]},"favorites":[{"bad":true}],"categories":[false,"bad",true],"download_directory":"/tmp/good","behavior":{"search_history":["one",false]}}),
            "",
        );
        let c: crate::model::Config = serde_json::from_value(v).unwrap();
        assert!(!c.wallhaven);
        assert!(c.policy.safe);
        assert_eq!(c.policy.minimum_width, 0);
        assert_eq!(c.categories, [false, true, true]);
        assert_eq!(c.download_directory, "/tmp/good");
        assert_eq!(c.policy.general_topics, vec!["g000"]);
        assert!(c.favorites.is_empty());
    }
}
