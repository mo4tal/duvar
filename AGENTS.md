# AGENTS.md — Duvar Architecture & Contributor Guide

Guide for AI coding assistants and human contributors working on **Duvar** (`duvar@mo4tal.local`).

## 1. What Duvar Is

Duvar is a **GNOME Shell 51** extension with two cleanly separated layers:

1. **Pure-GJS GNOME Shell & Palette Layer (`extension/`)**
   - Extracts dominant OKLab colors and a 16-slice top-bar luminance profile from light/dark wallpapers in an isolated subprocess (`palette-worker.js`).
   - Synthesizes 10 perceptually balanced light/dark theme profiles (`palette.js`) and applies them to GNOME Shell (directly via `St.Theme.load_stylesheet`, or via User Themes when enabled), GTK 3/4, and optional Libadwaita user CSS.
   - Renders ambient-tinted floating glass top-panel capsules (`capsule` / `smart` / `scrim` / `plain`) that keep top-bar text and icons legible over arbitrary dark/light wallpapers.
2. **Native Rust + GTK 4 + Libadwaita Gallery Layer (`src/` → `libduvar_native.so`)**
   - Loaded inside GNOME's own `AdwPreferencesWindow` via GObject Introspection (`extension/native/Duvar-1.0.gir` + `.typelib`).
   - Provides multi-source wallpaper discovery (`Keşfet`), local library + persistent download queue (`İndirilenler`), 200-topic & safety filtering (`Filtreler`), settings & About dialog (`Ayarlar`), and a Loupe-style Pro Lightbox.
   - No background daemon, no panel indicator, no separate desktop launcher.

A gallery-free pure-JS variant for **GNOME Extensions (EGO)** is packaged separately via `distribution/pack-store.sh`.

---

## 2. Repository Map

```text
duvar/
├── Cargo.toml                  # Crate manifest (cdylib + rlib + 4 CLI binaries)
├── Cargo.lock                  # Locked dependency graph (--locked --offline)
├── AGENTS.md                   # This file
├── CHANGELOG.md                # Release history
├── README.md                   # Project showcase & quick start
├── LICENSE                     # GPL-3.0-or-later
│
├── extension/                  # GNOME Shell extension payload
│   ├── metadata.json           # Extension UUID, shell-version, version & version-name
│   ├── extension.js            # Shell lifecycle, theme application, top-bar capsule observer
│   ├── palette.js              # OKLab palette generator, CSS templates (Shell/GTK/Adw/BMS)
│   ├── palette-worker.js       # Isolated GJS subprocess for image decoding & OKLab clustering
│   ├── palette-prefs.js        # "Renkler" preferences page + live top-bar preview cards
│   ├── prefs.js                # Entry point: loads libduvar_native.so via GI + adds Renkler
│   ├── gallery-wallpaper.js    # File picker helper bridging Renkler and wallpaper targets
│   ├── user-theme-policy.js    # User Themes activation policy (auto for local, manual for EGO)
│   ├── setup-palette.sh        # Optional dependency checker/enabler script
│   ├── schemas/                # GSettings XML schema (org.gnome.shell.extensions.duvar)
│   ├── icons/hicolor/          # App icons (local.mo4tal.Duvar) & symbolic filter icon
│   └── native/Duvar-1.0.gir    # GObject Introspection definition for duvar_populate / duvar_version
│
├── src/                        # Rust native library (duvar_native) & tooling
│   ├── lib.rs                  # C ABI entry points: duvar_populate, duvar_version
│   ├── model.rs                # Config, Policy, Rating, Item, Sort, Filter types & defaults
│   ├── registry.rs             # Declarative capabilities & contracts for all 12 sources
│   ├── sources.rs              # Provider query builders & JSON response parsers
│   ├── providers.rs            # Network facade coordinating search, thumbnails, previews, downloads
│   │   └── providers/
│   │       ├── http.rs         # ureq HTTP client, cooldowns, Retry-After, strict HTTPS validation
│   │       ├── metadata.rs     # Tag & header verification, fail-closed rating checks
│   │       └── transfer.rs     # Streaming bounded downloads with atomic swap & cancellation
│   ├── engine.rs               # 6-lane thread pool (8 LIFO thumb workers, 2 preview, 2 download)
│   ├── cache.rs                # 256 MiB shared disk cache, atomic_fast writes, throttled trim
│   ├── thumbnail.rs            # Lock-free thumbnail decoding & RGBA disk cache
│   ├── files.rs                # Safe file ops, pixel-verified facts, trash/restore
│   ├── store.rs                # Atomic config, metadata store (~/.local/share/duvar/metadata)
│   ├── library.rs              # Local folder scanner & downloaded items index
│   ├── queue.rs                # Persistent download queue (schema v2) & recovery
│   ├── catalog.rs              # 100 general + 100 sensitive topics with Turkish casefold search
│   ├── runtime.rs              # State transitions, pending action deduplication
│   ├── errors.rs               # Machine error codes & path/secret redaction
│   ├── diagnostics.rs          # Lightweight performance spans
│   ├── distribution.rs         # PAYLOAD_FILES manifest, SHA-256 verification, dlopen preflight
│   ├── doctor.rs               # Offline health audit logic
│   ├── ui.rs                   # Main GTK4/Adwaita window, Keşfet grid (180-LRU), Pro Lightbox
│   │   └── ui/
│   │       ├── preferences.rs  # İndirilenler, Filtreler, Ayarlar tabs, Sources & About dialogs
│   │       ├── topics.rs       # 200-topic searchable category selector dialog
│   │       └── transfer_ui.rs  # Queue controls & responsive button strips
│   └── bin/
│       ├── duvar-package.rs    # Builds ZIP, clean source tarball, sharing bundle & manifest
│       ├── duvar-install.rs    # Preflight-checked atomic installer with rollback checkpoints
│       ├── duvar-doctor.rs     # Offline health & optional --verify / --gui test runner
│       └── duvar-check.rs      # Explicit live source & migration diagnostic tool
│
├── distribution/               # EGO store-only packaging
│   ├── pack-store.sh           # Builds gallery-free pure-JS EGO candidate ZIP
│   └── store/                  # Store-only prefs.js & manual user-theme-policy.js
│
├── integrations/               # Optional desktop integrations (not bundled in EGO ZIP)
│   ├── ghostty-sync.py         # Mirrors Duvar's light/dark palette-colors into Ghostty themes
│   └── duvar-ghostty.service   # User systemd unit for ghostty-sync.py
│
├── tests/                      # Automated verification suites (all isolated, zero host side effects)
│   ├── palette-roles.js        # Exhaustive contrast, OKLab token, Polkit/Shell CSS & BMS checks
│   ├── palette-worker.sh       # Subprocess image extraction, spatial luminance & XML tests
│   ├── run-preferences-smoke.py# End-to-end GTK4/Wayland GUI suite (runs 6 preferences-*.js)
│   ├── palette-shell-smoke.sh  # Isolated headless GNOME Shell 51 E2E (local + EGO store ZIP)
│   └── install-roundtrip.py    # Isolated install/upgrade/rollback verification
│
└── docs/                       # User & packaging documentation + visual assets
    ├── icon.svg                # Application icon (copy of local.mo4tal.Duvar.svg)
    ├── DISTRIBUTION.md         # Sharing, installation & build instructions (TR + EN)
    ├── SOURCES.md              # Source API contracts & content safety rules
    └── screenshots/            # Real Wayland screenshots used in README.md
```

---

## 3. Version & Packaging Contract (CRITICAL)

Whenever you change the version or add/remove any file in `extension/`, keep these **4 places strictly synchronized**:

1. `Cargo.toml` (`version = "X.Y.Z"`) and `Cargo.lock`
2. `extension/metadata.json` (`"version": N`, `"version-name": "X.Y.Z"`)
3. `src/distribution.rs` → `PAYLOAD_FILES` (every shipped file in `extension/` must be listed; `duvar-package` hashes each file into `native/duvar-build.json`, and `duvar-install` + `prefs.js` enforce exact version & SHA-256 matches)
4. `src/bin/duvar-package.rs` → `--extra-source=...` args to `gnome-extensions pack` (and `distribution/pack-store.sh` if the file belongs in the pure-JS store build)

---

## 4. Build, Test & Install Commands

### Fast verification (no Wayland / no network required)
```sh
cargo fmt --all --check
cargo clippy --locked --offline --all-targets -- -D warnings
TMPXDG=$(mktemp -d) && XDG_DATA_HOME="$TMPXDG" cargo test --locked --offline --lib && rm -rf "$TMPXDG"
gjs -m tests/palette-roles.js
bash tests/palette-worker.sh
```

### Release build & packaging
```sh
cargo build --release --locked
./target/release/duvar-package
bash distribution/pack-store.sh
```
Outputs go into `dist/` (gitignored):
- `dist/duvar-<ver>-linux-<arch>.tar.gz` — self-contained sharing bundle (`duvar-install`, `duvar-doctor`, ZIP, source tarball, `SHA256SUMS`)
- `dist/duvar@mo4tal.local.shell-extension.zip` — full extension ZIP with native Rust gallery
- `dist/store/duvar@mo4tal.local.shell-extension.zip` — pure-JS EGO store candidate

### Full Wayland GUI & Headless GNOME Shell 51 verification
```sh
python3 tests/run-preferences-smoke.py
bash tests/palette-shell-smoke.sh
DUVAR_STORE_ZIP=dist/store/duvar@mo4tal.local.shell-extension.zip bash tests/palette-shell-smoke.sh
python3 tests/install-roundtrip.py
```
> All tests run inside private temporary `HOME` / `XDG_*` / `dbus-run-session` sandboxes and never touch the host desktop settings, keyring, or wallpapers. Smoke logs and test screenshots default to `target/smoke/`.

### Local atomic installation
```sh
./target/release/duvar-install --install dist/duvar@mo4tal.local.shell-extension.zip
./target/release/duvar-doctor
```

---

## 5. Non-Negotiable Invariants

- **Fail-closed safety:** New installs start strictly in `SFW` mode. Sensitive permissions (`questionable`, `explicit`, `unknown`) are session-only unless explicitly opted in. Never auto-enable sensitive permissions when selecting topics.
- **No sidecar clutter:** Source metadata lives exclusively under `~/.local/share/duvar/metadata/`. Never write `.json` sidecars into the user's wallpaper directories.
- **Zero secret leaks:** Wallhaven API keys live only in the existing Secret Service keyring (`secret-tool`). Never persist keys into config, queue, logs, or manifests.
- **GNOME Shell CSS hygiene:** When editing `extension/palette.js`, verify selectors against GNOME Shell 51's built-in stylesheet. Never paint background colors onto `.prompt-dialog-null-label` or `.dialog-list-item`, preserve warning accents on root/error labels, and keep Blur My Shell panel blur untouched.
- **Zero-splice gallery grid:** In `src/ui.rs`, thumbnails paint in place onto fixed-height cards without splicing `gio::ListStore` items, preventing scroll-position jumps.
- **Portable design:** Never hardcode personal paths, usernames, or distro-only assumptions in runtime code or docs.
