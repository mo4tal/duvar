#!/usr/bin/env bash
# Duvar — Recommended GNOME Desktop Setup (Companion Extensions + Dynamic Presets)
# Installs & harmonizes:
#   1. Duvar (duvar@mo4tal.github.io)
#   2. Blur my Shell (blur-my-shell@aunetx)
#   3. User Themes (user-theme@gnome-shell-extensions.gcampax.github.com)
#   4. User Avatar In Quick Settings (quick-settings-avatar@d-go)
#   5. Rounded Window Corners Native (rounded-window-corners@ztrahmet.github.io)
#
# Safe by design: no sudo, no hardcoded paths/usernames/tokens, automatic dconf backup & rollback.

set -euo pipefail

REPO="mo4tal/duvar"
DUVAR_UUID="duvar@mo4tal.github.io"
COMPANION_UUIDS=(
    "blur-my-shell@aunetx"
    "user-theme@gnome-shell-extensions.gcampax.github.com"
    "quick-settings-avatar@d-go"
    "rounded-window-corners@ztrahmet.github.io"
)

DRY_RUN=0
ONLY_PRESETS=0
SKIP_DUVAR=0
ASSUME_YES=0
DUVAR_ZIP=""
ROLLBACK_DIR=""

usage() {
    cat <<'EOF'
Duvar — Recommended GNOME Desktop Setup

Usage:
  bash setup-recommended-desktop.sh [OPTIONS]

Options:
  --dry-run             Show every action without modifying files or dconf
  --only-presets        Skip downloads/installs; apply harmonized presets only
  --skip-duvar          Install & configure companion extensions only
  --duvar-zip PATH      Install Duvar from a local .shell-extension.zip file
  --rollback DIR        Restore a previous dconf backup created by this script
  -y, --yes             Proceed without interactive confirmation
  -h, --help            Show this help message

No sudo required. Existing dconf settings are backed up automatically before any change.
EOF
}

while (($# > 0)); do
    case "$1" in
        --dry-run) DRY_RUN=1; shift ;;
        --only-presets) ONLY_PRESETS=1; shift ;;
        --skip-duvar) SKIP_DUVAR=1; shift ;;
        --duvar-zip) DUVAR_ZIP=${2:?--duvar-zip requires a path}; shift 2 ;;
        --rollback) ROLLBACK_DIR=${2:?--rollback requires a backup directory}; shift 2 ;;
        -y|--yes) ASSUME_YES=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
done

if (( EUID == 0 )); then
    echo "Please run this script as your normal desktop user (no sudo)." >&2
    exit 2
fi

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Missing required command: $1" >&2
        exit 1
    }
}

need_cmd python3
need_cmd dconf
need_cmd gsettings

if [[ -n "$ROLLBACK_DIR" ]]; then
    [[ -d "$ROLLBACK_DIR" ]] || { echo "Backup directory not found: $ROLLBACK_DIR" >&2; exit 1; }
    echo "Restoring dconf settings from: $ROLLBACK_DIR"
    for pair in \
        "blur-my-shell.dconf:/org/gnome/shell/extensions/blur-my-shell/" \
        "quick-settings-avatar.dconf:/org/gnome/shell/extensions/quick-settings-avatar/" \
        "rounded-window-corners-native.dconf:/org/gnome/shell/extensions/rounded-window-corners-native/" \
        "duvar.dconf:/org/gnome/shell/extensions/duvar/"; do
        file="${pair%%:*}"
        dpath="${pair#*:}"
        if [[ -f "$ROLLBACK_DIR/$file" ]]; then
            if (( DRY_RUN )); then
                echo "[dry-run] dconf reset -f $dpath && dconf load $dpath < $ROLLBACK_DIR/$file"
            else
                dconf reset -f "$dpath"
                if [[ -s "$ROLLBACK_DIR/$file" ]]; then
                    dconf load "$dpath" < "$ROLLBACK_DIR/$file"
                fi
            fi
        fi
    done
    if [[ -s "$ROLLBACK_DIR/enabled-extensions.txt" ]]; then
        saved_exts=$(cat "$ROLLBACK_DIR/enabled-extensions.txt")
        if (( DRY_RUN )); then
            echo "[dry-run] gsettings set org.gnome.shell enabled-extensions $saved_exts"
        else
            gsettings set org.gnome.shell enabled-extensions "$saved_exts"
        fi
    fi
    echo "Rollback complete."
    exit 0
fi

need_cmd gnome-shell
need_cmd gnome-extensions
need_cmd unzip
need_cmd curl
need_cmd glib-compile-schemas

SHELL_RAW=$(gnome-shell --version)
SHELL_MAJOR=$(python3 -c 'import re,sys; m=re.search(r"\b(\d+)", sys.argv[1]); print(m.group(1) if m else "")' "$SHELL_RAW")
ARCH=$(uname -m)
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
STATE_HOME="${XDG_STATE_HOME:-$HOME/.local/state}"
EXT_DIR="$DATA_HOME/gnome-shell/extensions"

if [[ -z "$SHELL_MAJOR" ]]; then
    echo "Could not determine GNOME Shell version from: $SHELL_RAW" >&2
    exit 1
fi

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║          Duvar — Recommended GNOME Desktop Setup            ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo "  Detected : $SHELL_RAW ($ARCH)"
echo "  Target   : $EXT_DIR"
echo "  Bundle   : Duvar + Blur my Shell + User Themes +"
echo "             User Avatar In Quick Settings + Rounded Window Corners Native"
echo

if (( ! ASSUME_YES && ! DRY_RUN )); then
    read -r -p "Install/update missing extensions and apply harmonized presets? [y/N] " reply
    [[ "$reply" =~ ^[YyEe]$ ]] || { echo "Aborted."; exit 0; }
fi

install_ego_extension() {
    local uuid="$1"
    if [[ -d "$EXT_DIR/$uuid" || -d "/usr/share/gnome-shell/extensions/$uuid" ]]; then
        echo "  ✓ Already installed: $uuid"
        return 0
    fi
    echo "  → Fetching metadata from extensions.gnome.org: $uuid (GNOME $SHELL_MAJOR)"
    local download_path
    download_path=$(python3 - "$uuid" "$SHELL_MAJOR" <<'PY'
import json, sys, urllib.parse, urllib.request
uuid, major = sys.argv[1], sys.argv[2]
url = f"https://extensions.gnome.org/extension-info/?uuid={urllib.parse.quote(uuid)}&shell_version={urllib.parse.quote(major)}"
req = urllib.request.Request(url, headers={"User-Agent": "Duvar-Desktop-Setup/0.6.0 (+https://github.com/mo4tal/duvar)"})
try:
    with urllib.request.urlopen(req, timeout=15) as resp:
        info = json.loads(resp.read().decode("utf-8"))
except Exception as exc:
    print(f"ERROR:{exc}")
    sys.exit(0)
dl = info.get("download_url")
if not dl:
    print("UNSUPPORTED")
else:
    print("https://extensions.gnome.org" + dl)
PY
)
    if [[ "$download_path" == UNSUPPORTED || "$download_path" == ERROR:* ]]; then
        echo "  ! Skipping $uuid ($download_path — check GNOME $SHELL_MAJOR compatibility on EGO)" >&2
        return 0
    fi
    if (( DRY_RUN )); then
        echo "  [dry-run] Would download & install $uuid from $download_path"
        return 0
    fi
    local tmp_zip
    tmp_zip=$(mktemp --suffix=".zip")
    curl -fsSL --proto '=https' --tlsv1.2 --max-time 45 "$download_path" -o "$tmp_zip"
    gnome-extensions install --force "$tmp_zip"
    rm -f "$tmp_zip"
    echo "  ✓ Installed: $uuid"
}

install_duvar() {
    if (( SKIP_DUVAR )); then
        echo "  · Skipping Duvar install (--skip-duvar)"
        return 0
    fi
    if [[ -n "$DUVAR_ZIP" ]]; then
        if (( DRY_RUN )); then
            echo "  [dry-run] Would install Duvar from local ZIP: $DUVAR_ZIP"
            return 0
        fi
        gnome-extensions install --force "$DUVAR_ZIP"
        echo "  ✓ Installed Duvar from $DUVAR_ZIP"
        return 0
    fi
    if [[ -d "$EXT_DIR/$DUVAR_UUID" ]]; then
        echo "  ✓ Already installed: $DUVAR_UUID"
        return 0
    fi
    echo "  → Querying latest GitHub Release for $REPO ($ARCH)…"
    local release_info
    release_info=$(python3 - "$REPO" "$ARCH" <<'PY'
import json, sys, urllib.request
repo, arch = sys.argv[1], sys.argv[2]
url = f"https://api.github.com/repos/{repo}/releases/latest"
req = urllib.request.Request(url, headers={"User-Agent": "Duvar-Desktop-Setup/0.6.0 (+https://github.com/mo4tal/duvar)"})
with urllib.request.urlopen(req, timeout=15) as resp:
    rel = json.loads(resp.read().decode("utf-8"))
assets = {a["name"]: a["browser_download_url"] for a in rel.get("assets", [])}
tar_match = next((n for n in assets if n.endswith(f"-linux-{arch}.tar.gz")), None)
if tar_match:
    print("TAR|" + tar_match + "|" + assets[tar_match])
else:
    pure_match = next((n for n in assets if "pure-js-store" in n and n.endswith(".zip")), None)
    if pure_match:
        print("ZIP|" + pure_match + "|" + assets[pure_match])
    else:
        print("NONE||")
PY
)
    local kind="${release_info%%|*}"
    local rest="${release_info#*|}"
    local asset_name="${rest%%|*}"
    local asset_url="${rest#*|}"
    if [[ "$kind" == "NONE" || -z "$asset_url" ]]; then
        echo "  ! Could not locate a matching release asset for $ARCH." >&2
        return 1
    fi
    if (( DRY_RUN )); then
        echo "  [dry-run] Would download & install Duvar ($asset_name) from $asset_url"
        return 0
    fi
    local workdir
    workdir=$(mktemp -d)
    trap 'rm -rf -- "$workdir"' RETURN
    curl -fsSL --proto '=https' --tlsv1.2 --max-time 90 "$asset_url" -o "$workdir/$asset_name"
    if [[ "$kind" == "TAR" ]]; then
        tar -xzf "$workdir/$asset_name" -C "$workdir"
        local bundle_dir
        bundle_dir=$(find "$workdir" -mindepth 1 -maxdepth 1 -type d | head -n 1)
        (cd "$bundle_dir" && ./duvar-install)
    else
        gnome-extensions install --force "$workdir/$asset_name"
    fi
    echo "  ✓ Installed Duvar ($asset_name)"
}

if (( ! ONLY_PRESETS )); then
    echo "1) Installing extensions…"
    install_duvar
    for uuid in "${COMPANION_UUIDS[@]}"; do
        install_ego_extension "$uuid"
    done
    echo
fi

# 2) Backup existing dconf settings before writing anything
BACKUP_DIR="$STATE_HOME/duvar/desktop-preset-backup-$(date +%Y%m%d-%H%M%S)"
echo "2) Backing up current dconf settings → $BACKUP_DIR"
if (( ! DRY_RUN )); then
    mkdir -p "$BACKUP_DIR"
    dconf dump /org/gnome/shell/extensions/blur-my-shell/ > "$BACKUP_DIR/blur-my-shell.dconf" || true
    dconf dump /org/gnome/shell/extensions/quick-settings-avatar/ > "$BACKUP_DIR/quick-settings-avatar.dconf" || true
    dconf dump /org/gnome/shell/extensions/rounded-window-corners-native/ > "$BACKUP_DIR/rounded-window-corners-native.dconf" || true
    dconf dump /org/gnome/shell/extensions/duvar/ > "$BACKUP_DIR/duvar.dconf" || true
    gsettings get org.gnome.shell enabled-extensions > "$BACKUP_DIR/enabled-extensions.txt" || true
fi

# 3) Apply harmonized, portable, dynamic presets (ZERO personal data, paths, or tokens)
echo "3) Applying harmonized dynamic presets…"
if (( DRY_RUN )); then
    echo "  [dry-run] Would configure Blur my Shell (dynamic dual-kawase blur, gradient panel, blurred popups & rounded app blur)"
    echo "  [dry-run] Would configure User Avatar In Quick Settings (clean avatar header, size 60, frameless)"
    echo "  [dry-run] Would configure Rounded Window Corners Native (dynamic libadwaita + gtk.css tracking)"
    echo "  [dry-run] Would configure Duvar (adaptive OKLab palette + balanced floating glass capsules + GTK/Adwaita layers)"
else
    # Blur my Shell — tuned to complement Duvar's floating glass top-bar capsules & Shell menus
    dconf load /org/gnome/shell/extensions/blur-my-shell/ <<'DCONF'
[/]
hacks-level=2
power-saver=false
pipelines={'pipeline_default': {'name': <'Default'>, 'effects': <[<{'type': <'dual_kawase_blur'>, 'id': <'effect_000000000000'>, 'params': <{'unscaled_radius': <30>, 'brightness': <0.6>}>}>]>}, 'pipeline_duvar_dynamic': {'name': <'Duvar Dynamic Blur'>, 'effects': <[<{'type': <'dual_kawase_blur'>, 'id': <'effect_duvar_dynamic'>, 'params': <{'unscaled_radius': <50>, 'brightness': <0.8>}>}>]>}, 'pipeline_default_light': {'name': <'Default (Light mode)'>, 'effects': <[<{'type': <'dual_kawase_blur'>, 'id': <'effect_default_light_blur'>, 'params': <{'unscaled_radius': <30>, 'brightness': <1>}>}>, <{'type': <'color'>, 'id': <'effect_default_light_color'>, 'params': <{'color': <(1.0, 1.0, 1.0, 0.5)>, 'blend_mode': <3>}>}>]>}}

[appfolder]
pipeline='pipeline_duvar_dynamic'
pipeline-auto-mode=true
pipeline-dark='pipeline_duvar_dynamic'
pipeline-light='pipeline_duvar_dynamic'
style-dialogs=4

[applications]
blacklist=['Plank', 'com.desktop.ding', 'Conky']
blur=true
blur-menus=true
blur-on-overview=true
corner-radius=25
corner-when-maximized=false
dynamic-opacity=false
enable-all=true
menu-corner-radius=25
opacity=235
pipeline='pipeline_duvar_dynamic'
pipeline-auto-mode=true
pipeline-dark='pipeline_duvar_dynamic'
pipeline-light='pipeline_duvar_dynamic'
unblur-when-fullscreen=true
whitelist=@as []

[overview]
pipeline='pipeline_default'
pipeline-auto-mode=true
pipeline-dark='pipeline_duvar_dynamic'
pipeline-light='pipeline_duvar_dynamic'
style-components=4

[panel]
blur=true
corner-radius=0
force-light-text=false
gradient-panel=true
gradient-panel-mode=0
override-background=true
override-background-dynamically=true
override-background-dynamically-mode=0
pipeline='pipeline_duvar_dynamic'
pipeline-auto-mode=false
pipeline-dark='pipeline_default'
pipeline-light='pipeline_default_light'
static-blur=false
style-panel=0
unblur-in-overview=true

[popup]
blur=true
blur-behind-dialogs=true
notification-corner-radius=16
osd-window-corner-radius=999
override-background=true
pipeline='pipeline_duvar_dynamic'
pipeline-auto-mode=true
pipeline-dark='pipeline_duvar_dynamic'
pipeline-light='pipeline_duvar_dynamic'
preserve-shell-theme=true
resize-popup-corner-radius=999
screenshot-corner-radius=36
style-popup=3
submenu-corner-radius=24

[lockscreen]
pipeline='pipeline_duvar_dynamic'
pipeline-dark='pipeline_default'
pipeline-light='pipeline_default_light'

[screenshot]
pipeline='pipeline_duvar_dynamic'
pipeline-dark='pipeline_default'
pipeline-light='pipeline_default_light'
DCONF

    # User Avatar In Quick Settings — clean frameless avatar block adapting to the logged-in user
    dconf load /org/gnome/shell/extensions/quick-settings-avatar/ <<'DCONF'
[/]
avatar-hostname=true
avatar-nobackground=true
avatar-position=0
avatar-realname=true
avatar-size=60
avatar-username=true
DCONF

    # Rounded Window Corners Native — dynamically follows Libadwaita + ~/.config/gtk-4.0/gtk.css
    dconf load /org/gnome/shell/extensions/rounded-window-corners-native/ <<'DCONF'
[/]
custom-style=false
round-libadwaita-apps=false
round-libhandy-apps=true
round-maximized=false
use-allowlist=false
DCONF

    # Duvar — enable adaptive OKLab engine, balanced floating glass capsules & GTK/Adwaita sync
    if [[ -d "$EXT_DIR/$DUVAR_UUID/schemas" ]]; then
        glib-compile-schemas "$EXT_DIR/$DUVAR_UUID/schemas" 2>/dev/null || true
        gsettings --schemadir "$EXT_DIR/$DUVAR_UUID/schemas" set org.gnome.shell.extensions.duvar palette-follow true
        gsettings --schemadir "$EXT_DIR/$DUVAR_UUID/schemas" set org.gnome.shell.extensions.duvar palette-gtk true
        gsettings --schemadir "$EXT_DIR/$DUVAR_UUID/schemas" set org.gnome.shell.extensions.duvar palette-adwaita true
        gsettings --schemadir "$EXT_DIR/$DUVAR_UUID/schemas" set org.gnome.shell.extensions.duvar palette-adwaita-refresh true
        gsettings --schemadir "$EXT_DIR/$DUVAR_UUID/schemas" set org.gnome.shell.extensions.duvar palette-panel-style 'capsule'
        gsettings --schemadir "$EXT_DIR/$DUVAR_UUID/schemas" set org.gnome.shell.extensions.duvar palette-panel-capsule-strength 'balanced'
        gsettings --schemadir "$EXT_DIR/$DUVAR_UUID/schemas" set org.gnome.shell.extensions.duvar palette-enabled true
    fi
fi

# 4) Enable installed bundle extensions without touching any of the user's existing extensions
echo "4) Enabling installed bundle extensions (preserving existing extensions)…"
if (( DRY_RUN )); then
    echo "  [dry-run] Would append installed UUIDs to org.gnome.shell enabled-extensions"
else
    python3 - "$EXT_DIR" "$DUVAR_UUID" "${COMPANION_UUIDS[@]}" <<'PY'
import ast, pathlib, subprocess, sys
ext_dir = pathlib.Path(sys.argv[1])
candidates = sys.argv[2:]
raw = subprocess.check_output(["gsettings", "get", "org.gnome.shell", "enabled-extensions"], text=True).strip()
try:
    current = list(ast.literal_eval(raw))
except Exception:
    current = []
added = []
for uuid in candidates:
    if (ext_dir / uuid).is_dir() or (pathlib.Path("/usr/share/gnome-shell/extensions") / uuid).is_dir():
        if uuid not in current:
            current.append(uuid)
            added.append(uuid)
subprocess.run(["gsettings", "set", "org.gnome.shell", "enabled-extensions", repr(current)], check=True)
print("  ✓ Enabled:", ", ".join(added) if added else "all already enabled")
PY
fi

echo
echo "Done! Everything is configured cleanly."
echo "  • Rollback anytime : bash $0 --rollback '$BACKUP_DIR'"
echo "  • Next step        : Log out and back in once so GNOME Shell loads any newly installed extensions,"
echo "                       then open Extensions → Duvar → Settings gear."
