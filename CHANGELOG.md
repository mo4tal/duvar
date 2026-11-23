# Changelog

All notable changes to **Duvar** are documented in this file.

## [0.6.0] — 2026-11-23

### Added
- **Floating glass top-panel capsules:** Ambient-tinted glass pills around top-bar items in both light and dark modes (`capsule`, `smart`, `scrim`, `plain`) with selectable intensity (`subtle`, `balanced`, `crisp`) and live top-bar previews in **Renkler**.
- **Direct Shell stylesheet loading:** Applies GNOME Shell theme styles via `St.Theme.load_stylesheet` even when the User Themes extension is not installed or enabled, while staying compatible when User Themes is active.
- **OKLab dominant color & 16-slice contrast engine:** Perceptual OKLab clustering paired with spatial top-bar luminance profiling to preserve legibility across opposite-brightness wallpapers (e.g. pitch-black wallpaper in light mode or bright wallpaper in dark mode).
- **Pro Lightbox preview:** Instant progressive transition from cached thumbnail to native pixels, cursor-anchored mouse-wheel zoom, left-click drag-to-pan, touch pinch zoom, right-click context popover, `←/→` navigation, and an `AdwOverlaySplitView` metadata/tags drawer.
- **Original GNOME / Adwaita app icon:** Full-color scalable SVG (`io.github.mo4tal.Duvar.svg`) and monochrome symbolic SVG (`io.github.mo4tal.Duvar-symbolic.svg`), bundled in both the full and EGO store packages.
- **10-language localization (`locales/`):** Turkish default (`tr`) plus English (`en`), Spanish (`es`), German (`de`), French (`fr`), Portuguese (`pt`), Russian (`ru`), Japanese (`ja`), Chinese (`zh`), and Arabic (`ar`), selectable in Settings (`ui-language`), backed by a documented `locales/CONTEXT.md` catalog for humans and AI assistants.
- **Recommended GNOME desktop preset script (`scripts/setup-recommended-desktop.sh`):** One-command, sudo-free, rollback-safe setup for Duvar + Blur My Shell + User Themes + Quick Settings Avatar + Rounded Window Corners with dynamic GNOME Shell version discovery.
- **Redesigned About dialog & bilingual documentation:** Full `AdwAboutDialog` with release notes, technical debug info, GitHub repository/issue links, custom hero banner, English-first `README.md`, Turkish `README.tr.md`, and synchronized `0.6.0` package contracts.

### Changed
- **Complete Adwaita HIG UI redesign:** Streamlined all 5 tabs (`Keşfet`, `İndirilenler`, `Filtreler`, `Ayarlar`, `Renkler`), removed double-hitbox `FlowBox`/`GridView` hover artifacts, tightened action bars and dropdowns, and aligned button proportions across wide and narrow (480 px) layouts.
- **6-lane Rust async engine:** 8 LIFO thumbnail workers with unbind cancellation, 2 dedicated preview/metadata workers, lock-free thumbnail decode, 180-item in-memory LRU cache, zero-splice in-place card painting (eliminating scroll jumps), and throttled disk cache trimming.
- **Server-side sorting & source honesty:** Real server-side sort/filter mapping for Wallhaven, Openverse, Yande.re, and AIBooru; real dates/dimensions for Bing, NASA, Wikimedia Commons, and local folders; clear source compatibility indicators in the UI.

### Fixed
- **Polkit & Shell dialog styling:** Removed unintended background fill on `.prompt-dialog-null-label` (fixing the empty grey box under Polkit password fields) and `.dialog-list-item`; preserved warning colors on root/error labels.
- **Scroll & download stability:** Downloading a wallpaper no longer resets scroll position to the top of `Keşfet` or alters active filter selections.

## [0.5.x] — 2026-11-22

- Added dynamic wallpaper-driven palette engine (`palette.js`, `palette-worker.js`, `palette-prefs.js`) with 10 theme profiles, GTK 3/4 and optional Libadwaita styling, Blur My Shell compatibility, and pure-JS EGO store packaging (`distribution/pack-store.sh`).
- Added self-contained distribution pipeline (`duvar-package`, `duvar-install`, `duvar-doctor`) with SHA-256 manifests, `dlopen` preflight checks, and rollback checkpoints.
- Added optional Ghostty palette sync integration (`integrations/ghostty-sync.py`).

## [0.4.0] — 2026-11-15

- Added native-resolution original preview, 200-topic searchable catalogue (100 general + 100 sensitive), and museum open-access sources (Cleveland Museum of Art, The Met, Victoria and Albert Museum).
- Removed unreliable unauthenticated booru providers and split UI into `Keşfet`, `İndirilenler`, `Filtreler`, and `Ayarlar`.

## [0.1.0 – 0.3.0] — 2026-11-13

- Initial Rust + GTK 4 + Libadwaita extension architecture loaded via GObject Introspection into GNOME Shell 51's preferences window.
- Added persistent download queue, sidecar-free metadata storage, fail-closed SFW defaults, and keyring-backed Wallhaven API support.
