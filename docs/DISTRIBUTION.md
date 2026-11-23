# Duvar — Dağıtım ve Kurulum / Distribution & Installation

## Türkçe

### 1. Hazır paylaşım paketiyle kurulum (önerilen — x86_64)

[GitHub Releases](https://github.com/mo4tal/duvar/releases) sayfasından `duvar-<sürüm>-linux-x86_64.tar.gz` arşivini indir, aç ve klasörün içinde çalıştır:

```sh
sha256sum -c SHA256SUMS   # isteğe bağlı bütünlük kontrolü
./duvar-install --check    # kurmadan önce uyumluluk denetimi
./duvar-install            # atomik kurulum + geri dönüş noktası
```

- **sudo gerekmez.** Rust, Cargo veya derleme araçları gerekmez.
- Kurucu; dosya SHA-256 özetlerini, işlemci mimarisini, GNOME Shell sürümünü ve `libduvar_native.so` dinamik yüklemesini **mevcut kurulumu değiştirmeden önce** doğrular.
- Başarılı güncellemede önceki kod için otomatik geri dönüş noktası oluşturur (`duvar-install --rollback <yol>`). Tercihler, duvar kâğıtları ve anahtarlık dokunulmadan kalır.
- Kurulum sonrası çevrimdışı sağlık denetimi için: `./duvar-doctor`

Ardından **Uzantılar → Duvar → etkinleştir → ayarlar (dişli)**. İlk kurulumda veya Shell tarafı güncellendiğinde GNOME Shell’in yeni JS modülünü yüklemesi için bir kez oturumu kapatıp açman gerekebilir; tercih penceresi ise kapatıp açıldığında anında güncellenir.

### 2. Kaynaktan derleme (tüm Linux mimarileri)

**Gereksinimler:** Rust / Cargo ≥ 1.88, C derleyici, `pkg-config`, GTK 4 (≥ 4.12) ve Libadwaita (≥ 1.6) geliştirme paketleri, `g-ir-compiler`, `glib-compile-schemas`, `gnome-extensions`, `tar`, `unzip`.

```sh
git clone https://github.com/mo4tal/duvar.git
cd duvar
cargo build --release --locked
./target/release/duvar-package
./target/release/duvar-install --install dist/duvar@mo4tal.local.shell-extension.zip
```

### 3. GNOME Extensions (EGO) için saf-JS paket

Tam sürüm; derlenmiş Rust kitaplığını (`libduvar_native.so`) ve GI typelib’ini taşıdığı için doğrudan GitHub Releases üzerinden paylaşılır. Derlenmiş ikili içermeyen, yalnızca **Renkler** motorunu ve ayar sayfasını barındıran EGO mağaza adayı ayrı üretilir:

```sh
bash distribution/pack-store.sh
# Çıktı: dist/store/duvar@mo4tal.local.shell-extension.zip
```

İki paket aynı UUID’yi (`duvar@mo4tal.local`) ve aynı GSettings şemasını kullanır; aynı anda yalnızca biri kurulur.

### 4. Renk motoru katmanları

- **GNOME Shell:** User Themes kurulu olmasa bile `St.Theme.load_stylesheet` ile doğrudan uygulanır; User Themes etkinse onunla uyumlu çalışır.
- **Üst panel kapsülleri:** Açık ve koyu temada `capsule`, `smart`, `scrim` veya `plain` stiliyle duvar kâğıdına uyarlanan okunaklı üst panel sağlar. Blur My Shell açıksa bulanıklık ayarlarına dokunmaz.
- **GTK 3 / GTK 4:** Renkler açıldığında kullanıcı tema dizininde açık ve koyu paletleri üretir.
- **Libadwaita (isteğe bağlı, varsayılan kapalı):** GTK 4.20+ kullanıcı CSS’sinin başına yalnızca Duvar’a ait yönetilen `@import` satırını ekler; kapatıldığında temizler. Sembolik bağlantılara veya üçüncü taraf tema kurallarına dokunmaz.
- **Ghostty (isteğe bağlı):** `integrations/ghostty-sync.py` ve `integrations/duvar-ghostty.service` ile Duvar’ın kayıtlı açık/koyu paletini Ghostty temalarına yansıtabilirsin.

---

## English

### 1. Prebuilt release installation (Linux x86_64)

Download `duvar-<version>-linux-x86_64.tar.gz` from [GitHub Releases](https://github.com/mo4tal/duvar/releases), extract it, and run:

```sh
sha256sum -c SHA256SUMS   # optional integrity check
./duvar-install --check    # dry-run compatibility check
./duvar-install            # atomic install with rollback checkpoint
```

- **No `sudo`, no Rust toolchain, no automatic system package installs, and no forced logout.**
- Preflight validates SHA-256 checksums, CPU architecture, GNOME Shell 51, GTK ≥ 4.12, Libadwaita ≥ 1.6, and `dlopen` loading before touching any existing installation.
- Run `./duvar-doctor` anytime for an offline health check.
- Enable **Duvar** in GNOME Extensions and click its settings gear. A manual logout/login may be needed once for GNOME Shell to reload updated extension JS; reopening the preferences window loads updated native gallery UI immediately.

### 2. Building from source

Requires Rust / Cargo ≥ 1.88, `pkg-config`, GTK 4 (≥ 4.12) & Libadwaita (≥ 1.6) dev headers, `g-ir-compiler`, `glib-compile-schemas`, `gnome-extensions`, `tar`, and `unzip`.

```sh
git clone https://github.com/mo4tal/duvar.git
cd duvar
cargo build --release --locked
./target/release/duvar-package
./target/release/duvar-install --install dist/duvar@mo4tal.local.shell-extension.zip
```

### 3. Pure-JS GNOME Extensions (EGO) variant

Because extensions.gnome.org does not accept precompiled native libraries, a gallery-free pure-JS variant containing only the adaptive color engine and its **Renkler** preferences page can be built with:

```sh
bash distribution/pack-store.sh
# Output: dist/store/duvar@mo4tal.local.shell-extension.zip
```

### 4. Verification commands

```sh
cargo fmt --all --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline --lib
gjs -m tests/palette-roles.js
bash tests/palette-worker.sh
python3 tests/run-preferences-smoke.py
bash tests/palette-shell-smoke.sh
```
