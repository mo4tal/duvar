use crate::{
    model::{Config, DownloadedImage, Wallpaper},
    runtime::{Actions, Facts, Phase},
};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};
#[derive(Clone, Debug, Default)]
pub struct Index {
    pub entries: Vec<DownloadedImage>,
    by_id: BTreeMap<String, Vec<usize>>,
    snapshot: BTreeMap<String, Option<u64>>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub files: usize,
    pub missing: usize,
    pub bytes: u64,
    pub bad_records: usize,
}
impl Index {
    pub fn new(entries: Vec<DownloadedImage>) -> Self {
        let snapshot = entries
            .iter()
            .map(|e| {
                (
                    e.path.clone(),
                    std::fs::symlink_metadata(&e.path)
                        .ok()
                        .filter(|m| m.is_file())
                        .map(|m| m.len()),
                )
            })
            .collect();
        Self::from_snapshot(entries, snapshot)
    }
    fn from_snapshot(
        mut entries: Vec<DownloadedImage>,
        snapshot: BTreeMap<String, Option<u64>>,
    ) -> Self {
        entries.sort_by_key(|e| std::cmp::Reverse(e.downloaded_at));
        let mut by_id: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, e) in entries.iter().enumerate() {
            by_id.entry(e.item.id.clone()).or_default().push(i);
        }
        Self {
            entries,
            by_id,
            snapshot,
        }
    }
    pub fn resolve(&self, item: &Wallpaper) -> Option<&DownloadedImage> {
        let ids = self.by_id.get(&item.id)?;
        if item.local
            && let Ok(url) = url::Url::parse(&item.image)
            && let Ok(path) = url.to_file_path()
            && let Some(e) = ids
                .iter()
                .map(|i| &self.entries[*i])
                .find(|e| Path::new(&e.path) == path && Path::new(&e.path).is_file())
        {
            return Some(e);
        }
        ids.iter()
            .map(|i| &self.entries[*i])
            .find(|e| Path::new(&e.path).is_file())
    }
    pub fn known(&self, id: &str) -> Option<&DownloadedImage> {
        self.by_id
            .get(id)
            .and_then(|v| v.first())
            .map(|i| &self.entries[*i])
    }
    pub fn localize(&self, item: &Wallpaper) -> Wallpaper {
        let Some(record) = self.resolve(item) else {
            return item.clone();
        };
        let mut item = item.clone();
        if let Ok(uri) = url::Url::from_file_path(&record.path) {
            item.local = true;
            item.image = uri.to_string();
            item.thumbnail = item.image.clone();
            item.preview_url = item.image.clone();
            if record.facts.width > 0 {
                item.width = record.facts.width;
                item.height = record.facts.height;
            }
        }
        item
    }
    pub fn summary(&self) -> Summary {
        let mut s = Summary::default();
        for bytes in self.snapshot.values() {
            match bytes {
                Some(bytes) => {
                    s.files += 1;
                    s.bytes += bytes;
                }
                None => s.missing += 1,
            }
        }
        s.bad_records = crate::store::bad_receipts();
        s
    }
    pub fn filtered(&self, config: &Config) -> Vec<Wallpaper> {
        let b = &config.behavior;
        let query = crate::catalog::search_key(&b.library_query);
        let mut entries = self
            .entries
            .iter()
            .filter(|e| {
                let exists = self.snapshot.get(&e.path).is_some_and(|v| v.is_some());
                if b.library_missing == exists {
                    return false;
                }
                if !config.policy.accepts(&e.item) {
                    return false;
                }
                if !b.library_source.is_empty() && !e.item.provider.starts_with(&b.library_source) {
                    return false;
                }
                if !b.library_format.is_empty() && e.facts.format != b.library_format {
                    return false;
                }
                if b.library_orientation != "any"
                    && format!("{:?}", e.facts.orientation).to_lowercase() != b.library_orientation
                {
                    return false;
                }
                query.is_empty()
                    || crate::catalog::search_key(&format!(
                        "{} {} {}",
                        e.item.title, e.item.author, e.item.provider
                    ))
                    .contains(&query)
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| match config.behavior.library_sort.as_str() {
            "size" => b.facts.bytes.cmp(&a.facts.bytes),
            "resolution" => (b.facts.width as u64 * b.facts.height as u64)
                .cmp(&(a.facts.width as u64 * a.facts.height as u64)),
            "title" => a.item.title.cmp(&b.item.title),
            _ => b.downloaded_at.cmp(&a.downloaded_at),
        });
        entries
            .into_iter()
            .map(|e| {
                let mut item = e.item.clone();
                if self.snapshot.get(&e.path).is_some_and(|v| v.is_some())
                    && let Ok(uri) = url::Url::from_file_path(&e.path)
                {
                    item.local = true;
                    item.image = uri.to_string();
                    item.thumbnail = item.image.clone();
                    item.preview_url = item.image.clone();
                }
                item
            })
            .collect()
    }
    pub fn facts(&self, item: &Wallpaper) -> Facts {
        self.resolve(item)
            .or_else(|| self.known(&item.id))
            .map(|e| e.facts.clone())
            .unwrap_or_else(|| Facts::from_size(item.width, item.height))
    }
    pub fn actions(&self, item: &Wallpaper, phase: Phase) -> Actions {
        let record = self.resolve(item);
        let exists = record.is_some()
            || (item.local
                && url::Url::parse(&item.image)
                    .ok()
                    .and_then(|u| u.to_file_path().ok())
                    .is_some_and(|p| p.is_file()));
        Actions::for_state(
            if exists && !phase.pending() {
                Phase::Completed
            } else {
                phase
            },
            record.is_some_and(crate::files::managed_name),
            exists,
        )
    }
    pub fn remove_path(&mut self, path: &str) {
        let entries = self
            .entries
            .iter()
            .filter(|e| e.path != path)
            .cloned()
            .collect();
        self.snapshot.remove(path);
        let snapshot = std::mem::take(&mut self.snapshot);
        *self = Self::from_snapshot(entries, snapshot);
    }
}
pub fn load() -> anyhow::Result<Index> {
    let mut records = crate::store::downloaded()?;
    for record in &mut records {
        if record.schema_version == 0
            && Path::new(&record.path).is_file()
            && let Ok(facts) = crate::files::facts(Path::new(&record.path), false)
        {
            record.facts = facts;
            record.item.width = record.facts.width;
            record.item.height = record.facts.height;
            record.schema_version = 2;
            crate::store::receipt_with_facts(
                &record.item,
                Path::new(&record.path),
                record.facts.clone(),
            )?;
        }
    }
    Ok(Index::new(records))
}
