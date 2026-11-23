//! Portable package contracts. No user preferences or credentials are read here.
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    ffi::{CStr, CString},
    fs,
    io::Read,
    path::Path,
    process::Command,
};
pub const UUID: &str = "duvar@mo4tal.local";
pub const BUILD_MANIFEST: &str = "native/duvar-build.json";
pub const PAYLOAD_FILES: &[&str] = &[
    "metadata.json",
    "prefs.js",
    "extension.js",
    "palette.js",
    "palette-worker.js",
    "palette-prefs.js",
    "user-theme-policy.js",
    "gallery-wallpaper.js",
    "setup-palette.sh",
    "LICENSE",
    "native/libduvar_native.so",
    "native/Duvar-1.0.gir",
    "native/Duvar-1.0.typelib",
    "schemas/org.gnome.shell.extensions.duvar.gschema.xml",
    "icons/hicolor/index.theme",
    "icons/hicolor/scalable/actions/duvar-filter-symbolic.svg",
    "icons/hicolor/scalable/apps/local.mo4tal.Duvar.svg",
    "icons/hicolor/symbolic/apps/local.mo4tal.Duvar-symbolic.svg",
];
pub fn sha(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn version(value: &str) -> Option<(u32, u32)> {
    let token = value
        .split_whitespace()
        .find(|s| s.starts_with(|c: char| c.is_ascii_digit()))?;
    let mut n = token.split('.');
    let major = n.next()?.parse().ok()?;
    let minor = n.next().unwrap_or("0").parse().unwrap_or(0);
    Some((major, minor))
}
pub fn verify_manifest(root: &Path, manifest: &serde_json::Value) -> Result<()> {
    if manifest["schema"] != 1 || manifest["uuid"] != UUID {
        bail!("Paket manifesti bu sürümle uyumlu değil");
    }
    let entries = manifest["artifacts"]
        .as_object()
        .context("Dosya hash listesi eksik")?;
    for rel in PAYLOAD_FILES {
        let expected = entries
            .get(*rel)
            .and_then(|v| v.as_str())
            .context("Paket manifestinde zorunlu dosya eksik")?;
        if expected.len() != 64 || sha(&root.join(rel))? != expected {
            bail!("Paket bütünlük kontrolü başarısız: {rel}");
        }
    }
    Ok(())
}
struct Module(*mut libc::c_void);
impl Module {
    fn open(path: &Path) -> Result<Self> {
        let name = CString::new(path.as_os_str().as_encoded_bytes())?;
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        if handle.is_null() {
            let error = unsafe { libc::dlerror() };
            let message = if error.is_null() {
                "Bilinmeyen kitaplık yükleme hatası".into()
            } else {
                unsafe { CStr::from_ptr(error) }
                    .to_string_lossy()
                    .into_owned()
            };
            bail!(
                "Yerel kitaplık bu bilgisayarda yüklenemedi: {message}. Eksik/uyumsuz sistem kitaplıklarını tamamla veya kaynak paketini bu bilgisayarda derle."
            );
        }
        Ok(Self(handle))
    }
    fn symbol(&self, name: &str) -> Result<*mut libc::c_void> {
        let name = CString::new(name)?;
        let p = unsafe { libc::dlsym(self.0, name.as_ptr()) };
        if p.is_null() {
            bail!(
                "Gerekli kitaplık sembolü bulunamadı: {}",
                name.to_string_lossy()
            );
        }
        Ok(p)
    }
    fn number(&self, name: &str) -> Result<u32> {
        let p = self.symbol(name)?;
        let get: unsafe extern "C" fn() -> u32 = unsafe { std::mem::transmute(p) };
        Ok(unsafe { get() })
    }
    fn product_version(&self) -> Result<String> {
        let p = self.symbol("duvar_version")?;
        let get: unsafe extern "C" fn() -> *const libc::c_char = unsafe { std::mem::transmute(p) };
        let value = unsafe { get() };
        if value.is_null() {
            bail!("Kitaplık sürümü boş");
        }
        Ok(unsafe { CStr::from_ptr(value) }.to_str()?.to_owned())
    }
}
impl Drop for Module {
    fn drop(&mut self) {
        unsafe { libc::dlclose(self.0) };
    }
}
#[derive(Debug, Serialize)]
pub struct Compatibility {
    pub architecture: String,
    pub shell: String,
    pub gtk: String,
    pub adwaita: String,
    pub product_version: String,
    pub manifest_verified: bool,
}
pub fn preflight(root: &Path) -> Result<Compatibility> {
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("metadata.json"))?)?;
    if metadata["uuid"] != UUID {
        bail!("Bu paket Duvar uzantısına ait değil");
    }
    let supported = metadata["shell-version"]
        .as_array()
        .context("GNOME sürüm listesi eksik")?;
    let shell = Command::new("gnome-shell")
        .arg("--version")
        .output()
        .context("GNOME Shell bulunamadı; Duvar GNOME masaüstü gerektirir")?;
    let shell = String::from_utf8(shell.stdout)?.trim().to_owned();
    let major = version(&shell)
        .context("GNOME Shell sürümü belirlenemedi")?
        .0;
    if !supported
        .iter()
        .any(|v| v.as_str() == Some(&major.to_string()))
    {
        bail!(
            "Bu paket GNOME {:?} için hazırlanmış; bilgisayardaki sürüm: {shell}. GNOME sürümünü zorlayarak etkinleştirme yapılmadı.",
            supported
        );
    }
    let manifest = root.join(BUILD_MANIFEST);
    let verified = manifest.is_file();
    if verified {
        let manifest: serde_json::Value = serde_json::from_slice(&fs::read(manifest)?)?;
        if manifest["architecture"] != std::env::consts::ARCH {
            bail!(
                "Paket mimarisi {} bu bilgisayarla ({}) eşleşmiyor; kaynak paketinden derle.",
                manifest["architecture"],
                std::env::consts::ARCH
            );
        }
        verify_manifest(root, &manifest)?;
    }
    // These C getters are pure metadata; no GTK display or GUI initialization is needed.
    let module = Module::open(&root.join("native/libduvar_native.so"))?;
    module.symbol("duvar_populate")?;
    let gtk = (
        module.number("gtk_get_major_version")?,
        module.number("gtk_get_minor_version")?,
    );
    let adw = (
        module.number("adw_get_major_version")?,
        module.number("adw_get_minor_version")?,
    );
    if gtk.0 != 4 || gtk.1 < 12 || adw.0 != 1 || adw.1 < 6 {
        bail!(
            "GTK ≥4.12 ve libadwaita ≥1.6 gerekiyor; kurulu: GTK {}.{}, Adwaita {}.{}",
            gtk.0,
            gtk.1,
            adw.0,
            adw.1
        );
    }
    let expected = metadata["version-name"]
        .as_str()
        .context("Paket sürümü eksik")?
        .split('-')
        .next()
        .unwrap_or("");
    let product = module.product_version().or_else(|e| {
        if !verified && expected.starts_with("0.4.") {
            Ok(expected.into())
        } else {
            Err(e)
        }
    })?;
    if product != expected {
        bail!("Paket arayüzü ({expected}) ile Rust kitaplığı ({product}) eşleşmiyor");
    }
    Ok(Compatibility {
        architecture: std::env::consts::ARCH.into(),
        shell,
        gtk: format!("{}.{}", gtk.0, gtk.1),
        adwaita: format!("{}.{}", adw.0, adw.1),
        product_version: product,
        manifest_verified: verified,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_version_formats() {
        assert_eq!(version("GNOME Shell 51.2"), Some((51, 2)));
        assert_eq!(version("GNOME Shell 51.beta"), Some((51, 0)));
        assert_eq!(version("unavailable"), None);
    }
    #[test]
    fn manifest_rejects_missing_contract() {
        let t = tempfile::tempdir().unwrap();
        assert!(
            verify_manifest(
                t.path(),
                &serde_json::json!({"schema":1,"uuid":UUID,"artifacts":{}})
            )
            .is_err()
        );
    }
}
