use super::*;
use crate::{
    errors::{Code, Failure},
    runtime::{Facts, Meter, Phase},
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
pub type Sink<'a> = dyn FnMut(Phase, u64, Option<u64>, Option<f64>, Option<u64>) + 'a;
pub struct RawFile {
    pub lease: crate::cache::Lease,
    pub facts: Facts,
    _temporary: Option<tempfile::NamedTempFile>,
}
pub(super) struct Asset<'a> {
    network: &'a Network,
    key: String,
}
impl Drop for Asset<'_> {
    fn drop(&mut self) {
        self.network
            .assets
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.key);
        self.network.asset_ready.notify_all();
    }
}
impl Network {
    pub(super) fn asset<'a>(&'a self, key: &str, cancel: &AtomicBool) -> Result<Asset<'a>> {
        let mut set = self.assets.lock().unwrap_or_else(|e| e.into_inner());
        while set.contains(key) {
            check(cancel)?;
            set = self
                .asset_ready
                .wait_timeout(set, Duration::from_millis(50))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        set.insert(key.into());
        Ok(Asset {
            network: self,
            key: key.into(),
        })
    }
    pub fn original(
        &self,
        item: &Wallpaper,
        cancel: &AtomicBool,
        sink: &mut Sink<'_>,
    ) -> Result<RawFile> {
        let _span = crate::diagnostics::Span::new(None, "original");
        if item.local {
            let path = url::Url::parse(&item.image)?
                .to_file_path()
                .map_err(|_| Failure::new(Code::InvalidPath, "Dosya yolu geçersiz"))?;
            let lease = crate::cache::Lease::new(path.clone());
            let facts = crate::files::facts(&path, true)?;
            return Ok(RawFile {
                lease,
                facts,
                _temporary: None,
            });
        }
        valid_url(&item.image)?;
        sink(Phase::Waiting, 0, None, None, None);
        let _asset = self.asset(&item.image, cancel)?;
        let cache_ready = crate::store::prepare_cache().is_ok();
        let path =
            crate::store::cache().join(format!("{}.original", crate::store::key(&item.image)));
        let lease = crate::cache::Lease::new(path.clone());
        if path.is_file() {
            if let Ok(facts) = cached_facts(&path) {
                crate::cache::touch(&path);
                sink(Phase::Saving, facts.bytes, Some(facts.bytes), None, None);
                return Ok(RawFile {
                    lease,
                    facts,
                    _temporary: None,
                });
            }
            crate::diagnostics::log(
                None,
                "cache",
                "corrupt_original",
                "Yeniden indiriliyor",
                None,
            );
            let _ = fs::remove_file(&path);
            let _ = fs::remove_file(path.with_extension("info"));
        }
        let mut response = self.request(&item.image, 120, cancel)?;
        let total = response
            .headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());
        let mut file = if cache_ready {
            tempfile::Builder::new()
                .prefix(".duvar-transfer-")
                .tempfile_in(crate::store::cache())
                .or_else(|_| {
                    tempfile::Builder::new()
                        .prefix(".duvar-transfer-")
                        .tempfile()
                })?
        } else {
            tempfile::Builder::new()
                .prefix(".duvar-transfer-")
                .tempfile()?
        };
        sink(Phase::Downloading, 0, total, None, None);
        let (_, bytes) = pump(
            &mut response.body_mut().as_reader(),
            file.as_file_mut(),
            total,
            crate::files::MAX_FILE,
            cancel,
            sink,
        )?;
        check(cancel)?;
        sink(Phase::Validating, bytes, Some(bytes), None, None);
        file.as_file().sync_all()?;
        let mut facts = crate::files::facts(file.path(), true)?;
        facts.learned_at = crate::runtime::now();
        check(cancel)?;
        if !cache_ready {
            let lease = crate::cache::Lease::new(file.path().to_path_buf());
            return Ok(RawFile {
                lease,
                facts,
                _temporary: Some(file),
            });
        }
        if let Err(error) = file.persist(&path) {
            let file = error.file;
            let lease = crate::cache::Lease::new(file.path().to_path_buf());
            crate::diagnostics::log(
                None,
                "cache",
                "temporary_fallback",
                "Özgün dosya geçici alandan kullanılıyor",
                None,
            );
            return Ok(RawFile {
                lease,
                facts,
                _temporary: Some(file),
            });
        }

        if let Err(e) =
            crate::store::atomic(&path.with_extension("info"), &serde_json::to_vec(&facts)?)
        {
            crate::diagnostics::log(None, "cache", "metadata_write", &e.to_string(), None);
        }
        crate::cache::mark(&path, item.rating);
        crate::cache::mark(&path.with_extension("info"), item.rating);
        if let Err(e) = crate::cache::trim(256 * 1024 * 1024, false) {
            crate::diagnostics::log(None, "cache", "trim", &e.to_string(), None);
        }
        Ok(RawFile {
            lease,
            facts,
            _temporary: None,
        })
    }
    pub fn download_progress(
        &self,
        item: &Wallpaper,
        directory: &Path,
        cancel: &AtomicBool,
        sink: &mut Sink<'_>,
    ) -> Result<(PathBuf, Facts)> {
        let _span = crate::diagnostics::Span::new(None, "download");
        let raw = self.original(item, cancel, sink)?;
        check(cancel)?;
        sink(
            Phase::Saving,
            raw.facts.bytes,
            Some(raw.facts.bytes),
            None,
            None,
        );
        if !directory.is_absolute() {
            return Err(
                Failure::new(Code::InvalidPath, "İndirme klasörü mutlak yerel yol olmalı").into(),
            );
        }
        let _disk = crate::diagnostics::Span::new(None, "disk_save");
        fs::create_dir_all(directory)?;
        let ext = &raw.facts.format;
        let path = directory.join(format!(
            "duvar-{}-{}.{}",
            &crate::store::key(&item.id)[..12],
            &raw.facts.sha256[..12],
            ext
        ));
        if fs::symlink_metadata(&path).is_ok() {
            if crate::files::hash(&path)? != raw.facts.sha256 {
                return Err(Failure::new(
                    Code::Conflict,
                    "Aynı isimli dosya farklı; üzerine yazılmayacak",
                )
                .into());
            }
        } else {
            let mut out = tempfile::Builder::new()
                .prefix(".duvar-save-")
                .tempfile_in(directory)?;
            let mut input = crate::files::regular(&raw.lease.path)?;
            let mut buffer = [0u8; 65536];
            loop {
                check(cancel)?;
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                out.write_all(&buffer[..n])?;
            }
            out.as_file().sync_all()?;
            check(cancel)?;
            out.persist_noclobber(&path).map_err(|e| e.error)?;
        }
        crate::store::receipt_with_facts(item, &path, raw.facts.clone())?;
        sink(
            Phase::Completed,
            raw.facts.bytes,
            Some(raw.facts.bytes),
            None,
            Some(0),
        );
        Ok((path, raw.facts))
    }
}
fn cached_facts(path: &Path) -> Result<Facts> {
    let info = path.with_extension("info");
    if fs::metadata(&info).is_ok_and(|m| m.len() <= 65536) {
        let facts: Facts = serde_json::from_slice(&fs::read(&info)?)?;
        if facts.verified
            && crate::files::regular(path)?.metadata()?.len() == facts.bytes
            && crate::files::hash(path)? == facts.sha256
        {
            return Ok(facts);
        }
    }
    let facts = crate::files::facts(path, true)?;
    let _ = crate::store::atomic(&info, &serde_json::to_vec(&facts)?);
    Ok(facts)
}
pub fn pump<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    total: Option<u64>,
    limit: u64,
    cancel: &AtomicBool,
    sink: &mut Sink<'_>,
) -> Result<(String, u64)> {
    if total.is_some_and(|n| n > limit) {
        return Err(Failure::new(
            Code::TooLarge,
            "Sunucunun bildirdiği dosya boyut sınırını aşıyor",
        )
        .into());
    }
    let mut bytes = 0u64;
    let mut hash = Sha256::new();
    let mut meter = Meter::new();
    let mut buffer = [0u8; 65536];
    loop {
        check(cancel)?;
        let count = reader.read(&mut buffer).map_err(|_| {
            Failure::new(
                Code::PartialBody,
                "Aktarım kesildi; eksik dosya kaydedilmedi",
            )
        })?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(count as u64)
            .ok_or_else(|| Failure::new(Code::TooLarge, "Boyut taşması"))?;
        if bytes > limit {
            return Err(Failure::new(Code::TooLarge, "Aktarım boyut sınırını aşıyor").into());
        }
        writer.write_all(&buffer[..count]).map_err(|e| {
            if e.raw_os_error() == Some(libc::ENOSPC) {
                Failure::new(Code::DiskFull, "Diskte boş alan yok")
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                Failure::new(Code::Permission, "Dosya yazma izni yok")
            } else {
                Failure::new(Code::Network, "Dosya yazılamadı")
            }
        })?;
        hash.update(&buffer[..count]);
        if let Some((speed, eta)) = meter.update(bytes, total, false) {
            sink(Phase::Downloading, bytes, total, speed, eta);
        }
    }
    if bytes == 0 {
        return Err(Failure::new(Code::EmptyBody, "Sunucu boş görsel döndürdü").into());
    }
    if total.is_some_and(|n| n != bytes) {
        return Err(Failure::new(
            Code::PartialBody,
            "Aktarım boyutu Content-Length ile uyuşmuyor",
        )
        .into());
    }
    check(cancel)?;
    if let Some((speed, eta)) = meter.update(bytes, total, true) {
        sink(Phase::Downloading, bytes, total, speed, eta);
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_and_oversized_bodies_never_complete() {
        let c = AtomicBool::new(false);
        let mut sink = |_, _, _, _, _| {};
        assert!(
            pump(
                &mut &b"123"[..],
                &mut Vec::new(),
                Some(5),
                10,
                &c,
                &mut sink
            )
            .is_err()
        );
        assert!(pump(&mut &b"12345"[..], &mut Vec::new(), None, 4, &c, &mut sink).is_err());
        assert!(pump(&mut &b""[..], &mut Vec::new(), None, 4, &c, &mut sink).is_err());
    }
    #[test]
    fn cancelled_stream_stops_before_writing() {
        let c = AtomicBool::new(true);
        let mut out = Vec::new();
        assert!(
            pump(
                &mut &b"a"[..],
                &mut out,
                None,
                10,
                &c,
                &mut |_, _, _, _, _| {}
            )
            .is_err()
        );
        assert!(out.is_empty());
    }
}
#[cfg(test)]
mod fault_tests {
    use super::*;
    struct Reject;
    impl Write for Reject {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from_raw_os_error(libc::ENOSPC))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn full_disk_is_an_explicit_failure() {
        let cancel = AtomicBool::new(false);
        let e = pump(
            &mut &b"data"[..],
            &mut Reject,
            Some(4),
            10,
            &cancel,
            &mut |_, _, _, _, _| {},
        )
        .unwrap_err();
        assert!(e.to_string().contains("DiskFull"), "{e}");
    }
    #[test]
    fn cancellation_after_a_chunk_is_observed() {
        let cancel = AtomicBool::new(false);
        struct Stop<'a>(&'a AtomicBool);
        impl Read for Stop<'_> {
            fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
                b[..4].copy_from_slice(b"data");
                self.0.store(true, Ordering::Relaxed);
                Ok(4)
            }
        }
        let mut out = Vec::new();
        assert!(
            pump(
                &mut Stop(&cancel),
                &mut out,
                None,
                100,
                &cancel,
                &mut |_, _, _, _, _| {}
            )
            .is_err()
        );
        assert!(out.len() <= 4);
    }
}
