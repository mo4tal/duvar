//! Shared contracts used by cards, preview, queue, diagnostics and the library.
use crate::model::Wallpaper;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Remote,
    Queued,
    Waiting,
    Downloading,
    Validating,
    Saving,
    Completed,
    PauseRequested,
    Paused,
    CancelRequested,
    Cancelled,
    Failed,
    Missing,
    Trashed,
}
impl Phase {
    pub fn active(self) -> bool {
        matches!(
            self,
            Self::Waiting
                | Self::Downloading
                | Self::Validating
                | Self::Saving
                | Self::PauseRequested
                | Self::CancelRequested
        )
    }
    pub fn pending(self) -> bool {
        self.active() || self == Self::Queued
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Remote => "İndirilmedi",
            Self::Queued => "Sırada",
            Self::Waiting => "Önbellek bekleniyor",
            Self::Downloading => "İndiriliyor",
            Self::Validating => "Doğrulanıyor",
            Self::Saving => "Kaydediliyor",
            Self::Completed => "İndirildi",
            Self::PauseRequested => "Duraklatılıyor",
            Self::Paused => "Duraklatıldı",
            Self::CancelRequested => "İptal ediliyor",
            Self::Cancelled => "İptal edildi",
            Self::Failed => "Başarısız",
            Self::Missing => "Dosya bulunamadı",
            Self::Trashed => "Çöp kutusunda",
        }
    }
    pub fn from_status(s: &str) -> Self {
        match s {
            "queued" | "resume_requested" => Self::Queued,
            "active" | "downloading" => Self::Downloading,
            "waiting" => Self::Waiting,
            "validating" => Self::Validating,
            "saving" => Self::Saving,
            "done" | "completed" => Self::Completed,
            "pause_requested" => Self::PauseRequested,
            "paused" => Self::Paused,
            "cancel_requested" => Self::CancelRequested,
            "cancelled" => Self::Cancelled,
            "missing" => Self::Missing,
            "trashed" => Self::Trashed,
            _ => Self::Failed,
        }
    }
    pub fn transition(self, next: Self) -> bool {
        use Phase::*;
        self == next
            || match self {
                Remote | Missing | Trashed => matches!(next, Queued | Completed),
                Queued => matches!(
                    next,
                    Waiting
                        | Downloading
                        | Validating
                        | Saving
                        | Paused
                        | CancelRequested
                        | Cancelled
                        | Failed
                ),
                Waiting | Downloading => matches!(
                    next,
                    Waiting
                        | Downloading
                        | Saving
                        | Completed
                        | Validating
                        | PauseRequested
                        | CancelRequested
                        | Cancelled
                        | Failed
                ),
                Validating => matches!(
                    next,
                    Saving | PauseRequested | CancelRequested | Cancelled | Failed
                ),
                Saving => matches!(next, Completed | CancelRequested | Cancelled | Failed),
                Completed => matches!(next, Missing | Trashed | Queued),
                PauseRequested => matches!(next, Paused | Failed | Completed),
                Paused | Cancelled | Failed => matches!(next, Queued | Cancelled | Trashed),
                CancelRequested => matches!(next, Cancelled | Completed | Failed),
            }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub job: u64,
    pub image: String,
    pub phase: Phase,
    pub received: u64,
    pub total: Option<u64>,
    pub bytes_per_second: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub at: u64,
}
impl Progress {
    pub fn new(job: u64, image: String, phase: Phase) -> Self {
        Self {
            job,
            image,
            phase,
            received: 0,
            total: None,
            bytes_per_second: None,
            eta_seconds: None,
            at: now(),
        }
    }
    pub fn fraction(&self) -> Option<f64> {
        if self.phase == Phase::Completed {
            return Some(1.0);
        }
        self.total
            .filter(|n| *n > 0)
            .map(|n| (self.received as f64 / n as f64).clamp(0.0, 0.999))
    }
    pub fn text(&self) -> String {
        let mut s = self.phase.label().to_string();
        if self.received > 0 {
            s.push_str(&format!(" · {}", size(self.received)));
            if let Some(total) = self.total {
                s.push_str(&format!(" / {}", size(total)));
            }
        }
        if let Some(speed) = self.bytes_per_second.filter(|v| v.is_finite() && *v > 0.0) {
            s.push_str(&format!(" · {}/sn", size(speed as u64)));
        }
        if let Some(eta) = self.eta_seconds {
            s.push_str(&format!(" · {} kaldı", duration(eta)));
        }
        s
    }
}
pub struct Meter {
    start: Instant,
    last: Instant,
    last_bytes: u64,
    ema: Option<f64>,
    samples: u32,
}
impl Meter {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last: now,
            last_bytes: 0,
            ema: None,
            samples: 0,
        }
    }
    pub fn update(
        &mut self,
        received: u64,
        total: Option<u64>,
        force: bool,
    ) -> Option<(Option<f64>, Option<u64>)> {
        let elapsed = self.last.elapsed();
        if !force
            && (elapsed < Duration::from_millis(125)
                || (received.saturating_sub(self.last_bytes) < 32768
                    && elapsed < Duration::from_millis(500)))
        {
            return None;
        }
        if elapsed.as_secs_f64() > 0.05 {
            let rate = received.saturating_sub(self.last_bytes) as f64 / elapsed.as_secs_f64();
            self.ema = Some(self.ema.map_or(rate, |old| old * 0.75 + rate * 0.25));
            self.samples += 1;
        }
        self.last = Instant::now();
        self.last_bytes = received;
        let speed = if self.start.elapsed() >= Duration::from_millis(300) && self.samples >= 2 {
            self.ema
        } else {
            None
        };
        let eta = speed.and_then(|s| {
            total
                .filter(|t| *t > received)
                .filter(|_| s > 1.0)
                .map(|t| ((t - received) as f64 / s).ceil() as u64)
        });
        Some((speed, eta))
    }
}

impl Default for Meter {
    fn default() -> Self {
        Self::new()
    }
}
pub fn size(n: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", units[unit])
    }
}
pub fn duration(n: u64) -> String {
    if n >= 3600 {
        format!("{} sa {} dk", n / 3600, n % 3600 / 60)
    } else if n >= 60 {
        format!("{} dk {} sn", n / 60, n % 60)
    } else {
        format!("{n} sn")
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Landscape,
    Portrait,
    Square,
    #[default]
    Unknown,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Facts {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub bytes: u64,
    pub sha256: String,
    pub alpha: bool,
    pub verified: bool,
    pub orientation: Orientation,
    pub learned_at: u64,
    pub dimension_origin: String,
}
impl Facts {
    pub fn dimensions(&self) -> String {
        if self.width > 0 && self.height > 0 {
            format!("{} × {}", self.width, self.height)
        } else {
            "Boyut bilinmiyor".into()
        }
    }
    pub fn megapixels(&self) -> Option<f64> {
        if self.width == 0 || self.height == 0 {
            None
        } else {
            Some(self.width as f64 * self.height as f64 / 1_000_000.0)
        }
    }
    pub fn ratio(&self) -> String {
        if self.width == 0 || self.height == 0 {
            return "Bilinmiyor".into();
        }
        fn gcd(mut a: u32, mut b: u32) -> u32 {
            while b != 0 {
                let r = a % b;
                a = b;
                b = r;
            }
            a.max(1)
        }
        let d = gcd(self.width, self.height);
        format!("{}:{}", self.width / d, self.height / d)
    }
    pub fn orientation_label(&self) -> &'static str {
        match self.orientation {
            Orientation::Landscape => "Yatay",
            Orientation::Portrait => "Dikey",
            Orientation::Square => "Kare",
            Orientation::Unknown => "Yön bilinmiyor",
        }
    }
    pub fn from_size(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            dimension_origin: if width > 0 && height > 0 {
                "source".into()
            } else {
                "unknown".into()
            },
            orientation: if width == 0 || height == 0 {
                Orientation::Unknown
            } else if width == height {
                Orientation::Square
            } else if width > height {
                Orientation::Landscape
            } else {
                Orientation::Portrait
            },
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Actions {
    pub download: bool,
    pub apply: bool,
    pub cancel: bool,
    pub retry: bool,
    pub trash: bool,
    pub open: bool,
    pub restore: bool,
}
impl Actions {
    pub fn for_state(phase: Phase, managed: bool, local_exists: bool) -> Self {
        let pending = phase.pending();
        Self {
            download: !pending && !local_exists && phase != Phase::Trashed,
            apply: local_exists && !matches!(phase, Phase::Trashed),
            cancel: pending,
            retry: matches!(
                phase,
                Phase::Failed | Phase::Cancelled | Phase::Missing | Phase::Paused
            ),
            trash: managed && local_exists && !pending,
            open: local_exists,
            restore: phase == Phase::Trashed,
        }
    }
}
pub fn types(item: &Wallpaper) -> Vec<String> {
    let mut out = vec![item.rating.label().to_string()];
    let f = Facts::from_size(item.width, item.height);
    if f.orientation != Orientation::Unknown {
        out.push(f.orientation_label().into());
    }
    let room = 8 - out.len();
    for t in crate::catalog::matched(item).take(room) {
        out.push(t.title.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_is_not_complete_until_committed() {
        let mut p = Progress::new(1, "x".into(), Phase::Downloading);
        p.total = Some(10);
        p.received = 10;
        assert!(p.fraction().unwrap() < 1.0);
        p.phase = Phase::Completed;
        assert_eq!(p.fraction(), Some(1.0));
        p.total = None;
        p.phase = Phase::Downloading;
        assert_eq!(p.fraction(), None);
    }
    #[test]
    fn pending_actions_never_duplicate_downloads() {
        for phase in [
            Phase::Queued,
            Phase::Waiting,
            Phase::Downloading,
            Phase::Validating,
            Phase::Saving,
        ] {
            let a = Actions::for_state(phase, false, false);
            assert!(!a.download);
            assert!(a.cancel);
        }
        let a = Actions::for_state(Phase::Completed, true, true);
        assert!(!a.download && a.apply && a.trash);
    }
    #[test]
    fn facts_do_not_render_zero_dimensions() {
        let f = Facts::default();
        assert!(!f.dimensions().contains("0 × 0"));
        assert_eq!(Facts::from_size(3840, 2160).ratio(), "16:9");
        assert_eq!(Facts::from_size(1, 2).orientation, Orientation::Portrait);
    }
    #[test]
    fn invalid_state_jumps_are_rejected() {
        assert!(!Phase::Queued.transition(Phase::Completed));
        assert!(Phase::Downloading.transition(Phase::Validating));
        assert!(!Phase::Trashed.active());
    }
}
