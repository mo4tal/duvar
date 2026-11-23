use crate::model::Config;
use serde::Serialize;
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Capability {
    pub id: &'static str,
    pub label: &'static str,
    pub search: bool,
    pub dates: bool,
    pub dimensions: bool,
    pub tags: bool,
    pub ratings: bool,
    pub license: bool,
    pub anonymous: bool,
    pub unknown_permission: bool,
    #[serde(skip)]
    pub enabled: fn(&Config) -> bool,
}
pub static SOURCES: &[Capability] = &[
    Capability {
        id: "Wallhaven",
        label: "Wallhaven",
        search: true,
        dates: true,
        dimensions: true,
        tags: true,
        ratings: true,
        license: false,
        anonymous: true,
        unknown_permission: false,
        enabled: |c| c.wallhaven,
    },
    Capability {
        id: "Openverse",
        label: "Openverse",
        search: true,
        dates: false,
        dimensions: true,
        tags: true,
        ratings: true,
        license: true,
        anonymous: true,
        unknown_permission: false,
        enabled: |c| c.openverse,
    },
    Capability {
        id: "NASA",
        label: "NASA",
        search: true,
        dates: true,
        dimensions: true,
        tags: true,
        ratings: false,
        license: true,
        anonymous: true,
        unknown_permission: false,
        enabled: |c| c.nasa,
    },
    Capability {
        id: "Bing",
        label: "Bing",
        search: false,
        dates: true,
        dimensions: true,
        tags: false,
        ratings: false,
        license: true,
        anonymous: true,
        unknown_permission: false,
        enabled: |c| c.bing,
    },
    Capability {
        id: "Commons",
        label: "Wikimedia Commons",
        search: true,
        dates: true,
        dimensions: true,
        tags: true,
        ratings: false,
        license: true,
        anonymous: true,
        unknown_permission: true,
        enabled: |c| c.commons,
    },
    Capability {
        id: "Yande.re",
        label: "Yande.re",
        search: true,
        dates: true,
        dimensions: true,
        tags: true,
        ratings: true,
        license: false,
        anonymous: true,
        unknown_permission: false,
        enabled: |c| c.yandere,
    },
    Capability {
        id: "AIBooru",
        label: "AIBooru",
        search: true,
        dates: true,
        dimensions: true,
        tags: true,
        ratings: true,
        license: false,
        anonymous: true,
        unknown_permission: false,
        enabled: |c| c.aibooru,
    },
    Capability {
        id: "NekosAPI",
        label: "NekosAPI",
        search: true,
        dates: false,
        dimensions: false,
        tags: true,
        ratings: true,
        license: false,
        anonymous: true,
        unknown_permission: false,
        enabled: |c| c.nekos,
    },
    Capability {
        id: "V&A",
        label: "Victoria and Albert Museum",
        search: true,
        dates: false,
        dimensions: false,
        tags: true,
        ratings: false,
        license: true,
        anonymous: true,
        unknown_permission: true,
        enabled: |c| c.vam,
    },
    Capability {
        id: "Cleveland",
        label: "Cleveland Museum of Art",
        search: true,
        dates: false,
        dimensions: true,
        tags: true,
        ratings: false,
        license: true,
        anonymous: true,
        unknown_permission: true,
        enabled: |c| c.cleveland,
    },
    Capability {
        id: "The Met",
        label: "The Met",
        search: true,
        dates: true,
        dimensions: false,
        tags: true,
        ratings: false,
        license: true,
        anonymous: true,
        unknown_permission: true,
        enabled: |c| c.met,
    },
    Capability {
        id: "Yerel",
        label: "Yerel klasör",
        search: true,
        dates: true,
        dimensions: true,
        tags: false,
        ratings: false,
        license: false,
        anonymous: true,
        unknown_permission: true,
        enabled: |c| c.local,
    },
];
pub fn enabled(config: &Config) -> Vec<&'static Capability> {
    SOURCES.iter().filter(|s| (s.enabled)(config)).collect()
}
pub fn compatibility_notes(config: &Config) -> Vec<String> {
    let active = enabled(config);
    if active.is_empty() {
        return vec![
            "Hiçbir kaynak etkin değil; Kaynaklar penceresinden en az bir kaynak aç.".into(),
        ];
    }
    let mut notes = Vec::new();
    let unknown_blocked: Vec<&str> = active
        .iter()
        .filter(|s| s.unknown_permission && !config.policy.unknown)
        .map(|s| s.label)
        .collect();
    if !unknown_blocked.is_empty() {
        notes.push(format!(
            "Etiketsiz içerik izni kapalı olduğu için {} gizlenir.",
            unknown_blocked.join(", ")
        ));
    }
    if config.gallery_order == "newest" {
        let no_dates: Vec<&str> = active
            .iter()
            .filter(|s| !s.dates)
            .map(|s| s.label)
            .collect();
        if !no_dates.is_empty() {
            notes.push(format!(
                "Tarih bilgisi sunmayan kaynaklar ({}) listenin sonunda gruplanır.",
                no_dates.join(", ")
            ));
        }
    }
    if matches!(config.gallery_order.as_str(), "resolution" | "wide")
        || config.policy.minimum_width > 0
        || config.policy.minimum_height > 0
        || config.policy.orientation != "any"
    {
        let no_dims: Vec<&str> = active
            .iter()
            .filter(|s| !s.dimensions)
            .map(|s| s.label)
            .collect();
        if !no_dims.is_empty() {
            notes.push(format!(
                "Ön boyut verisi sunmayan kaynaklar ({}) {} davranır.",
                no_dims.join(", "),
                if config.policy.allow_missing_dimensions {
                    "sondan listelenerek"
                } else {
                    "boyut filtresinde gizlenerek"
                }
            ));
        }
    }
    if config.policy.needs_tags() {
        let no_tags: Vec<&str> = active.iter().filter(|s| !s.tags).map(|s| s.label).collect();
        if !no_tags.is_empty() {
            notes.push(format!(
                "Etiket verisi olmayan kaynaklar ({}) konu/etiket kuralında elenir.",
                no_tags.join(", ")
            ));
        }
    }
    notes
}
pub fn describe(source: &Capability) -> String {
    let yes = |b| if b { "Var" } else { "Yok" };
    format!(
        "Arama: {} · Tarih: {} · Boyut: {}\nEtiket: {} · Güvenlik derecesi: {} · Lisans: {}\nErişim: {}{}",
        yes(source.search),
        yes(source.dates),
        yes(source.dimensions),
        yes(source.tags),
        yes(source.ratings),
        yes(source.license),
        if source.anonymous {
            "Anahtarsız"
        } else {
            "Anahtarlı"
        },
        if source.id == "Wallhaven" {
            " (yetişkin sınıfı anahtar ister)"
        } else {
            ""
        }
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_contract_has_no_duplicates_or_removed_adapters() {
        let ids = SOURCES
            .iter()
            .map(|s| s.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), SOURCES.len());
        assert!(!ids.contains("Danbooru"));
        assert!(!ids.contains("Konachan"));
        assert!(!ids.contains("Gelbooru"));
        assert_eq!(enabled(&Config::default()).len(), 2);
    }
}
