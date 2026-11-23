use super::*;
use crate::runtime::Facts;
impl Network {
    pub fn metadata(&self, item: &Wallpaper, cancel: &AtomicBool) -> Result<Facts> {
        let _span = crate::diagnostics::Span::new(None, "metadata");
        if item.local {
            let path = url::Url::parse(&item.image)?
                .to_file_path()
                .map_err(|_| anyhow::anyhow!("Dosya yolu geçersiz"))?;
            return crate::files::facts(&path, false);
        }
        let _asset = self.asset(&item.image, cancel)?;
        let key = crate::store::key(&item.image);
        let original = crate::store::cache().join(format!("{key}.info"));
        if let Ok(m) = std::fs::metadata(&original)
            && m.len() <= 65536
            && let Ok(facts) = serde_json::from_slice::<Facts>(&std::fs::read(&original)?)
            && facts.verified
            && facts.width > 0
            && facts.height > 0
        {
            return Ok(facts);
        }
        let cache = crate::store::cache().join(format!("{key}.headerinfo"));
        if std::fs::metadata(&cache).is_ok_and(|m| m.len() <= 65536)
            && let Ok(facts) = serde_json::from_slice::<Facts>(&std::fs::read(&cache)?)
            && facts.width > 0
            && facts.height > 0
            && crate::runtime::now().saturating_sub(facts.learned_at) < 3600
        {
            return Ok(facts);
        }
        let mut response = self.request_range(&item.image, 15, cancel)?;
        let bytes = if response.status().as_u16() == 206 {
            response
                .headers()
                .get("content-range")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split('/').next_back())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        } else {
            response
                .headers()
                .get("content-length")
                .and_then(|h| h.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        };
        let mut data = Vec::new();
        let mut reader = response.body_mut().as_reader().take(131072);
        let mut buf = [0u8; 16384];
        loop {
            check(cancel)?;
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
        }
        let (w, h, format) = crate::store::image_info(&data)?;
        let mut facts = Facts::from_size(w, h);
        facts.bytes = bytes;
        facts.format = format.extensions_str()[0].into();
        if facts.format == "jpeg" {
            facts.format = "jpg".into();
        }
        facts.learned_at = crate::runtime::now();
        facts.dimension_origin = "original_header".into();
        if crate::store::prepare_cache().is_ok() {
            if let Err(e) = crate::store::atomic(&cache, &serde_json::to_vec(&facts)?) {
                crate::diagnostics::log(
                    None,
                    "metadata",
                    "cache_write_failed",
                    &e.to_string(),
                    None,
                );
            } else {
                crate::cache::mark(&cache, item.rating);
            }
        }
        Ok(facts)
    }
}
