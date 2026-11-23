//! Clean, allowlisted distribution. Never collects live preferences, images or diagnostics.
use anyhow::{Context, Result, bail};
use duvar_native::distribution::{BUILD_MANIFEST, PAYLOAD_FILES, UUID, sha};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
fn run(command: &mut Command) -> Result<()> {
    let status = command.status()?;
    if !status.success() {
        bail!("Paketleme aracı başarısız: {status}");
    }
    Ok(())
}
fn copy(from: &Path, to: &Path) -> Result<()> {
    if let Some(p) = to.parent() {
        fs::create_dir_all(p)?;
    }
    fs::copy(from, to)?;
    Ok(())
}
fn tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        if e.file_type()?.is_dir() {
            tree(&e.path(), &to.join(e.file_name()))?;
        } else if e.file_type()?.is_file() {
            copy(&e.path(), &to.join(e.file_name()))?;
        } else {
            bail!("Kaynakta beklenmeyen bağlantı: {}", e.path().display());
        }
    }
    Ok(())
}
fn licenses(root: &Path, to: &Path) -> Result<()> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--offline",
            "--filter-platform",
        ])
        .arg(format!("{}-unknown-linux-gnu", std::env::consts::ARCH))
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        bail!(
            "Bağımlılık lisans listesi okunamadı: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)?;
    let mut index = Vec::new();
    for package in metadata["packages"]
        .as_array()
        .context("Paket listesi yok")?
    {
        let name = package["name"].as_str().context("Paket adı yok")?;
        if name == "duvar" {
            continue;
        }
        let version = package["version"].as_str().context("Paket sürümü yok")?;
        let source = Path::new(
            package["manifest_path"]
                .as_str()
                .context("Paket kaynağı yok")?,
        )
        .parent()
        .context("Kaynak klasörü yok")?;
        let dest = to.join(format!("{name}-{version}"));
        let mut names = Vec::new();
        for e in fs::read_dir(source)? {
            let e = e?;
            let file = e.file_name().to_string_lossy().into_owned();
            if e.file_type()?.is_file()
                && [
                    "LICENSE",
                    "LICENCE",
                    "COPYING",
                    "NOTICE",
                    "COPYRIGHT",
                    "UNLICENSE",
                ]
                .iter()
                .any(|p| file.to_uppercase().starts_with(p))
            {
                copy(&e.path(), &dest.join(&file))?;
                names.push(file);
            }
        }
        if let Some(file) = package["license_file"].as_str() {
            let file = Path::new(file);
            let source = if file.is_absolute() {
                file.to_path_buf()
            } else {
                source.join(file)
            };
            let name = source
                .file_name()
                .context("Lisans dosya adı yok")?
                .to_string_lossy()
                .into_owned();
            if !names.contains(&name) {
                copy(&source, &dest.join(&name))?;
                names.push(name);
            }
        }
        if names.is_empty() {
            bail!(
                "{name} {version} için lisans metni bulunamadı; eksik lisanslı paylaşım paketi üretilmedi"
            );
        }
        index.push(json!({"name":name,"version":version,"spdx":package["license"],"files":names,"source":package["repository"]}));
    }
    let rust = Path::new("/usr/share/licenses/rust");
    for file in ["COPYRIGHT-library.html.rustc", "LICENSE-MIT.cargo"] {
        if rust.join(file).is_file() {
            copy(
                &rust.join(file),
                &to.join("rust-standard-library").join(file),
            )?;
        }
    }
    fs::write(to.join("index.json"), serde_json::to_vec_pretty(&index)?)?;
    Ok(())
}
fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let binary = std::env::current_exe()?;
    let bins = binary.parent().context("Derleme dizini yok")?;
    let library = bins.join("libduvar_native.so");
    if !library.is_file() {
        bail!("Önce cargo build --release --locked çalıştır");
    }
    let dist = root.join("dist");
    fs::create_dir_all(&dist)?;
    let stage = tempfile::Builder::new()
        .prefix("package-")
        .tempdir_in(&dist)?;
    for rel in PAYLOAD_FILES {
        let input = if *rel == "native/libduvar_native.so" {
            library.clone()
        } else if *rel == "LICENSE" {
            root.join(rel)
        } else {
            root.join("extension").join(rel)
        };
        if *rel != "native/Duvar-1.0.typelib" {
            copy(&input, &stage.path().join(rel))?;
        }
    }
    copy(
        &root.join("docs/DISTRIBUTION.md"),
        &stage.path().join("README.md"),
    )?;
    run(Command::new("g-ir-compiler")
        .arg(stage.path().join("native/Duvar-1.0.gir"))
        .arg("--output")
        .arg(stage.path().join("native/Duvar-1.0.typelib")))?;
    run(Command::new("glib-compile-schemas")
        .arg("--strict")
        .arg(stage.path().join("schemas")))?;
    let mut artifacts = serde_json::Map::new();
    for rel in PAYLOAD_FILES {
        artifacts.insert((*rel).into(), json!(sha(&stage.path().join(rel))?));
    }
    let manifest = json!({"schema":1,"version":env!("CARGO_PKG_VERSION"),"uuid":UUID,"architecture":std::env::consts::ARCH,"supported_shell":[51],"minimum_gtk":"4.12","minimum_adwaita":"1.6","artifacts":artifacts,"privacy":"No user preferences, keys, photos, logs or device reports"});
    fs::write(
        stage.path().join(BUILD_MANIFEST),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    licenses(root, &stage.path().join("third-party"))?;
    run(Command::new("gnome-extensions")
        .args([
            "pack",
            "--force",
            "--extra-source=native",
            "--extra-source=icons",
            "--extra-source=LICENSE",
            "--extra-source=README.md",
            "--extra-source=third-party",
            "--extra-source=palette.js",
            "--extra-source=palette-worker.js",
            "--extra-source=user-theme-policy.js",
            "--extra-source=palette-prefs.js",
            "--extra-source=gallery-wallpaper.js",
            "--extra-source=setup-palette.sh",
        ])
        .arg("--out-dir")
        .arg(&dist)
        .arg(stage.path()))?;
    let archive = dist.join(format!("{UUID}.shell-extension.zip"));
    if !archive.is_file() {
        bail!("Uzantı ZIP dosyası oluşmadı");
    }
    let source = tempfile::Builder::new()
        .prefix("source-")
        .tempdir_in(&dist)?;
    let source_name = format!("duvar-source-{}", env!("CARGO_PKG_VERSION"));
    let source_root = source.path().join(&source_name);
    fs::create_dir(&source_root)?;
    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "LICENSE",
        "README.md",
        "AGENTS.md",
        "CHANGELOG.md",
        ".gitignore",
    ] {
        copy(&root.join(file), &source_root.join(file))?;
    }
    tree(&root.join("src"), &source_root.join("src"))?;
    tree(
        &root.join("distribution"),
        &source_root.join("distribution"),
    )?;
    tree(
        &root.join("integrations"),
        &source_root.join("integrations"),
    )?;
    for entry in fs::read_dir(root.join("tests"))? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|e| e == "py" || e == "js" || e == "sh")
        {
            copy(
                &entry.path(),
                &source_root.join("tests").join(entry.file_name()),
            )?;
        }
    }
    for entry in PAYLOAD_FILES
        .iter()
        .filter(|p| **p != "LICENSE" && !p.ends_with(".so") && !p.ends_with(".typelib"))
    {
        copy(
            &root.join("extension").join(entry),
            &source_root.join("extension").join(entry),
        )?;
    }
    for file in ["DISTRIBUTION.md", "SOURCES.md"] {
        copy(
            &root.join("docs").join(file),
            &source_root.join("docs").join(file),
        )?;
    }
    let source_archive = dist.join(format!("{source_name}.tar.gz"));
    run(Command::new("tar")
        .arg("-czf")
        .arg(&source_archive)
        .arg("-C")
        .arg(source.path())
        .arg(&source_name))?;
    let sharing = tempfile::Builder::new()
        .prefix("share-")
        .tempdir_in(&dist)?;
    let share_name = format!(
        "duvar-{}-linux-{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH
    );
    let share_root = sharing.path().join(&share_name);
    fs::create_dir(&share_root)?;
    for bin in ["duvar-install", "duvar-doctor"] {
        copy(&bins.join(bin), &share_root.join(bin))?;
    }
    for file in [&archive, &source_archive] {
        copy(
            file,
            &share_root.join(file.file_name().context("Dosya adı yok")?),
        )?;
    }
    copy(
        &root.join("docs/DISTRIBUTION.md"),
        &share_root.join("ONCE-OKU.md"),
    )?;
    copy(&root.join("LICENSE"), &share_root.join("LICENSE"))?;
    let mut sums = String::new();
    for file in [
        "duvar-install".to_owned(),
        "duvar-doctor".into(),
        archive.file_name().unwrap().to_string_lossy().into(),
        source_archive.file_name().unwrap().to_string_lossy().into(),
        "ONCE-OKU.md".into(),
        "LICENSE".into(),
    ] {
        sums.push_str(&format!("{}  {file}\n", sha(&share_root.join(&file))?));
    }
    fs::write(share_root.join("SHA256SUMS"), sums)?;
    let share_archive = dist.join(format!("{share_name}.tar.gz"));
    run(Command::new("tar")
        .arg("-czf")
        .arg(&share_archive)
        .arg("-C")
        .arg(sharing.path())
        .arg(&share_name))?;
    let share_hash = sha(&share_archive)?;
    fs::write(
        dist.join(format!("{share_name}.tar.gz.sha256")),
        format!("{share_hash}  {share_name}.tar.gz\n"),
    )?;
    let output = json!({"version":env!("CARGO_PKG_VERSION"),"uuid":UUID,"architecture":std::env::consts::ARCH,"library_sha256":sha(&library)?,"archive_sha256":sha(&archive)?,"source_sha256":sha(&source_archive)?,"share_sha256":share_hash,"lock_sha256":sha(&root.join("Cargo.lock"))?,"artifacts":manifest["artifacts"],"installed":false});
    fs::write(
        dist.join("manifest.json"),
        serde_json::to_vec_pretty(&output)?,
    )?;
    println!(
        "Paylaşılacak tek dosya: {}\nStandart GNOME ZIP: {}\nKaynak: {}",
        share_archive.display(),
        archive.display(),
        source_archive.display()
    );
    Ok(())
}
