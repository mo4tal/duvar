use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
    time::Instant,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub at: u64,
    pub job: Option<u64>,
    pub component: String,
    pub event: String,
    pub duration_ms: Option<u64>,
    pub detail: String,
    #[serde(default)]
    pub code: Option<crate::errors::Code>,
}
fn ring() -> &'static Mutex<VecDeque<Record>> {
    static RING: OnceLock<Mutex<VecDeque<Record>>> = OnceLock::new();
    RING.get_or_init(|| Mutex::new(VecDeque::new()))
}
pub fn log(job: Option<u64>, component: &str, event: &str, detail: &str, duration_ms: Option<u64>) {
    append(job, component, event, detail, duration_ms, None);
}
fn append(
    job: Option<u64>,
    component: &str,
    event: &str,
    detail: &str,
    duration_ms: Option<u64>,
    code: Option<crate::errors::Code>,
) {
    let mut ring = ring().lock().unwrap_or_else(|e| e.into_inner());
    if ring.len() >= 256 {
        ring.pop_front();
    }
    ring.push_back(Record {
        at: crate::runtime::now(),
        job,
        component: component.into(),
        event: event.into(),
        duration_ms,
        detail: crate::errors::redact(detail, true),
        code,
    });
}
pub fn records() -> Vec<Record> {
    ring()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .cloned()
        .collect()
}
pub struct Span {
    job: Option<u64>,
    component: String,
    start: Instant,
}
impl Span {
    pub fn new(job: Option<u64>, component: &str) -> Self {
        log(job, component, "start", "", None);
        Self {
            job,
            component: component.into(),
            start: Instant::now(),
        }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        log(
            self.job,
            &self.component,
            "finish",
            "",
            Some(self.start.elapsed().as_millis() as u64),
        );
    }
}
pub fn export(path: &std::path::Path) -> anyhow::Result<()> {
    let report = serde_json::json!({"schema":1,"version":env!("CARGO_PKG_VERSION"),"events":records(),"privacy":"No image contents, credentials, sensitive titles or local file paths are exported"});
    crate::store::atomic(path, &serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

pub fn failure(job: Option<u64>, component: &str, error: &crate::errors::Failure) {
    append(
        job,
        component,
        "error",
        &error.message,
        None,
        Some(error.code),
    );
}
