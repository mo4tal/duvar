use crate::model::DownloadRecord;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicU64, Ordering},
};
pub fn next_id() -> u64 {
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    LAST.fetch_max(now, Ordering::SeqCst);
    LAST.fetch_add(1, Ordering::SeqCst)
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ApplyOrder([u64; 2]);

impl ApplyOrder {
    pub fn from_legacy(id: u64) -> Self {
        Self([id; 2])
    }

    pub fn latest(self) -> u64 {
        self.0[0].max(self.0[1])
    }

    pub fn mark(&mut self, scope: &str, id: u64) {
        if matches!(scope, "light" | "both") {
            self.0[0] = self.0[0].max(id);
        }
        if matches!(scope, "dark" | "both") {
            self.0[1] = self.0[1].max(id);
        }
    }

    pub fn pending_scope(self, scope: &str, id: u64) -> Option<&'static str> {
        match (
            matches!(scope, "light" | "both") && self.0[0] == id,
            matches!(scope, "dark" | "both") && self.0[1] == id,
        ) {
            (true, true) => Some("both"),
            (true, false) => Some("light"),
            (false, true) => Some("dark"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Envelope {
    pub schema_version: u32,
    pub latest_apply: u64,
    pub apply_order: ApplyOrder,
    pub paused: bool,
    pub jobs: Vec<DownloadRecord>,
}
pub fn normalize(mut entries: Vec<DownloadRecord>) -> (Vec<DownloadRecord>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut keys = BTreeSet::new();
    entries.retain_mut(|r| {
        if r.id == 0
            || r.item.id.trim().is_empty()
            || !std::path::Path::new(&r.directory).is_absolute()
        {
            warnings.push("Geçersiz iş kaydı atlandı".into());
            return false;
        }
        let key = (r.item.id.clone(), r.directory.clone(), r.scope.clone());
        if matches!(
            r.status.as_str(),
            "queued" | "active" | "downloading" | "waiting" | "validating" | "saving"
        ) && !keys.insert(key)
        {
            warnings.push("Yinelenen bekleyen iş birleştirildi".into());
            return false;
        }
        if matches!(
            r.status.as_str(),
            "active"
                | "downloading"
                | "waiting"
                | "validating"
                | "saving"
                | "resume_requested"
                | "pause_requested"
                | "cancel_requested"
        ) {
            r.status = "queued".into();
            r.error = "Önceki oturumda kesildi; yeniden başlatılacak".into();
            warnings.push("Kesilen iş kurtarıldı".into());
        }
        if !["queued", "paused", "cancelled", "done", "failed"].contains(&r.status.as_str()) {
            r.status = "failed".into();
            r.error = "Bilinmeyen kuyruk durumu güvenli duruma alındı".into();
        }
        if r.created_at == 0 {
            r.created_at = crate::runtime::now();
        }
        r.item.title = r.item.title.chars().take(1024).collect();
        r.error = crate::errors::redact(&r.error, true);
        true
    });
    entries.truncate(200);
    (entries, warnings)
}
pub fn decode(bytes: &[u8]) -> anyhow::Result<(Envelope, Vec<String>)> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    if value["schema_version"].as_u64().is_some_and(|n| n > 2) {
        anyhow::bail!("Kuyruk şeması bu sürümden yeni; kayıt değiştirilmeyecek");
    }
    let list = value
        .as_array()
        .or_else(|| value["jobs"].as_array())
        .ok_or_else(|| anyhow::anyhow!("Kuyruk listesi geçersiz"))?;
    let mut jobs = Vec::new();
    let mut warnings = Vec::new();
    for raw in list.iter().take(500) {
        match serde_json::from_value(raw.clone()) {
            Ok(job) => jobs.push(job),
            Err(_) => warnings.push("Tek bozuk kuyruk kaydı atlandı".into()),
        }
    }
    let (jobs, notes) = normalize(jobs);
    warnings.extend(notes);
    let latest_apply = value["latest_apply"].as_u64().unwrap_or_else(|| {
        jobs.iter()
            .filter(|r| r.scope.is_some())
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
    });
    Ok((
        Envelope {
            schema_version: 2,
            latest_apply,
            apply_order: serde_json::from_value(value["apply_order"].clone())
                .unwrap_or_else(|_| ApplyOrder::from_legacy(latest_apply)),
            paused: value["paused"].as_bool().unwrap_or(false),
            jobs,
        },
        warnings,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wallpaper_intents_only_supersede_the_same_target() {
        let mut order = ApplyOrder::from_legacy(10);
        order.mark("light", 12);
        assert_eq!(order.pending_scope("both", 10), Some("dark"));
        order.mark("dark", 13);
        order.mark("light", 11);
        assert_eq!(order.pending_scope("light", 12), Some("light"));
        assert_eq!(order.pending_scope("dark", 13), Some("dark"));
        assert_eq!(order.pending_scope("light", 11), None);
    }
    #[test]
    fn ids_are_unique_under_contention() {
        let threads = (0..4)
            .map(|_| std::thread::spawn(|| (0..100).map(|_| next_id()).collect::<Vec<_>>()))
            .collect::<Vec<_>>();
        let ids = threads
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 400);
    }
    #[test]
    fn future_schema_is_not_interpreted() {
        assert!(decode(br#"{"schema_version":99,"jobs":[]}"#).is_err());
    }
}
#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn one_bad_job_does_not_erase_good_jobs() {
        let item = serde_json::json!({"id":"fixture","provider":"test","title":"test","image":"https://example.invalid/a.jpg","thumbnail":"https://example.invalid/a.jpg","landing":"","rating":"safe","width":4,"height":3,"author":"","license":""});
        let job = serde_json::json!({"id":11,"item":item,"directory":"/tmp/duvar-test","scope":"both","status":"active","error":"","path":""});
        let bytes=serde_json::to_vec(&serde_json::json!({"schema_version":2,"latest_apply":25,"paused":true,"jobs":[{"broken":true},job.clone(),job]})).unwrap();
        let (e, w) = decode(&bytes).unwrap();
        assert_eq!(e.jobs.len(), 1);
        assert_eq!(e.jobs[0].status, "queued");
        assert!(w.len() >= 3);
        assert_eq!(e.latest_apply, 25);
        assert!(e.paused);
    }
}
