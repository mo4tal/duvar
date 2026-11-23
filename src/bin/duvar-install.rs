//! Atomic installer/rollback for this extension only. Runtime user data is never copied.
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let m = e.file_type()?;
        if m.is_symlink() {
            bail!("Paket sembolik bağlantı içeremez");
        }
        let dest = to.join(e.file_name());
        if m.is_dir() {
            copy_tree(&e.path(), &dest)?;
        } else if m.is_file() {
            fs::copy(e.path(), dest)?;
        } else {
            bail!("Paket normal dosya içermeli");
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|a| a == "--help") {
        println!(
            "duvar-install [--install|--check] [ARCHIVE.zip]\nduvar-install --rollback CHECKPOINT_DIRECTORY\nArgümansız çalıştırma yanındaki Duvar ZIP dosyasını kurar. --check kurulum yapmadan uyumluluğu denetler. sudo kullanma."
        );
        return Ok(());
    }
    let mode = args.first().map(String::as_str).unwrap_or("--install");
    if !["--install", "--rollback", "--check"].contains(&mode) || args.len() > 2 {
        bail!(
            "Kullanım: duvar-install [--install|--check] [ARCHIVE.zip] veya --rollback CHECKPOINT"
        );
    }
    let adjacent = std::env::current_exe()?
        .parent()
        .context("Kurucu klasörü yok")?
        .join("duvar@mo4tal.local.shell-extension.zip");
    let input = match args.get(1) {
        Some(p) => PathBuf::from(p),
        None if mode != "--rollback" => adjacent,
        None => bail!("Geri dönüş klasörünü belirt"),
    };
    let input = input.canonicalize().with_context(|| {
        format!(
            "Paket bulunamadı: {}. ZIP ile kurucuyu aynı klasöre koy veya dosya yolunu belirt.",
            input.display()
        )
    })?;
    let root = glib::user_data_dir().join("gnome-shell/extensions");
    fs::create_dir_all(&root)?;
    let target = root.join("duvar@mo4tal.local");
    let stage = if mode == "--check" {
        tempfile::Builder::new().prefix("duvar-check-").tempdir()?
    } else {
        tempfile::Builder::new()
            .prefix(".duvar-stage-")
            .tempdir_in(&root)?
    };
    if mode != "--rollback" {
        let out = Command::new("/usr/bin/unzip")
            .args(["-Z1"])
            .arg(&input)
            .output()?;
        if !out.status.success() {
            bail!("ZIP listelenemedi");
        }
        let info = Command::new("/usr/bin/unzip")
            .args(["-Z", "-l"])
            .arg(&input)
            .output()?;
        if !info.status.success() {
            bail!("ZIP özellikleri okunamadı");
        }
        for line in String::from_utf8(info.stdout)?.lines() {
            if let Some(mode) = line.split_whitespace().next()
                && mode.len() == 10
                && mode.starts_with('l')
            {
                bail!("Paket sembolik bağlantı içeriyor");
            }
        }
        for name in String::from_utf8(out.stdout)?.lines() {
            let p = Path::new(name);
            if p.is_absolute()
                || p.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                bail!("Geçersiz paket yolu");
            }
        }
        if !Command::new("/usr/bin/unzip")
            .args(["-q"])
            .arg(&input)
            .arg("-d")
            .arg(stage.path())
            .status()?
            .success()
        {
            bail!("Paket açılamadı");
        }
    } else {
        copy_tree(&input, stage.path())?;
    }
    // Validate the entire staged tree, including extracted symlinks, before publishing it.
    fn validate(p: &Path) -> Result<()> {
        for e in fs::read_dir(p)? {
            let e = e?;
            let t = e.file_type()?;
            if t.is_symlink() {
                bail!("Paket bağlantı içeriyor");
            }
            if t.is_dir() {
                validate(&e.path())?;
            } else if !t.is_file() {
                bail!("Geçersiz paket dosyası");
            }
        }
        Ok(())
    }
    validate(stage.path())?;
    let compatibility = duvar_native::distribution::preflight(stage.path())?;
    println!(
        "Uyumlu: {} · {} · GTK {} · Adwaita {} · Duvar {}",
        compatibility.shell,
        compatibility.architecture,
        compatibility.gtk,
        compatibility.adwaita,
        compatibility.product_version
    );
    if mode == "--check" {
        println!("Kontrol tamamlandı; kurulu uzantı değiştirilmedi.");
        return Ok(());
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&fs::read(stage.path().join("metadata.json"))?)?;
    if meta["uuid"] != "duvar@mo4tal.local" {
        bail!("Bu paket Duvar uzantısına ait değil");
    }
    if !Command::new("/usr/bin/glib-compile-schemas")
        .arg("--strict")
        .arg(stage.path().join("schemas"))
        .status()?
        .success()
    {
        bail!("Ayar şeması derlenemedi; kurulu sürüm korunuyor");
    }
    for rel in [
        "prefs.js",
        "extension.js",
        "native/libduvar_native.so",
        "native/Duvar-1.0.typelib",
        "native/Duvar-1.0.gir",
        "schemas/gschemas.compiled",
    ] {
        if !stage.path().join(rel).is_file() {
            bail!("Eksik paket bileşeni: {rel}");
        }
    }
    let checkpoints = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local/state"))
        .join("duvar/checkpoints");
    fs::create_dir_all(&checkpoints)?;
    if target.exists() {
        let checkpoint = checkpoints.join(format!(
            "installed-before-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_micros()
        ));
        copy_tree(&target, &checkpoint)?;
        let old = std::ffi::CString::new(target.as_os_str().as_encoded_bytes())?;
        let new = std::ffi::CString::new(stage.path().as_os_str().as_encoded_bytes())?;
        let result = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                old.as_ptr(),
                libc::AT_FDCWD,
                new.as_ptr(),
                libc::RENAME_EXCHANGE,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error())
                .context("Atomik dizin değişimi başarısız; kurulu sürüm korunuyor");
        }
        println!(
            "Geri dönüş: duvar-install --rollback '{}'",
            checkpoint.display()
        );
    } else {
        fs::rename(stage.path(), &target)?;
    }
    println!(
        "Duvar {} kuruldu. Tercih penceresini kapatıp aç. Güncellemede GNOME Shell eski kodu bellekte tutabilir; yeni Shell kodu için uygun bir zamanda oturumu kapatıp aç. İlk kurulumda Uzantılar uygulamasından Duvar’ı etkinleştir. Oturum otomatik kapatılmaz.",
        meta["version-name"].as_str().unwrap_or("")
    );
    Ok(())
}
