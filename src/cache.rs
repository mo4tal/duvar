use anyhow::Result;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};
fn leases() -> &'static Mutex<BTreeMap<PathBuf, usize>> {
    static L: OnceLock<Mutex<BTreeMap<PathBuf, usize>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(BTreeMap::new()))
}
pub struct Lease {
    pub path: PathBuf,
}
impl Lease {
    pub fn new(path: PathBuf) -> Self {
        *leases()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(path.clone())
            .or_default() += 1;
        Self { path }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        let mut l = leases().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = l.get_mut(&self.path) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                l.remove(&self.path);
            }
        }
    }
}
pub fn protected(path: &Path) -> bool {
    leases()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains_key(path)
}
fn remove_unleased(path: &Path) -> Result<bool> {
    let active = leases().lock().unwrap_or_else(|e| e.into_inner());
    if active.contains_key(path) {
        return Ok(false);
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
pub fn touch(path: &Path) {
    if let Ok(file) = fs::File::open(path) {
        let _ = file.set_modified(SystemTime::now());
    }
}
static DIRTY: AtomicBool = AtomicBool::new(false);
pub fn dirty() {
    DIRTY.store(true, Ordering::Relaxed);
}
pub fn settle() {
    if DIRTY.swap(false, Ordering::Relaxed) {
        let _ = trim(256 * 1024 * 1024, true);
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Stats {
    pub thumbnails: u64,
    pub originals: u64,
    pub facts: u64,
    pub bytes: u64,
    pub corrupt: u64,
    pub protected: u64,
}
fn marker(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.cachemeta",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}
fn kind(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rgba") => Some("thumbnails"),
        Some("original") => Some("originals"),
        Some("info") | Some("headerinfo") => Some("facts"),
        _ => None,
    }
}
pub fn stats() -> Result<Stats> {
    let mut s = Stats::default();
    if !crate::store::cache().is_dir() {
        return Ok(s);
    }
    for e in fs::read_dir(crate::store::cache())?.filter_map(Result::ok) {
        let path = e.path();
        let Some(kind) = kind(&path) else {
            continue;
        };
        let m = fs::symlink_metadata(&path)?;
        if !m.is_file() {
            continue;
        }
        s.bytes += m.len();
        match kind {
            "thumbnails" => {
                s.thumbnails += 1;
                if s.thumbnails <= 16 && crate::store::read_pixels(&path).is_err() {
                    s.corrupt += 1;
                }
            }
            "originals" => {
                s.originals += 1;
                if m.len() == 0 || m.len() > crate::files::MAX_FILE {
                    s.corrupt += 1;
                }
            }
            _ => s.facts += 1,
        }
        if protected(&path) {
            s.protected += 1;
        }
    }
    Ok(s)
}
pub fn trim(limit: u64, force: bool) -> Result<()> {
    static LAST: OnceLock<Mutex<Instant>> = OnceLock::new();
    let last = LAST.get_or_init(|| Mutex::new(Instant::now() - Duration::from_secs(60)));
    {
        let mut l = last.lock().unwrap_or_else(|e| e.into_inner());
        if !force && l.elapsed() < Duration::from_secs(30) {
            dirty();
            return Ok(());
        }
        *l = Instant::now();
    }
    let root = crate::store::cache();
    if !root.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(&root)?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            kind(&p)?;
            let m = fs::symlink_metadata(&p).ok()?;
            if !m.is_file() {
                return None;
            }
            Some((m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len(), p))
        })
        .collect::<Vec<_>>();
    let mut total: u64 = entries.iter().map(|e| e.1).sum();
    entries.sort_by_key(|e| e.0);
    for (_, len, path) in entries {
        if total <= limit {
            break;
        }
        if protected(&path) {
            continue;
        }
        if remove_unleased(&path)? {
            total = total.saturating_sub(len);
            let _ = fs::remove_file(marker(&path));
        }
    }
    Ok(())
}
pub fn mark(path: &Path, rating: crate::model::Rating) {
    dirty();
    let sensitive = rating != crate::model::Rating::Safe;
    let _ = crate::store::atomic_fast(&marker(path), if sensitive { b"private" } else { b"safe" });
}
pub fn clear(part: &str) -> Result<u64> {
    let mut removed = 0;
    if !crate::store::cache().is_dir() {
        return Ok(0);
    }
    for e in fs::read_dir(crate::store::cache())?.filter_map(Result::ok) {
        let path = e.path();
        let Some(k) = kind(&path) else {
            continue;
        };
        if protected(&path) {
            continue;
        }
        if part == "sensitive" {
            if fs::read(marker(&path)).unwrap_or_default() != b"private" {
                continue;
            }
        } else if part != "all" && part != k {
            continue;
        }
        if fs::symlink_metadata(&path)?.is_file() && remove_unleased(&path)? {
            let _ = fs::remove_file(marker(&path));
            removed += 1;
        }
    }
    Ok(removed)
}
