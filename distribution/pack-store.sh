#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
out="$root/dist/store"
mkdir -p "$out"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/schemas" "$stage/icons/hicolor/scalable/apps" "$stage/icons/hicolor/symbolic/apps"
cp "$root"/extension/{extension.js,palette.js,palette-worker.js,palette-prefs.js} "$stage/"
cp "$root"/extension/schemas/*.xml "$stage/schemas/"
cp "$root"/extension/icons/hicolor/index.theme "$stage/icons/hicolor/"
cp "$root"/extension/icons/hicolor/scalable/apps/local.mo4tal.Duvar.svg "$stage/icons/hicolor/scalable/apps/"
cp "$root"/extension/icons/hicolor/symbolic/apps/local.mo4tal.Duvar-symbolic.svg "$stage/icons/hicolor/symbolic/apps/"
cp "$root"/distribution/store/{prefs.js,user-theme-policy.js} "$stage/"
cp "$root/LICENSE" "$stage/"
python - "$root/extension/metadata.json" "$stage/metadata.json" <<'PY'
import json, sys
from pathlib import Path

metadata = json.loads(Path(sys.argv[1]).read_text())
metadata.pop('version', None)
metadata.pop('version-name', None)
metadata['description'] = (
    'Açık ve koyu duvar kâğıtlarından dinamik kontrastlı GNOME Shell, GTK ve libadwaita renk paletleri üretir.\n'
    'User Themes olmadan da doğrudan Shell stilini uygular; User Themes açıksa onunla uyumlu çalışır. '
    'Duvar galerisi bu pakete dahil değildir.'
)
Path(sys.argv[2]).write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + '\n')
PY
glib-compile-schemas --strict --dry-run "$stage/schemas"
gnome-extensions pack --force --out-dir "$out" \
    --extra-source=LICENSE --extra-source=icons --extra-source=palette.js --extra-source=palette-worker.js \
    --extra-source=palette-prefs.js --extra-source=user-theme-policy.js "$stage"
printf '%s\n' "$out/duvar@mo4tal.local.shell-extension.zip"
