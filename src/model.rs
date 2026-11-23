use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Safe,
    Sketchy,
    Adult,
    #[default]
    Unknown,
}
impl Rating {
    pub fn label(self) -> &'static str {
        match self {
            Self::Safe => "SFW",
            Self::Sketchy => "Şüpheli",
            Self::Adult => "Yetişkin",
            Self::Unknown => "Etiketsiz",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallpaper {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub thumbnail: String,
    #[serde(default)]
    pub preview_url: String,
    pub image: String,
    pub landing: String,
    pub rating: Rating,
    pub tags: Option<Vec<String>>,
    pub width: u32,
    pub height: u32,
    pub author: String,
    pub license: String,
    #[serde(default)]
    pub local: bool,
    #[serde(default)]
    pub created_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Policy {
    pub safe: bool,
    pub sketchy: bool,
    pub adult: bool,
    pub unknown: bool,
    pub include_enabled: bool,
    pub exclude_enabled: bool,
    pub include_tags: String,
    pub exclude_tags: String,
    pub all_included: bool,
    pub minimum_width: u32,
    pub minimum_height: u32,
    pub general_topics: Vec<String>,
    pub sensitive_topics: Vec<String>,
    pub orientation: String,
    pub formats: Vec<String>,
    pub allow_missing_dimensions: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            safe: true,
            sketchy: false,
            adult: false,
            unknown: false,
            include_enabled: false,
            exclude_enabled: true,
            include_tags: String::new(),
            exclude_tags: String::new(),
            all_included: true,
            minimum_width: 0,
            minimum_height: 0,
            general_topics: Vec::new(),
            sensitive_topics: Vec::new(),
            orientation: "any".into(),
            formats: Vec::new(),
            allow_missing_dimensions: false,
        }
    }
}
fn normalized(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn terms(s: &str) -> BTreeSet<String> {
    s.split(',')
        .map(normalized)
        .filter(|s| !s.is_empty())
        .take(64)
        .collect()
}
impl Policy {
    pub fn rating_allowed(&self, r: Rating) -> bool {
        match r {
            Rating::Safe => self.safe,
            Rating::Sketchy => self.sketchy,
            Rating::Adult => self.adult,
            Rating::Unknown => self.unknown,
        }
    }
    pub fn needs_tags(&self) -> bool {
        (self.include_enabled && !terms(&self.include_tags).is_empty())
            || (self.exclude_enabled && !terms(&self.exclude_tags).is_empty())
            || !self.general_topics.is_empty()
            || !self.sensitive_topics.is_empty()
    }
    pub fn accepts(&self, w: &Wallpaper) -> bool {
        if !self.rating_allowed(w.rating) {
            return false;
        }
        if !(self.allow_missing_dimensions && (w.width == 0 || w.height == 0))
            && self.minimum_width > 0
            && w.width < self.minimum_width
        {
            return false;
        }
        if !(self.allow_missing_dimensions && (w.width == 0 || w.height == 0))
            && self.minimum_height > 0
            && w.height < self.minimum_height
        {
            return false;
        }
        if self.orientation != "any" {
            if w.width == 0 || w.height == 0 {
                if !self.allow_missing_dimensions {
                    return false;
                }
            } else if (self.orientation == "landscape" && w.width <= w.height)
                || (self.orientation == "portrait" && w.width >= w.height)
                || (self.orientation == "square" && w.width != w.height)
            {
                return false;
            }
        }
        if !self.formats.is_empty() {
            let ext = url::Url::parse(&w.image)
                .ok()
                .and_then(|u| {
                    std::path::Path::new(u.path())
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                })
                .unwrap_or_default();
            let ext = if ext == "jpeg" { "jpg" } else { &ext };
            if !self.formats.iter().any(|f| f == ext) {
                return false;
            }
        }
        if ((self.include_enabled && !terms(&self.include_tags).is_empty())
            || (self.exclude_enabled && !terms(&self.exclude_tags).is_empty()))
            && w.tags.is_none()
        {
            return false;
        }
        if !crate::catalog::accepts(&self.general_topics, &self.sensitive_topics, w) {
            return false;
        }
        let tags = w
            .tags
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|s| normalized(s))
            .collect::<BTreeSet<_>>();
        let excluded = terms(&self.exclude_tags);
        if self.exclude_enabled && excluded.iter().any(|s| tags.contains(s)) {
            return false;
        }
        let included = terms(&self.include_tags);
        if self.include_enabled && !included.is_empty() {
            return if self.all_included {
                included.iter().all(|s| tags.contains(s))
            } else {
                included.iter().any(|s| tags.contains(s))
            };
        }
        true
    }
    pub fn wallhaven_purity(&self) -> String {
        [self.safe, self.sketchy, self.adult]
            .map(|x| if x { '1' } else { '0' })
            .iter()
            .collect()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub behavior: Behavior,
    pub policy: Policy,
    pub remember_sensitive: bool,
    pub wallhaven: bool,
    pub openverse: bool,
    pub nasa: bool,
    pub bing: bool,
    pub commons: bool,
    pub yandere: bool,
    pub aibooru: bool,
    pub nekos: bool,
    pub danbooru: bool,
    pub konachan: bool,
    pub gelbooru: bool,
    pub vam: bool,
    pub cleveland: bool,
    pub met: bool,
    pub gallery_order: String,
    pub local: bool,
    pub local_directory: String,
    pub download_directory: String,
    pub query: String,
    pub sort: String,
    pub categories: [bool; 3],
    pub favorites: Vec<Wallpaper>,
    pub cache_mib: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 2,
            behavior: Behavior::default(),
            policy: Policy::default(),
            remember_sensitive: false,
            wallhaven: true,
            openverse: true,
            nasa: false,
            bing: false,
            commons: false,
            yandere: false,
            aibooru: false,
            nekos: false,
            danbooru: false,
            konachan: false,
            gelbooru: false,
            vam: false,
            cleveland: false,
            met: false,
            gallery_order: "source".into(),
            local: false,
            local_directory: String::new(),
            download_directory: String::new(),
            query: String::new(),
            sort: "toplist".into(),
            categories: [true, true, true],
            favorites: Vec::new(),
            cache_mib: 256,
        }
    }
}
impl Config {
    pub fn persistent(&self) -> Self {
        let mut result = self.clone();
        if !result.remember_sensitive {
            result.policy.safe = true;
            result.policy.sketchy = false;
            result.policy.adult = false;
            result.policy.unknown = false;
        }
        result
    }
    pub fn validate(&mut self) {
        for path in [&mut self.local_directory, &mut self.download_directory] {
            if !path.is_empty()
                && (!std::path::Path::new(path.as_str()).is_absolute() || path.contains('\0'))
            {
                path.clear();
            }
        }
        let mut ids = BTreeSet::new();
        self.favorites
            .retain(|f| !f.id.is_empty() && ids.insert(f.id.clone()));
        self.query = self.query.trim().chars().take(256).collect();
        self.behavior.validate();
        self.policy
            .formats
            .retain(|f| ["jpg", "png", "webp"].contains(&f.as_str()));
        self.policy.formats.sort();
        self.policy.formats.dedup();
        if !["any", "landscape", "portrait", "square"].contains(&self.policy.orientation.as_str()) {
            self.policy.orientation = "any".into();
        }
        self.policy.include_tags = self.policy.include_tags.chars().take(512).collect();
        self.policy.exclude_tags = self.policy.exclude_tags.chars().take(512).collect();
        self.cache_mib = self.cache_mib.clamp(32, 1024);
        self.policy.minimum_width = self.policy.minimum_width.min(16384);
        self.policy.minimum_height = self.policy.minimum_height.min(16384);
        self.favorites.truncate(128);
        self.danbooru = false;
        self.konachan = false;
        self.gelbooru = false;
        self.policy
            .general_topics
            .retain(|id| crate::catalog::all(false).iter().any(|t| t.id == *id));
        self.policy
            .sensitive_topics
            .retain(|id| crate::catalog::all(true).iter().any(|t| t.id == *id));
        self.policy.general_topics.sort();
        self.policy.general_topics.dedup();
        self.policy.sensitive_topics.sort();
        self.policy.sensitive_topics.dedup();
        if !["source", "newest", "resolution", "wide", "random"]
            .contains(&self.gallery_order.as_str())
        {
            self.gallery_order = "source".into();
        }

        if !["latest", "toplist", "random"].contains(&self.sort.as_str()) {
            self.sort = "latest".into()
        };
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn item(r: Rating, tags: Option<Vec<&str>>) -> Wallpaper {
        Wallpaper {
            id: "x".into(),
            provider: "fixture".into(),
            title: String::new(),
            thumbnail: String::new(),
            preview_url: String::new(),
            image: String::new(),
            landing: String::new(),
            rating: r,
            tags: tags.map(|v| v.into_iter().map(str::to_string).collect()),
            width: 1920,
            height: 1080,
            author: String::new(),
            license: String::new(),
            local: false,
            created_at: 0,
        }
    }
    #[test]
    fn unknown_and_sensitive_are_not_safe_defaults() {
        let p = Policy::default();
        assert!(p.accepts(&item(Rating::Safe, None)));
        for r in [Rating::Sketchy, Rating::Adult, Rating::Unknown] {
            assert!(!p.accepts(&item(r, None)))
        }
    }
    #[test]
    fn exclusion_wins_and_missing_tags_fail_closed() {
        let mut p = Policy {
            include_enabled: true,
            include_tags: "space".into(),
            exclude_tags: "watermark".into(),
            ..Default::default()
        };
        assert!(!p.accepts(&item(Rating::Safe, Some(vec!["space", "watermark"]))));
        assert!(!p.accepts(&item(Rating::Safe, None)));
        assert!(p.accepts(&item(Rating::Safe, Some(vec!["SPACE"]))));
        p.exclude_enabled = false;
        assert!(p.accepts(&item(Rating::Safe, Some(vec!["space", "watermark"]))))
    }
    #[test]
    fn unknown_dimensions_do_not_pass_minimum_resolution() {
        let p = Policy {
            minimum_width: 3840,
            ..Default::default()
        };
        assert!(!p.accepts(&item(Rating::Safe, None)));
        assert_eq!(p.wallhaven_purity(), "100")
    }
    #[test]
    fn deserialize_defaults_stay_safe() {
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert!(cfg.policy.safe);
        assert!(!cfg.policy.adult);
        assert!(!cfg.policy.unknown)
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    #[test]
    fn sensitive_permissions_are_session_only_unless_opted_in() {
        let mut c = Config::default();
        c.policy.adult = true;
        c.policy.safe = false;
        assert!(!c.persistent().policy.adult);
        assert!(c.persistent().policy.safe);
        c.remember_sensitive = true;
        assert!(c.persistent().policy.adult);
        assert!(!c.persistent().policy.safe);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub id: u64,
    pub item: Wallpaper,
    pub directory: String,
    pub scope: Option<String>,
    pub status: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub last_error_code: String,
    #[serde(default)]
    pub progress: Option<crate::runtime::Progress>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadedImage {
    pub item: Wallpaper,
    pub path: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub facts: crate::runtime::Facts,
    #[serde(default)]
    pub downloaded_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Behavior {
    pub history_enabled: bool,
    pub search_history: Vec<String>,
    pub library_query: String,
    pub library_source: String,
    pub library_format: String,
    pub library_orientation: String,
    pub library_sort: String,
    pub library_missing: bool,
    pub checkerboard: bool,
    pub queue_query: String,
    pub queue_filter: String,
}
impl Default for Behavior {
    fn default() -> Self {
        Self {
            history_enabled: false,
            search_history: vec![],
            library_query: String::new(),
            library_source: String::new(),
            library_format: String::new(),
            library_orientation: "any".into(),
            library_sort: "downloaded".into(),
            library_missing: false,
            checkerboard: false,
            queue_query: String::new(),
            queue_filter: "all".into(),
        }
    }
}
impl Behavior {
    pub fn validate(&mut self) {
        self.search_history
            .iter_mut()
            .for_each(|s| *s = s.trim().chars().take(256).collect());
        self.search_history.retain(|s| !s.is_empty());
        self.search_history.truncate(12);
        if !self.history_enabled {
            self.search_history.clear();
        }
        self.library_query = self.library_query.chars().take(256).collect();
        self.queue_query = self.queue_query.chars().take(256).collect();
        if !["downloaded", "size", "resolution", "title"].contains(&self.library_sort.as_str()) {
            self.library_sort = "downloaded".into();
        }
        if !["any", "landscape", "portrait", "square"].contains(&self.library_orientation.as_str())
        {
            self.library_orientation = "any".into();
        }
    }
    pub fn remember(&mut self, query: &str) {
        let query = query.trim();
        if !self.history_enabled || query.trim().is_empty() {
            return;
        }
        self.search_history.retain(|s| s != query);
        self.search_history
            .insert(0, query.trim().chars().take(256).collect());
        self.search_history.truncate(12);
    }
}
impl Policy {
    pub fn active_filters(&self) -> usize {
        usize::from(self.minimum_width > 0 || self.minimum_height > 0)
            + usize::from(self.orientation != "any")
            + usize::from(!self.formats.is_empty())
            + usize::from(!self.general_topics.is_empty())
            + usize::from(!self.sensitive_topics.is_empty())
            + usize::from(self.include_enabled && !self.include_tags.trim().is_empty())
            + usize::from(self.exclude_enabled && !self.exclude_tags.trim().is_empty())
    }
    pub fn explain(&self, item: &Wallpaper) -> Option<&'static str> {
        if !self.rating_allowed(item.rating) {
            Some("İçerik izni")
        } else if !self.accepts(item) {
            if item.width == 0 || item.height == 0 {
                Some("Eksik boyut veya etiket bilgisi")
            } else if self.minimum_width > item.width || self.minimum_height > item.height {
                Some("Çözünürlük sınırı")
            } else if !crate::catalog::accepts(&self.general_topics, &self.sensitive_topics, item) {
                Some("Konu seçimi")
            } else {
                Some("Etiket/biçim/yön kuralı")
            }
        } else {
            None
        }
    }
}

pub fn dimension(value: &serde_json::Value) -> u32 {
    let number = value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()));
    number.filter(|n| *n > 0 && *n <= 100000).unwrap_or(0) as u32
}
#[cfg(test)]
mod behavior_tests {
    use super::*;
    #[test]
    fn history_is_explicit_and_sensitive_permissions_stay_separate() {
        let mut b = Behavior::default();
        b.remember("mountain");
        assert!(b.search_history.is_empty());
        b.history_enabled = true;
        for _ in 0..3 {
            b.remember("mountain");
        }
        assert_eq!(b.search_history, vec!["mountain"]);
        b.history_enabled = false;
        b.validate();
        assert!(b.search_history.is_empty());
    }
    #[test]
    fn numeric_string_dimensions_are_bounded() {
        for v in [
            serde_json::json!(-1),
            serde_json::json!(0),
            serde_json::json!(100001),
            serde_json::json!("bad"),
        ] {
            assert_eq!(dimension(&v), 0);
        }
        assert_eq!(dimension(&serde_json::json!("3840")), 3840);
    }
    #[test]
    fn relative_destination_and_duplicate_favorites_are_rejected() {
        let mut c = Config {
            download_directory: "../bad".into(),
            ..Default::default()
        };
        c.validate();
        assert!(c.download_directory.is_empty());
        assert!(!c.policy.adult);
    }
}
