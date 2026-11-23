//! File ownership and recoverable deletion. Never removes a user's arbitrary local photo.
use crate::{model::DownloadedImage, runtime::Facts};
use anyhow::{Context, Result, bail};
use gio::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
pub const MAX_FILE: u64 = 64 * 1024 * 1024;
pub fn regular(path: &Path) -> Result<fs::File> {
    if !path.is_absolute() {
        bail!("Dosya yolu mutlak olmalı");
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let m = file.metadata()?;
    if !m.is_file() {
        bail!("Yalnızca normal görsel dosyaları kullanılabilir");
    }
    if m.len() == 0 {
        bail!("Görsel dosyası boş");
    }
    if m.len() > MAX_FILE {
        bail!("Dosya boyut sınırını aşıyor");
    }
    Ok(file)
}
pub fn hash(path: &Path) -> Result<String> {
    let mut file = regular(path)?;
    let mut digest = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        digest.update(&buf[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
pub fn facts(path: &Path, verify: bool) -> Result<Facts> {
    use image::ImageDecoder;
    let file = regular(path)?;
    let bytes = file.metadata()?.len();
    let mut reader =
        image::ImageReader::new(std::io::BufReader::new(file)).with_guessed_format()?;
    let format = reader.format().context("Görsel biçimi tanınmadı")?;
    if !matches!(
        format,
        image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP
    ) {
        bail!("Görsel biçimi desteklenmiyor");
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let (mut w, mut h) = decoder.dimensions();
    if w == 0
        || h == 0
        || w as u64 * h as u64 > 64 * 1024 * 1024
        || decoder.total_bytes() > 256 * 1024 * 1024
    {
        bail!("Görsel boyut sınırını aşıyor");
    }
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    if matches!(
        orientation,
        image::metadata::Orientation::Rotate90
            | image::metadata::Orientation::Rotate270
            | image::metadata::Orientation::Rotate90FlipH
            | image::metadata::Orientation::Rotate270FlipH
    ) {
        std::mem::swap(&mut w, &mut h);
    }
    let alpha = decoder.color_type().has_alpha();
    if verify {
        let _permit = crate::store::decode_permit()?;
        let _image = image::DynamicImage::from_decoder(decoder)?;
    }
    let mut f = Facts::from_size(w, h);
    f.bytes = bytes;
    f.format = format.extensions_str()[0].to_string();
    if f.format == "jpeg" {
        f.format = "jpg".into();
    }
    f.alpha = alpha;
    f.verified = verify;
    f.dimension_origin = "file".into();
    f.sha256 = hash(path)?;
    f.learned_at = crate::runtime::now();
    Ok(f)
}
pub fn managed_name(record: &DownloadedImage) -> bool {
    let name = Path::new(&record.path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let expected = format!("duvar-{}-", &crate::store::key(&record.item.id)[..12]);
    name.starts_with(&expected)
        && !record.facts.sha256.is_empty()
        && name.contains(&record.facts.sha256[..12.min(record.facts.sha256.len())])
}
pub fn owns(record: &DownloadedImage) -> Result<()> {
    let path = Path::new(&record.path);
    if record.schema_version > 2 {
        bail!("Bu makbuz daha yeni sürümde oluşturulmuş");
    }
    if !managed_name(record) {
        bail!("Bu dosyanın Duvar sahipliği doğrulanamadı");
    }
    let file = regular(path)?;
    let meta = file.metadata()?;
    if meta.len() != record.facts.bytes {
        bail!("Dosya indirmeden sonra değiştirilmiş; çöp işlemi durduruldu");
    }
    if hash(path)? != record.facts.sha256 {
        bail!("Dosyanın içerik parmak izi değişmiş");
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrashTicket {
    pub id: u64,
    pub record: DownloadedImage,
    pub trash_path: String,
    pub info_path: String,
    pub at: u64,
}
fn trash_roots(path: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut roots = vec![(glib::user_data_dir().join("Trash"), PathBuf::from("/"))];
    if let Ok(m) =
        fs::metadata(path).or_else(|_| fs::metadata(path.parent().unwrap_or(Path::new("/"))))
    {
        let mut top = path.parent().unwrap_or(Path::new("/")).to_path_buf();
        while let Some(parent) = top.parent() {
            if fs::metadata(parent).is_ok_and(|p| p.dev() == m.dev()) {
                top = parent.to_path_buf();
            } else {
                break;
            }
        }
        let uid = unsafe { libc::getuid() };
        roots.push((top.join(format!(".Trash-{uid}")), top.clone()));
        roots.push((top.join(".Trash").join(uid.to_string()), top));
    }
    roots
}
fn original_from_info(info: &str, top: &Path) -> Option<PathBuf> {
    let raw = info.lines().find_map(|line| line.strip_prefix("Path="))?;
    let url = if raw.starts_with('/') {
        format!("file://{raw}")
    } else {
        format!("file://{}/{}", top.display(), raw)
    };
    url::Url::parse(&url).ok()?.to_file_path().ok()
}
pub fn trash(record: &DownloadedImage) -> Result<TrashTicket> {
    owns(record)?;
    let path = Path::new(&record.path);
    let original = path.canonicalize()?;
    let before = fs::metadata(path)?;
    gio::File::for_path(path).trash(None::<&gio::Cancellable>)?;
    for (root, top) in trash_roots(&original) {
        let info_dir = root.join("info");
        if !info_dir.is_dir() {
            continue;
        }
        let mut entries = fs::read_dir(&info_dir)?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        entries
            .sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));
        for entry in entries.into_iter().take(5000) {
            let meta = entry.metadata()?;
            if !meta.is_file() || meta.len() > 8192 {
                continue;
            }
            let text = fs::read_to_string(entry.path())?;
            if original_from_info(&text, &top).as_deref() != Some(original.as_path()) {
                continue;
            }
            let name = entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(".trashinfo")
                .map(str::to_owned);
            let Some(name) = name else {
                continue;
            };
            let dest = root.join("files").join(name);
            if !dest.is_file() {
                continue;
            }
            let after = fs::metadata(&dest)?;
            if after.len() != before.len()
                || after.ino() != before.ino()
                || after.dev() != before.dev()
                || hash(&dest)? != record.facts.sha256
            {
                continue;
            }
            let ticket = TrashTicket {
                id: crate::queue::next_id(),
                record: record.clone(),
                trash_path: dest.to_string_lossy().into(),
                info_path: entry.path().to_string_lossy().into(),
                at: crate::runtime::now(),
            };
            crate::store::atomic(
                &crate::store::state_dir().join(format!("trash-{}.json", ticket.id)),
                &serde_json::to_vec(&ticket)?,
            )?;
            return Ok(ticket);
        }
    }
    bail!(
        "Dosya çöp kutusuna taşındı; geri alma kaydı bulunamadı. Dosya yöneticisinin Çöp Kutusu bölümünden geri yükleyebilirsin."
    )
}
pub fn restore(ticket: &TrashTicket) -> Result<DownloadedImage> {
    let target = Path::new(&ticket.record.path);
    if target.exists() || fs::symlink_metadata(target).is_ok() {
        bail!("Özgün konumda başka dosya var; üzerine yazılmayacak");
    }
    let trash = Path::new(&ticket.trash_path);
    if hash(trash)? != ticket.record.facts.sha256 {
        bail!("Çöp kutusundaki dosyanın parmak izi değişmiş");
    }
    let info = fs::read_to_string(&ticket.info_path)?;
    let allowed = trash_roots(target).iter().any(|(root, top)| {
        trash.starts_with(root.join("files"))
            && Path::new(&ticket.info_path).starts_with(root.join("info"))
            && original_from_info(&info, top).as_deref() == Some(target)
    });
    if !allowed {
        bail!("Geri alma kaydı doğrulanamadı");
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    gio::File::for_path(trash).move_(
        &gio::File::for_path(target),
        gio::FileCopyFlags::ALL_METADATA,
        None::<&gio::Cancellable>,
        None,
    )?;
    fs::remove_file(&ticket.info_path)?;
    crate::store::receipt_with_facts(&ticket.record.item, target, ticket.record.facts.clone())?;
    let _ = fs::remove_file(crate::store::state_dir().join(format!("trash-{}.json", ticket.id)));
    Ok(ticket.record.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(root: &Path) -> DownloadedImage {
        let item:crate::model::Wallpaper=serde_json::from_value(serde_json::json!({"id":"doctor:ownership","provider":"Fixture","title":"test","image":"https://example.invalid/test.png","thumbnail":"https://example.invalid/test.png","landing":"","rating":"safe","tags":[],"width":8,"height":5,"author":"","license":"CC0","local":false})).unwrap();
        let first = root.join("image.png");
        image::RgbaImage::from_pixel(8, 5, image::Rgba([20, 40, 60, 128]))
            .save(&first)
            .unwrap();
        let facts = facts(&first, true).unwrap();
        let name = format!(
            "duvar-{}-{}.png",
            &crate::store::key(&item.id)[..12],
            &facts.sha256[..12]
        );
        let path = root.join(name);
        fs::rename(first, &path).unwrap();
        DownloadedImage {
            item,
            path: path.to_string_lossy().into(),
            schema_version: 2,
            facts,
            downloaded_at: crate::runtime::now(),
        }
    }
    #[test]
    fn facts_come_from_pixels_not_name() {
        let t = tempfile::tempdir().unwrap();
        let r = fixture(t.path());
        assert_eq!((r.facts.width, r.facts.height), (8, 5));
        assert!(r.facts.alpha && r.facts.verified);
        assert_eq!(r.facts.format, "png");
        assert_eq!(r.facts.ratio(), "8:5");
        assert!(owns(&r).is_ok());
    }
    #[test]
    fn ownership_rejects_changes_and_symlinks() {
        let t = tempfile::tempdir().unwrap();
        let r = fixture(t.path());
        let bytes = fs::read(&r.path).unwrap();
        let link = t.path().join("link.png");
        std::os::unix::fs::symlink(&r.path, &link).unwrap();
        assert!(regular(&link).is_err());
        let mut bad = bytes.clone();
        bad[25] ^= 1;
        fs::write(&r.path, bad).unwrap();
        assert!(owns(&r).is_err());
        fs::write(&r.path, bytes).unwrap();
        assert!(owns(&r).is_ok());
        let mut arbitrary = r.clone();
        arbitrary.path = t.path().join("personal.png").to_string_lossy().into();
        fs::copy(&r.path, &arbitrary.path).unwrap();
        assert!(owns(&arbitrary).is_err());
    }
    #[test]
    fn corrupt_body_does_not_become_verified() {
        let t = tempfile::tempdir().unwrap();
        let r = fixture(t.path());
        let data = fs::read(&r.path).unwrap();
        fs::write(&r.path, &data[..33]).unwrap();
        assert!(facts(Path::new(&r.path), true).is_err());
    }
    #[test]
    fn trash_restore_preserves_bytes_and_refuses_overwrite() {
        let t = tempfile::tempdir().unwrap();
        let r = fixture(t.path());
        let ticket = match trash(&r) {
            Ok(t) => t,
            Err(_) => {
                assert!(
                    Path::new(&r.path).exists(),
                    "Failed trash must retain original"
                );
                let id = crate::queue::next_id();
                let root = glib::user_data_dir().join("Trash");
                fs::create_dir_all(root.join("files")).unwrap();
                fs::create_dir_all(root.join("info")).unwrap();
                let destination = root.join("files").join(format!("fixture-{id}.png"));
                let info = root
                    .join("info")
                    .join(format!("fixture-{id}.png.trashinfo"));
                fs::rename(&r.path, &destination).unwrap();
                let uri = url::Url::from_file_path(&r.path).unwrap();
                fs::write(
                    &info,
                    format!(
                        "[Trash Info]\nPath={}\nDeletionDate=2026-11-15T00:00:00\n",
                        uri.path()
                    ),
                )
                .unwrap();
                TrashTicket {
                    id,
                    record: r.clone(),
                    trash_path: destination.to_string_lossy().into(),
                    info_path: info.to_string_lossy().into(),
                    at: crate::runtime::now(),
                }
            }
        };
        assert!(!Path::new(&r.path).exists());
        fs::write(&r.path, b"unrelated file").unwrap();
        assert!(restore(&ticket).is_err());
        assert_eq!(fs::read(&r.path).unwrap(), b"unrelated file");
        fs::remove_file(&r.path).unwrap();
        let restored = restore(&ticket).unwrap();
        assert_eq!(hash(Path::new(&restored.path)).unwrap(), r.facts.sha256);
        assert!(owns(&restored).is_ok());
    }
}
