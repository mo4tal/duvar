#!/usr/bin/env bash
# Real GNOME Shell integration in an isolated headless Wayland session.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
if [[ ${1:-} != --child ]]; then
    parent=${TMPDIR:-/tmp}
    [[ ! -d /tmp/opencode ]] || parent=/tmp/opencode
    t=$(mktemp -d "$parent/duvar-shell.XXXXXX")
    chmod 700 "$t"
    mkdir -p "$t/home" "$t/runtime" "$t/config" "$t/data/gnome-shell/extensions" "$t/cache"
    chmod 700 "$t/runtime"
    extensions="$t/data/gnome-shell/extensions"
    usertheme=user-theme@gnome-shell-extensions.gcampax.github.com
    blur=blur-my-shell@aunetx
    current=${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions
    [[ -d "$current/$usertheme" ]] || current=/usr/share/gnome-shell/extensions
    cp -r "$current/$usertheme" "$extensions/"
    real_data=${XDG_DATA_HOME:-$HOME/.local/share}
    if [[ -d "$real_data/gnome-shell/extensions/$blur" ]]; then
        cp -r "$real_data/gnome-shell/extensions/$blur" "$extensions/"
    fi
    ext="$extensions/duvar@mo4tal.local"
    mkdir -p "$ext/schemas"
    if [[ -n ${DUVAR_STORE_ZIP:-} ]]; then
        unzip -oq "$DUVAR_STORE_ZIP" -d "$ext/"
    elif [[ -n ${DUVAR_LOCAL_ZIP:-} ]]; then
        unzip -oq "$DUVAR_LOCAL_ZIP" -d "$ext/"
    else
        cp "$root"/extension/{metadata.json,extension.js,palette.js,palette-worker.js,user-theme-policy.js} "$ext/"
        cp "$root"/extension/schemas/*.xml "$ext/schemas/"
        if [[ -n ${DUVAR_PREFS_SOURCE:-} ]]; then
            cp "$root"/extension/{prefs.js,palette-prefs.js,gallery-wallpaper.js} "$ext/"
            cp -r "$DUVAR_PREFS_SOURCE"/{native,icons} "$ext/"
        fi
    fi
    if [[ -n ${DUVAR_PREFS_SOURCE:-}${DUVAR_STORE_ZIP:-}${DUVAR_LOCAL_ZIP:-} ]]; then
        cat >> "$ext/prefs.js" <<'JS'

import GLib from 'gi://GLib';
const fill = DuvarPreferences.prototype.fillPreferencesWindow;
DuvarPreferences.prototype.fillPreferencesWindow = async function (window) {
    await fill.call(this, window);
    const record = () => GLib.file_set_contents(
        `${GLib.get_user_cache_dir()}/prefs-page`, window.get_visible_page()?.get_title() ?? '');
    window.connect('notify::visible-page', record);
    record();
};
JS
    fi
    glib-compile-schemas --strict "$ext/schemas"
    probe="$extensions/palette-probe@duvar.test"
    mkdir -p "$probe"
    cp "$root/tests/palette-render-probe.js" "$probe/extension.js"
    cp "$root/tests/palette-perf-probe.js" "$probe/"
    printf '%s\n' '{"uuid":"palette-probe@duvar.test","name":"Duvar render probe","description":"Isolated test","shell-version":["51"]}' > "$probe/metadata.json"
    if [[ ${DUVAR_WINDOW_CHECK:-0} == 1 ]]; then
        weather=weatherpanel@attentivecoder
        cp -r "$real_data/gnome-shell/extensions/$weather" "$extensions/"
        cp -r "$real_data/gnome-shell/extensions/quick-settings-avatar@d-go" "$extensions/"
        dconf dump /org/gnome/shell/extensions/blur-my-shell/ > "$t/blur.ini"
        for key in picture-uri picture-uri-dark; do
            gsettings get org.gnome.desktop.background "$key" > "$t/$key"
        done
    fi
    if [[ ${DUVAR_PERF_CHECK:-0} == 1 ]]; then
        for key in picture-uri picture-uri-dark; do
            gsettings get org.gnome.desktop.background "$key" > "$t/$key"
        done
    fi
    echo "İzole günlük: $t/shell.log"
    bash_flags=()
    if [[ $- == *x* ]]; then bash_flags=(-x); fi
    exec timeout "$([[ ${DUVAR_WINDOW_CHECK:-0} == 1 ]] && echo 180s || echo 45s)" env -u DBUS_SESSION_BUS_ADDRESS -u DISPLAY -u WAYLAND_DISPLAY -u GSETTINGS_BACKEND -u GTK_USE_PORTAL \
        HOME="$t/home" XDG_RUNTIME_DIR="$t/runtime" XDG_CONFIG_HOME="$t/config" \
        XDG_DATA_HOME="$t/data" XDG_CACHE_HOME="$t/cache" \
        dbus-run-session -- bash "${bash_flags[@]}" "$0" --child "$t"
fi
t=$2
extensions="$XDG_DATA_HOME/gnome-shell/extensions"
uuid=duvar@mo4tal.local
user=user-theme@gnome-shell-extensions.gcampax.github.com
blur=blur-my-shell@aunetx
theme() {
    if [[ -d "$extensions/$user/schemas" ]]; then
        gsettings --schemadir "$extensions/$user/schemas" get org.gnome.shell.extensions.user-theme name
    else
        gsettings get org.gnome.shell.extensions.user-theme name
    fi
}
set_duvar() { gsettings --schemadir "$extensions/$uuid/schemas" set org.gnome.shell.extensions.duvar "$@"; }
get_duvar() { gsettings --schemadir "$extensions/$uuid/schemas" get org.gnome.shell.extensions.duvar "$@"; }
current_pair() {
    get_duvar palette-colors | python -c '
import ast,json,sys
p=json.loads(ast.literal_eval(sys.stdin.read()))
if p.get("revision") == int(sys.argv[1]) and p.get("wallpapers") == sys.argv[2:]:
    print("yes")
' "$revision" "$@"
}
revision=$(sed -n 's/^export const paletteRevision = \([0-9]*\);$/\1/p' "$root/extension/palette.js")
wait_for() {
    local value=$1; shift
    for ((i=0; i<${WAIT_ATTEMPTS:-60}; i++)); do
        if [[ $("$@" 2>/dev/null || :) == "$value" ]]; then return 0; fi
        sleep .2
    done
    echo "Expected $value from $*; got $("$@" || :)" >&2
    tail -50 "$t/shell.log" >&2
    return 1
}
enabled="['$uuid', 'palette-probe@duvar.test']"
if [[ -d "$extensions/$blur" ]]; then enabled="['$blur', '$uuid', 'palette-probe@duvar.test']"; fi
weather=weatherpanel@attentivecoder
if [[ -d "$extensions/$weather" ]]; then enabled="[ '$weather', ${enabled#\[}"; fi
avatar=quick-settings-avatar@d-go
if [[ -d "$extensions/$avatar" ]]; then enabled="[ '$avatar', ${enabled#\[}"; fi
gsettings set org.gnome.shell enabled-extensions "$enabled"
gsettings set org.gnome.shell disable-user-extensions false
gsettings set org.gnome.desktop.interface color-scheme default
gsettings set org.gnome.desktop.interface accent-color blue
if [[ -s "$t/blur.ini" ]]; then
    dconf load /org/gnome/shell/extensions/blur-my-shell/ < "$t/blur.ini"
fi
if [[ -d "$extensions/$avatar" ]]; then
    for key in avatar-username avatar-realname; do
        gsettings --schemadir "$extensions/$avatar/schemas" set org.gnome.shell.extensions.quick-settings-avatar "$key" true
    done
fi
old_theme=$(theme)
old_gtk=$(gsettings get org.gnome.desktop.interface gtk-theme)
mkdir -p "$XDG_CONFIG_HOME/gtk-4.0"
printf '/* Existing user CSS; keep exactly. */\n' > "$t/original.css"
cp "$t/original.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
set_duvar palette-colors '{"light":{"accent":"#784030","surface":"#faf5f4","text":"#191919"},"dark":{"accent":"#aabbee","surface":"#20222a","text":"#f5f5f5"}}'
set_duvar palette-follow false
gnome-shell --headless --wayland --no-x11 --virtual-monitor 1280x720 >"$t/shell.log" 2>&1 &
pid=$!
clients=()
trap 'kill "$pid" "${clients[@]}" 2>/dev/null || :' EXIT
status() { gsettings --schemadir "$extensions/$uuid/schemas" get org.gnome.shell.extensions.duvar palette-status; }
wait_for "'Kapalı'" status
gsettings set org.gnome.desktop.interface color-scheme prefer-dark
test "$(theme)" = "$old_theme"
gsettings set org.gnome.desktop.interface color-scheme default
test "$(get_duvar palette-ownership)" = "''"
if [[ -n ${DUVAR_STORE_ZIP:-} ]]; then
    for key in palette-enabled palette-adwaita palette-adwaita-refresh; do
        test "$(get_duvar "$key")" = false
    done
    set_duvar palette-enabled true
    wait_for "'Etkin · Shell ve deneysel GTK teması'" status
    test "$(theme)" = "$old_theme"
    ! gnome-extensions list --enabled | grep -Fx "$user"
    test -s "$XDG_DATA_HOME/themes/duvar-shell-direct-b.css" || test -s "$XDG_DATA_HOME/themes/duvar-shell-direct-a.css"
    gnome-extensions enable "$user"
    wait_for "'Duvar-Live-light'" theme
    test ! -e "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css"
    printf 'P3\n1 1\n255\n255 0 0\n' > "$t/store-light.ppm"
    printf 'P3\n1 1\n255\n0 0 255\n' > "$t/store-dark.ppm"
    for mode in light dark; do
        key=picture-uri
        [[ $mode != dark ]] || key=picture-uri-dark
        gsettings set org.gnome.desktop.background "$key" "file://$t/store-$mode.ppm"
    done
    set_duvar palette-follow true
    wait_for yes current_pair "file://$t/store-light.ppm" "file://$t/store-dark.ppm"
    socket=$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -type s -name 'wayland-*' -printf '%f\n' | head -n1)
    WAYLAND_DISPLAY="$socket" GDK_BACKEND=wayland GTK_A11Y=none \
        dbus-update-activation-environment WAYLAND_DISPLAY GDK_BACKEND GTK_A11Y
    gnome-extensions prefs "$uuid"
    wait_for 'Renkler' cat "$XDG_CACHE_HOME/prefs-page"
    set_duvar palette-enabled false
    wait_for "$old_theme" theme
    echo 'PASS: store ZIP uses manual User Themes, clean defaults, wallpaper following and native-free preferences'
    exit 0
fi
set_duvar palette-gtk true
set_duvar palette-adwaita true
gsettings set org.gnome.desktop.interface gtk-theme rewaita-guard-dark
printf '@import url("rewaita.css");\n' > "$t/rewaita.css"
cp "$t/rewaita.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
set_duvar palette-enabled true
wait_for "'Palet kaydedildi; Rewaita teması seçili olduğu için uygulanmadı.'" status
test "$(theme)" = "$old_theme"
test "$(gsettings get org.gnome.desktop.interface gtk-theme)" = "'rewaita-guard-dark'"
cmp "$t/rewaita.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
for mode in light dark; do
    test -s "$XDG_DATA_HOME/themes/Duvar-Live-$mode/gnome-shell/gnome-shell.css"
done
cp "$t/original.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
gsettings set org.gnome.desktop.interface gtk-theme "$old_gtk"
wait_for "'Duvar-Live-light'" theme
wait_for "'Duvar-GTK-a-light'" gsettings get org.gnome.desktop.interface gtk-theme
wait_for yes bash -c 'grep -q duvar-palette.css "$1" && echo yes' _ "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
capture() {
    rm -f -- "$XDG_CACHE_HOME/$1.done" "$XDG_CACHE_HOME/$1.error"
    printf '%s\n' "$*" > "$XDG_CACHE_HOME/capture"
    wait_for yes bash -c '[[ -f "$1" ]] && echo yes' _ "$XDG_CACHE_HOME/$1.done"
    test -s "$XDG_CACHE_HOME/$1.png"
    if [[ -s "$XDG_CACHE_HOME/$1.error" ]]; then
        printf 'FAIL: %s: %s\n' "$1" "$(<"$XDG_CACHE_HOME/$1.error")" >&2
        return 1
    fi
}
check_toggle_pixels() {
    # Only the quick-settings rows, not the GTK probe or the wallpaper.
    magick "$XDG_CACHE_HOME/$1.png" -gravity NorthEast -crop 405x142+18+105 +repage -depth 8 rgb:- |
        python -c 'import sys; b = sys.stdin.buffer.read(); c = bytes.fromhex(sys.argv[1]); n = sum(b[i:i+3] == c for i in range(0, len(b), 3)); print(f"{sys.argv[1]}: {n} toggle pixels"); sys.exit(n < 500)' "$2"
}
capture light
check_toggle_pixels light 784030
check_panel() {
    local line expected
    line=$(grep -E "Duvar panel $1 " "$t/shell.log" | tail -n 1)
    [[ -n $line ]]
    # Panel buttons must always use transparent border-box with inset box-shadow pills.
    [[ $line == *" active bg=#00000000 "* ]]
    if [[ $line == *" overview=yes "* ]]; then
        # Overview must use translucent white frosted glass, never opaque desktop pills.
        [[ $line == *" activities=#ffffff38 "* || $line == *" activities=#00000000 "* ]]
        [[ $line == *" active bg=#00000000 #ffffff38" ]]
    else
        expected=$(python -c '
import re, sys
css = open(sys.argv[1]).read()
m = re.search(r"^#panel \.panel-button\.duvar-system-menu:active, [^\n]*?box-shadow: inset 0 0 0 100px rgba\((\d+),(\d+),(\d+),([0-9.]+)\);", css, re.M)
assert m, "missing active panel pill rule"
r, g, b, a = int(m[1]), int(m[2]), int(m[3]), float(m[4])
print(f"#{r:02x}{g:02x}{b:02x}{round(a * 255):02x}")
' "$XDG_DATA_HOME/themes/Duvar-Live-light/gnome-shell/gnome-shell.css")
        [[ $expected =~ ^#[0-9a-f]{8}$ ]]
        [[ $line == *" active bg=#00000000 $expected" ]]
    fi
}
check_panel light
socket=$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -type s -name 'wayland-*' -printf '%f\n' | head -n1)
test -n "$socket"
if [[ -n ${DUVAR_PREFS_SOURCE:-}${DUVAR_LOCAL_ZIP:-} ]]; then
    WAYLAND_DISPLAY="$socket" GDK_BACKEND=wayland GTK_A11Y=none \
        dbus-update-activation-environment WAYLAND_DISPLAY GDK_BACKEND GTK_A11Y
fi
for kind in 3 4 adw; do
    env -u GTK_THEME GDK_BACKEND=wayland GTK_A11Y=none \
        gjs -m "$root/tests/palette-gtk-probe.js" "$kind" "$socket" >"$t/gtk-$kind.log" 2>&1 &
    clients+=("$!")
done
check_gtk() {
    local kind
    for kind in 3 4; do
        wait_for "$1" cat "$XDG_CACHE_HOME/gtk-$kind" || { cat "$t/gtk-$kind.log"; return 1; }
    done
}
check_gtk 'Duvar-GTK-a-light rgb(25,25,25) rgb(245,245,245)'
wait_for 'Adwaita-empty rgb(25,25,25) rgb(245,245,245)' cat "$XDG_CACHE_HOME/gtk-adw"
wait_for 'rgb(120,64,48)' cat "$XDG_CACHE_HOME/gtk-adw-accent"
test -s "$XDG_DATA_HOME/themes/Duvar-GTK-a-light/gtk-3.0/gtk.css"
test -s "$XDG_DATA_HOME/themes/Duvar-GTK-a-dark/gtk-4.0/gtk.css"
test -s "$XDG_DATA_HOME/themes/Duvar-Live-light/gnome-shell/gnome-shell.css"
test -s "$XDG_DATA_HOME/themes/Duvar-Live-dark/gnome-shell/gnome-shell.css"
if [[ ${DUVAR_PERF_CHECK:-0} == 1 ]]; then
    light_uri=$(python -c 'import ast,sys; print(ast.literal_eval(sys.stdin.read()))' < "$t/picture-uri")
    dark_uri=$(python -c 'import ast,sys; print(ast.literal_eval(sys.stdin.read()))' < "$t/picture-uri-dark")
    desktop=org.gnome.desktop.interface
    bg_schema=org.gnome.desktop.background
    arm() {
        printf '%s\n' "$1" > "$XDG_CACHE_HOME/arm-performance"
        wait_for "$1" cat "$XDG_CACHE_HOME/perf-armed"
    }
    result() {
        wait_for yes bash -c '[[ -s "$1" ]] && echo yes' _ "$XDG_CACHE_HOME/perf-$1.json"
    }
    arm idle
    result idle
    arm mode-dark
    gsettings set "$desktop" color-scheme prefer-dark
    wait_for "'Duvar-Live-dark'" theme
    result mode-dark
    arm mode-default
    gsettings set "$desktop" color-scheme default
    wait_for "'Duvar-Live-light'" theme
    result mode-default
    arm wallpaper-no-pulse
    gsettings set "$bg_schema" picture-uri "$light_uri"
    gsettings set "$bg_schema" picture-uri-dark "$dark_uri"
    set_duvar palette-follow true
    wait_for yes current_pair "$light_uri" "$dark_uri"
    result wallpaper-no-pulse
    pulse_light=$dark_uri
    pulse_dark=$light_uri
    if [[ $pulse_light == "$light_uri" && $pulse_dark == "$dark_uri" ]]; then
        pulse_rgb=$(python - "$XDG_DATA_HOME/themes/Duvar-Live-light/gnome-shell/gnome-shell.css" <<'PY'
import re, sys
css = open(sys.argv[1]).read()
m = re.search(r'\.quick-toggle:checked[^{]*\{ background-color: #([0-9a-f]{6})', css)
r, g, b = bytes.fromhex(m[1]) if m else (0, 0, 0)
print('220 40 40' if g > r and g > b else '0 180 60')
PY
)
        printf 'P3\n2 1\n255\n%s %s\n' "$pulse_rgb" "$pulse_rgb" > "$t/perf-pulse.ppm"
        pulse_light="file://$t/perf-pulse.ppm"
    fi
    set_duvar palette-adwaita-refresh true
    arm wallpaper-pulse
    gsettings set "$bg_schema" picture-uri "$pulse_light"
    gsettings set "$bg_schema" picture-uri-dark "$pulse_dark"
    wait_for yes current_pair "$pulse_light" "$pulse_dark"
    result wallpaper-pulse
    python "$root/tests/palette-perf-report.py" "$XDG_CACHE_HOME"
    exit 0
fi
started=$(date +%s%3N)
gsettings set org.gnome.desktop.interface color-scheme prefer-dark
wait_for "'Duvar-Live-dark'" theme
wait_for "'Duvar-GTK-a-dark'" gsettings get org.gnome.desktop.interface gtk-theme
echo "Açık → koyu ayar yakınsaması: $(($(date +%s%3N) - started)) ms (200 ms örnekleme; kare süresi değil)"
capture dark
check_toggle_pixels dark aabbee
check_gtk 'Duvar-GTK-a-dark rgb(245,245,245) rgb(25,25,25)'
wait_for 'Adwaita-empty rgb(245,245,245) rgb(25,25,25)' cat "$XDG_CACHE_HOME/gtk-adw"
wait_for 'rgb(170,187,238)' cat "$XDG_CACHE_HOME/gtk-adw-accent"
echo "Shell ekran görüntüleri: $XDG_CACHE_HOME/{light,dark}.png"
# GNOME Settings “Default” writes default, not prefer-light.
capture before-default quick-keep
gsettings set org.gnome.desktop.interface color-scheme default
wait_for "'Duvar-Live-light'" theme
wait_for "'Duvar-GTK-a-light'" gsettings get org.gnome.desktop.interface gtk-theme
capture after-default quick-existing
check_toggle_pixels after-default 784030
check_panel after-default
check_gtk 'Duvar-GTK-a-light rgb(25,25,25) rgb(245,245,245)'
wait_for 'Adwaita-empty rgb(25,25,25) rgb(245,245,245)' cat "$XDG_CACHE_HOME/gtk-adw"
wait_for 'rgb(120,64,48)' cat "$XDG_CACHE_HOME/gtk-adw-accent"
set_duvar palette-gtk false
wait_for "$old_gtk" gsettings get org.gnome.desktop.interface gtk-theme
wait_for "'Duvar-Live-light'" theme
set_duvar palette-gtk true
wait_for "'Duvar-GTK-a-light'" gsettings get org.gnome.desktop.interface gtk-theme
printf 'P3\n2 1\n255\n255 0 0 255 0 0\n' > "$t/red.ppm"
wallpaper="file://$t/red.ppm"
gsettings set org.gnome.desktop.background picture-uri "$wallpaper"
gsettings set org.gnome.desktop.background picture-uri-dark "$wallpaper"
set_duvar palette-follow true
wait_for "'Duvar-GTK-b-light'" gsettings get org.gnome.desktop.interface gtk-theme
gsettings --schemadir "$extensions/$uuid/schemas" get org.gnome.shell.extensions.duvar palette-colors | grep -Fq "$wallpaper"
light_css="$XDG_DATA_HOME/themes/Duvar-Live-light/gnome-shell/gnome-shell.css"
cp "$light_css" "$t/red-shell.css"
dark_css="$XDG_DATA_HOME/themes/Duvar-Live-dark/gnome-shell/gnome-shell.css"
cp "$dark_css" "$t/red-dark-shell.css"
printf 'P3\n2 1\n255\n0 255 0 0 255 0\n' > "$t/green.ppm"
gsettings set org.gnome.desktop.background picture-uri "file://$t/green.ppm"
wait_for "'Duvar-GTK-a-light'" gsettings get org.gnome.desktop.interface gtk-theme
! cmp -s "$t/red-shell.css" "$light_css"
cmp -s "$t/red-dark-shell.css" "$dark_css"
grep -Fq '.quick-toggle:checked' "$light_css"
accent_for() { sed -n 's/^\.quick-settings \.quick-toggle:checked, .*background-color: #\([0-9a-f]\{6\}\);.*/\1/p' "$1"; }
green_accent=$(accent_for "$light_css")
[[ $green_accent =~ ^[0-9a-f]{6}$ && $green_accent != 784030 ]]
capture light
check_toggle_pixels light "$green_accent"
check_gtk 'Duvar-GTK-a-light rgb(25,25,25) rgb(245,245,245)'
# The existing libadwaita process still holds its original user provider.
! grep -Fq -- '--accent-color: #784030' "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css"
wait_for 'rgb(120,64,48)' cat "$XDG_CACHE_HOME/gtk-adw-accent"
# A second wallpaper change must refresh the already running client without
# the test touching the accessibility setting.
set_duvar palette-adwaita-refresh true
printf 'P3\n2 1\n255\n0 0 255 0 0 255\n' > "$t/blue.ppm"
gsettings set org.gnome.desktop.background picture-uri "file://$t/blue.ppm"
wait_for "'Duvar-GTK-b-light'" gsettings get org.gnome.desktop.interface gtk-theme
wait_for true cat "$XDG_CACHE_HOME/gtk-adw-contrast"
wait_for false cat "$XDG_CACHE_HOME/gtk-adw-contrast"
test -z "$(dconf read /org/gnome/desktop/a11y/interface/high-contrast)"
env -u GTK_THEME GDK_BACKEND=wayland GTK_A11Y=none \
    gjs -m "$root/tests/palette-gtk-probe.js" adw "$socket" adw-fresh >"$t/gtk-adw-fresh.log" 2>&1 &
clients+=("$!")
accent=$(sed -n 's/.*--accent-color: #\([0-9a-f]\{6\}\);.*/\1/p' "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css" | head -n 1)
[[ $accent =~ ^[0-9a-f]{6}$ ]]
printf -v fresh 'rgb(%d,%d,%d)' "0x${accent:0:2}" "0x${accent:2:2}" "0x${accent:4:2}"
wait_for "$fresh" cat "$XDG_CACHE_HOME/gtk-adw-fresh-accent"
wait_for "$fresh" cat "$XDG_CACHE_HOME/gtk-adw-accent"
echo 'PASS: Duvar automatically refreshed the existing libadwaita process and restored high contrast'
for color in red green blue; do
    gsettings set org.gnome.desktop.background picture-uri "file://$t/$color.ppm"
    gsettings set org.gnome.desktop.background picture-uri-dark "file://$t/$color.ppm"
done
wait_for yes current_pair "file://$t/blue.ppm" "file://$t/blue.ppm"
wait_for false cat "$XDG_CACHE_HOME/gtk-adw-contrast"
echo 'PASS: rapid wallpaper updates converge on the final light/dark pair'
gsettings set org.gnome.desktop.a11y.interface high-contrast false
set_duvar palette-adwaita false
wait_for yes bash -c 'cmp -s "$1" "$2" && echo yes' _ "$t/original.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
wait_for yes bash -c '[[ -s "$1" && $(<"$1") != "$2" ]] && echo yes' _ "$XDG_CACHE_HOME/gtk-adw-accent" "$fresh"
wait_for false cat "$XDG_CACHE_HOME/gtk-adw-contrast"
test "$(dconf read /org/gnome/desktop/a11y/interface/high-contrast)" = false
set_duvar palette-adwaita true
wait_for "$fresh" cat "$XDG_CACHE_HOME/gtk-adw-accent"
wait_for false cat "$XDG_CACHE_HOME/gtk-adw-contrast"
set_duvar palette-enabled false
wait_for "$old_theme" theme
wait_for "$old_gtk" gsettings get org.gnome.desktop.interface gtk-theme
wait_for "'blue'" gsettings get org.gnome.desktop.interface accent-color
cmp "$t/original.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
wait_for false cat "$XDG_CACHE_HOME/gtk-adw-contrast"
# A real high-contrast selection must win over a palette activation.
gsettings set org.gnome.desktop.a11y.interface high-contrast true
wait_for true cat "$XDG_CACHE_HOME/gtk-adw-contrast"
set_duvar palette-enabled true
wait_for "'Yüksek kontrast açık; Duvar renkleri uygulanmadı.'" status
test "$(gsettings get org.gnome.desktop.a11y.interface high-contrast)" = true
cmp "$t/original.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
set_duvar palette-enabled false
wait_for "'Kapalı'" status
gsettings reset org.gnome.desktop.a11y.interface high-contrast
touch "$XDG_CACHE_HOME/arm-pulse-disable"
wait_for yes bash -c '[[ -f "$1" ]] && echo yes' _ "$XDG_CACHE_HOME/pulse-armed"
set_duvar palette-enabled true
wait_for 'true false' cat "$XDG_CACHE_HOME/pulse-disable"
wait_for "$old_theme" theme
wait_for "$old_gtk" gsettings get org.gnome.desktop.interface gtk-theme
test -z "$(dconf read /org/gnome/desktop/a11y/interface/high-contrast)"
cmp "$t/original.css" "$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
echo 'PASS: disabling Duvar during its refresh pulse restores accessibility and theme settings'
set_duvar palette-enabled false
gnome-extensions enable "$uuid"
wait_for "'Kapalı'" status
if [[ -d "$extensions/$blur" ]]; then
    gnome-extensions list --active | grep -Fx "$blur"
fi
echo 'PASS: automatic User Themes and wallpaper updates, Rewaita protection, restoration and live GTK3/GTK4/libadwaita mode switching'
theme_schema=org.gnome.shell.extensions.user-theme
set_theme() { gsettings --schemadir "$extensions/$user/schemas" set "$theme_schema" name "$1"; }
set_duvar palette-adwaita-refresh false
set_theme Duvar-dark
gsettings set org.gnome.desktop.interface color-scheme prefer-dark
set_duvar palette-ownership '{"theme":"Duvar-dark","accent":"blue","appliedTheme":"Duvar-dark","appliedAccent":"blue"}'
set_duvar palette-enabled true
wait_for "'Duvar-Live-dark'" theme
gsettings set org.gnome.desktop.interface color-scheme default
wait_for "'Duvar-Live-light'" theme
set_duvar palette-enabled false
wait_for "'Duvar-dark'" theme
set_duvar palette-enabled true
wait_for "'Duvar-Live-light'" theme
is_duvar_gtk_light() {
    case "$(gsettings get org.gnome.desktop.interface gtk-theme)" in
        "'Duvar-GTK-a-light'"|"'Duvar-GTK-b-light'") echo yes ;;
        *) echo no ;;
    esac
}
wait_for yes is_duvar_gtk_light
active_gtk=$(python -c 'import ast,sys; print(ast.literal_eval(sys.argv[1]))' "$(gsettings get org.gnome.desktop.interface gtk-theme)")
rm -rf "$XDG_DATA_HOME/themes/Duvar-Live-dark" \
    "$XDG_DATA_HOME/themes/Duvar-Live-light/gnome-shell/gnome-shell.css" \
    "$XDG_DATA_HOME/themes/duvar-blur-accent.css" \
    "$XDG_DATA_HOME/themes/$active_gtk" \
    "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css"
repaired_files() {
    local gtk
    gtk=$(python -c 'import ast,sys; print(ast.literal_eval(sys.argv[1]))' "$(gsettings get org.gnome.desktop.interface gtk-theme)")
    [[ -s "$XDG_DATA_HOME/themes/Duvar-Live-dark/gnome-shell/gnome-shell.css" ]] &&
    [[ -s "$XDG_DATA_HOME/themes/Duvar-Live-dark/gnome-shell/duvar-blur-accent.css" ]] &&
    [[ -s "$XDG_DATA_HOME/themes/Duvar-Live-light/gnome-shell/gnome-shell.css" ]] &&
    [[ -s "$XDG_DATA_HOME/themes/duvar-blur-accent.css" ]] &&
    [[ -s "$XDG_DATA_HOME/themes/$gtk/gtk-4.0/gtk.css" ]] &&
    [[ -s "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css" ]] && echo yes || echo no
}
wait_for yes repaired_files
rm -f "$XDG_DATA_HOME/themes/duvar-blur-accent.css"
wait_for yes repaired_files
echo 'PASS: automatic detection and self-repair of deleted Duvar theme files'
set_theme TestExternalTheme
wait_for false get_duvar palette-enabled
wait_for "''" get_duvar palette-ownership
test "$(theme)" = "'TestExternalTheme'"
wait_for "$old_gtk" gsettings get org.gnome.desktop.interface gtk-theme
set_theme "$(python -c 'import ast,sys; print(ast.literal_eval(sys.argv[1]))' "$old_theme")"
set_duvar palette-adwaita-refresh true
echo 'PASS: disabled mode, legacy Duvar-dark ownership/rollback and manual theme override'
if [[ ${DUVAR_WINDOW_CHECK:-0} == 1 ]]; then
    # Actual native and sandboxed app windows in the same compositor used above.
    kill "${clients[@]}" 2>/dev/null || :
    clients=()
    printf 'P3\n3 1\n255\n0 0 0 100 101 100 255 255 255\n' > "$t/neutral.ppm"
    WAYLAND_DISPLAY="$socket" GDK_BACKEND=wayland GTK_A11Y=none GSETTINGS_BACKEND=memory \
        timeout 20s gjs -m "$root/tests/palette-smoke.js" "$extensions/$uuid" "$t/neutral.ppm"
    touch "$XDG_CACHE_HOME/gallery"
    gallery_pair=()
    for key in picture-uri picture-uri-dark; do
        uri=$(cat "$t/$key")
        gsettings set org.gnome.desktop.background "$key" "$uri"
        gallery_pair+=("$(python -c 'import ast,sys; print(ast.literal_eval(sys.argv[1]))' "$uri")")
    done
    set_duvar palette-enabled true
    wait_for yes current_pair "${gallery_pair[@]}"
    wait_for "'Duvar-Live-light'" theme
    gnome-extensions list --active | grep -Fx "$weather"
    gsettings set org.gnome.desktop.notifications show-banners true
    gsettings set org.gnome.desktop.interface color-scheme prefer-dark
    wait_for "'Duvar-Live-dark'" theme
    for surface in date weather notification; do
        capture "surface-dark-$surface" "$surface"
    done
    capture surface-dark-quick quick-keep
    css_before=$(sha256sum "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css")
    gsettings set org.gnome.desktop.interface color-scheme default
    wait_for "'Duvar-Live-light'" theme
    capture surface-light-quick quick-existing
    test "$(sha256sum "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css")" = "$css_before"
    for surface in date weather notification; do
        capture "surface-light-$surface" "$surface"
    done
    python "$root/tests/palette-surface-check.py" "$t/shell.log" "$XDG_CACHE_HOME" "$XDG_DATA_HOME/themes"
    export WAYLAND_DISPLAY="$socket" GDK_BACKEND=wayland GTK_A11Y=none
    unset GTK_THEME
    for kind in 4 adw; do
        gjs -m "$root/tests/palette-gtk-probe.js" "$kind" "$socket" "overlap-$kind" >"$t/overlap-$kind.log" 2>&1 &
        clients+=("$!")
        wait_for yes bash -c '[[ -s "$1" ]] && echo yes' _ "$XDG_CACHE_HOME/gtk-overlap-$kind"
    done
    for mode in light dark; do
        scheme=default
        [[ $mode != dark ]] || scheme=prefer-dark
        gsettings set org.gnome.desktop.interface color-scheme "$scheme"
        wait_for "'Duvar-Live-$mode'" theme
        for kind in 4 adw; do
            capture "overlap-$mode-$kind" "overlap-$kind"
        done
    done
    python "$root/tests/palette-window-check.py" "$XDG_CACHE_HOME" "$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css"
    kill "${clients[@]}" 2>/dev/null || :
    clients=()
    gsettings set org.gnome.desktop.interface color-scheme default
    wait_for "'Duvar-Live-light'" theme
    printf 'Duvar — GTK4 / libadwaita / Flatpak\nWindow stays open while its palette changes.\n' > "$t/duvar.txt"
    selection_color() {
        magick "$XDG_CACHE_HOME/$1.png" -crop "240x18+$((x+20))+$((y+65))" +repage -depth 8 rgb:- |
            python -c 'from collections import Counter
import sys
b = sys.stdin.buffer.read()
rgb, n = Counter(zip(b[::3], b[1::3], b[2::3])).most_common(1)[0]
assert n >= 240, (rgb, n)
print(*rgb)'
    }
    failed=0
    for mode in light dark; do
        if [[ $mode == dark ]]; then
            gsettings set org.gnome.desktop.interface color-scheme prefer-dark
            wait_for "'Duvar-Live-dark'" theme
        fi
        apps=(calculator files settings flatpak ghostty)
        [[ ${DUVAR_MISSION_CENTER:-0} != 1 ]] || apps+=(mission-center)
        [[ -f "$extensions/$uuid/prefs.js" ]] && apps+=(duvar-prefs)
        for app in "${apps[@]}"; do
            case $app in
                ghostty) args=(env LIBGL_ALWAYS_SOFTWARE=1 /usr/bin/ghostty --gtk-single-instance=false --shell-integration=none -e /usr/bin/bash --noprofile --norc);;
                calculator) args=(gnome-calculator);;
                files) args=(nautilus --new-window "$t");;
                settings) args=(env XDG_CURRENT_DESKTOP=GNOME gnome-control-center background);;
                duvar-prefs)
                    rm -f "$XDG_CACHE_HOME/prefs-page"
                    args=(gnome-extensions prefs "$uuid");;
                mission-center) args=(flatpak run --socket=wayland "--filesystem=$XDG_CONFIG_HOME/gtk-4.0:ro"
                    --command=sh io.missioncenter.MissionCenter -ec 'export XDG_CONFIG_HOME="$1"; exec missioncenter' _ "$XDG_CONFIG_HOME");;
                flatpak) args=(flatpak run --socket=wayland "--filesystem=$XDG_CONFIG_HOME/gtk-4.0:ro" "--filesystem=$t/duvar.txt:ro"
                    --command=sh org.gnome.TextEditor -ec '
                        export XDG_CONFIG_HOME="$1"
                        grep -q duvar-palette.css "$1/gtk-4.0/gtk.css"
                        grep -q -- --accent-bg-color "$1/gtk-4.0/duvar-palette.css"
                        exec /app/bin/gnome-text-editor --standalone "$2"
                    ' _ "$XDG_CONFIG_HOME" "$t/duvar.txt");;
            esac
            instance_file="$t/$app-$mode.instance"
            if [[ $app == mission-center || $app == flatpak ]]; then
                "${args[@]:0:2}" --instance-id-fd=3 "${args[@]:2}" 3>"$instance_file" >"$t/$app-$mode.log" 2>&1 &
            else
                "${args[@]}" >"$t/$app-$mode.log" 2>&1 &
            fi
            clients=("$!")
            if wait_for yes bash -c '[[ -s "$1" && $(<"$1") != "-" ]] && echo yes' _ "$XDG_CACHE_HOME/window"; then
                capture "$app-$mode"
                # Initial map can precede the window's final placement.
                read -r x y w h < "$XDG_CACHE_HOME/window"
                case $app in
                    ghostty) capture "$app-menu-$mode" click "$((x+w-63))" "$((y+23))" 1;;
                    calculator)
                        # The 1280×720 gallery centers Calculator. Use the mode
                        # button seen in the window, not the compositor frame's
                        # top inset (which can include client-side decoration).
                        # Look for the contrasting menu labels in the otherwise
                        # empty display; blur noise alone is not a match.
                        menu_open=0
                        for attempt in 1 2; do
                            capture "$app-menu-$mode" click 642 91 1 || continue
                            if magick "$XDG_CACHE_HOME/$app-menu-$mode.png" -crop 200x175+540+128 +repage -depth 8 rgb:- |
                                python -c 'import sys
red = sys.stdin.buffer.read()[::3]
labels = sum(r < 110 for r in red) if sys.argv[1] == "light" else sum(r > 145 for r in red)
sys.exit(labels < 300)' "$mode"; then
                                menu_open=1
                                break
                            fi
                        done
                        if [[ $menu_open == 0 ]]; then
                            echo "FAIL: Calculator mode menu did not open ($mode; frame $x $y $w $h)" >&2
                            failed=1
                        else
                            echo "PASS: Calculator mode menu ($mode, attempt $attempt)"
                        fi;;
                    files)
                        capture "wallpaper-$mode-quick" quick
                        accent=$(accent_for "$XDG_DATA_HOME/themes/Duvar-Live-$mode/gnome-shell/gnome-shell.css")
                        check_toggle_pixels "wallpaper-$mode-quick" "$accent"
                        gsettings set org.gnome.desktop.notifications show-banners true
                        capture "wallpaper-$mode-notification" notification
                        banner=$(grep -F "Duvar notification wallpaper-$mode-notification " "$t/shell.log" | tail -1 || :)
                        if [[ $banner =~ bms-popup-(surface|background)-(light|dark|transparent) ]]; then
                            if ! expected=$(python - "$XDG_DATA_HOME/themes/Duvar-Live-$mode/gnome-shell/duvar-blur-accent.css" "$banner" <<'PY'
import re, sys
css, banner = open(sys.argv[1]).read(), sys.argv[2]
cls = re.search(r'\bbms-popup-surface-(?:light|dark|transparent)\b', banner)[0]
matches = re.findall(rf'\.{cls} \.message\.bms-popup-blurred(?![:\w-])[^\n]*?\{{ background-color: rgba\(([\d, ]+),\s*(0\.\d+)\)', css)
assert matches, (cls, banner)
rgb, alpha = matches[-1]
assert 0.50 <= float(alpha) <= 0.88, (cls, alpha)
actual = re.search(r' (#[0-9a-f]{8}) ', banner)[1]
exp_rgb = '#%02x%02x%02x' % tuple(map(int, rgb.split(',')))
assert actual[:7] == exp_rgb and abs(int(actual[7:], 16) - round(float(alpha) * 255)) <= 1, (actual, exp_rgb, alpha)
print(actual)
PY
); then
                                echo "FAIL: notification ($mode): $banner" >&2
                                failed=1
                            else
                                echo "PASS: notification ($mode) $expected"
                            fi
                        else
                            expected=$(sed -n 's/^\.popup-menu-content, .*\.notification-banner { background-color: \(#[0-9a-f]\{6\}\);.*/\1ff/p' "$XDG_DATA_HOME/themes/Duvar-Live-$mode/gnome-shell/gnome-shell.css")
                            if [[ -z $expected || $banner != *"wallpaper-$mode-notification $expected "* ]]; then
                                echo "FAIL: notification ($mode), expected $expected: $banner" >&2
                                failed=1
                            else
                                echo "PASS: notification ($mode) $expected"
                            fi
                        fi
                        capture "wallpaper-$mode-date" date
                        if [[ -s "$t/blur.ini" ]]; then
                            css="$XDG_DATA_HOME/themes/Duvar-Live-$mode/gnome-shell/duvar-blur-accent.css"
                            read -r date_base date_hover < <(python - "$css" <<'PY'
import re, sys
css = open(sys.argv[1]).read()
def parse(pat):
    m = re.search(pat, css)
    assert m, pat
    val = m.group(1)
    if val.startswith('#'):
        return val + 'ff'
    r, g, b, a = re.match(r'rgba\((\d+),(\d+),(\d+),(0\.\d+)\)', val).groups()
    lo = max(0, round(float(a) * 255) - 1)
    hi = min(255, round(float(a) * 255) + 1)
    return '#%02x%02x%02x(%s)' % (int(r), int(g), int(b), '|'.join(f'{x:02x}' for x in range(lo, hi + 1)))
print(
    parse(r'events-button \{ background-color: ([^; ]+)'),
    parse(r'events-button:hover \{ background-color: ([^; ]+)'),
)
PY
)
                            test "$(grep -Ec "Duvar date wallpaper-$mode-date .* ${date_base}$" "$t/shell.log")" -eq 3 || failed=1
                            for state in hover focus; do
                                capture "wallpaper-$mode-date-$state" "date-$state"
                                if [[ $state == hover ]]; then date_bg=$date_hover; else date_bg=$date_base; fi
                                if grep -Eq "Duvar date wallpaper-$mode-date-$state world-clocks-button ${date_bg}$" "$t/shell.log" &&
                                    { [[ $state != focus ]] ||
                                        { date_ring=$(grep -m1 -oP 'events-button:focus \{ box-shadow: inset 0 0 0 2px \K#[0-9a-f]{6}' "$css") &&
                                          grep -Fx "Duvar focus wallpaper-$mode-date-focus ${date_ring}ff true 2" < <(sed 's/^.*Duvar focus /Duvar focus /' "$t/shell.log"); }; }; then
                                    echo "PASS: Blur My Shell date card $state ($mode)"
                                else
                                    echo "FAIL: Blur My Shell date card $state ($mode)" >&2
                                    failed=1
                                fi
                            done
                        fi;;
                esac
                if [[ $app == duvar-prefs ]]; then
                    # Click the real tab; assert the visible page, not just the window.
                    read -r x y w h < "$XDG_CACHE_HOME/window"
                    capture "duvar-prefs-colors-$mode" click "$((x+w/2+253))" "$((y+24))" 1
                    wait_for 'Renkler' cat "$XDG_CACHE_HOME/prefs-page"
                    echo "PASS: Duvar preferences opened Renkler ($mode)"
                fi
                if [[ $app == flatpak ]]; then
                    if [[ $mode == light ]]; then
                        window_id=$(cat "$XDG_CACHE_HOME/window-id")
                        capture flatpak-selection-before select-line "$((x+13))" "$((y+69))"
                        before=$(selection_color flatpak-selection-before)
                        touch "$XDG_CACHE_HOME/arm-pulse-frame"
                        gsettings set org.gnome.desktop.background picture-uri "file://$t/blue.ppm"
                        wait_for yes current_pair "file://$t/blue.ppm" "${gallery_pair[1]}"
                        wait_for yes bash -c '[[ -f "$1" ]] && echo yes' _ "$XDG_CACHE_HOME/flatpak-pulse.done"
                        wait_for false gsettings get org.gnome.desktop.a11y.interface high-contrast
                        capture flatpak-selection-after select-line "$((x+13))" "$((y+69))"
                        test "$(<"$XDG_CACHE_HOME/window-id")" = "$window_id"
                        after=$(selection_color flatpak-selection-after)
                        test "$before" != "$after"
                        echo "PASS: Flatpak Text Editor window $window_id selection RGB $before → $after"
                        gsettings set org.gnome.desktop.background picture-uri "${gallery_pair[0]}"
                        wait_for yes current_pair "${gallery_pair[@]}"
                    fi
                fi
                if [[ $app == duvar-prefs ]]; then
                    cp "$XDG_CACHE_HOME/window-id" "$XDG_CACHE_HOME/close-window"
                    wait_for '-' cat "$XDG_CACHE_HOME/window"
                fi
            else
                failed=1
                echo "Pencere eşlenmedi: $app ($mode) — $t/$app-$mode.log" >&2
            fi
            if [[ -s $instance_file ]]; then
                instance=$(<"$instance_file")
                [[ $instance =~ ^[0-9]+$ ]]
                flatpak kill "$instance"
            fi
            kill "${clients[@]}" 2>/dev/null || :
            wait "${clients[@]}" 2>/dev/null || :
            clients=()
            wait_for '-' cat "$XDG_CACHE_HOME/window"
        done
    done
    # The real wallpaper pair must also restore the light palette with Quick
    # Settings open. Refresh remains enabled, but a mode-only change must not
    # rewrite either palette stylesheet.
    companion="$XDG_CONFIG_HOME/gtk-4.0/duvar-palette.css"
    wait_for yes current_pair "${gallery_pair[@]}"
    wait_for false gsettings get org.gnome.desktop.a11y.interface high-contrast
    before_stamp=$(stat -c '%i %y' "$companion")
    capture wallpaper-before-default quick-keep
    start=$(date +%s%N)
    gsettings set org.gnome.desktop.interface color-scheme default
    wait_for "'Duvar-Live-light'" theme
    wait_for yes bash -c '[[ $(gsettings get org.gnome.desktop.interface gtk-theme) =~ Duvar-GTK-[ab]-light ]] && echo yes'
    echo "Gerçek çift koyu → Varsayılan ayar yakınsaması: $((($(date +%s%N) - start) / 1000000)) ms (200 ms örnekleme; kare süresi değil)"
    capture wallpaper-after-default quick-existing
    light_accent=$(accent_for "$light_css")
    check_toggle_pixels wallpaper-after-default "$light_accent"
    test "$before_stamp" = "$(stat -c '%i %y' "$companion")"
    wait_for yes current_pair "${gallery_pair[@]}"
    echo 'PASS: real wallpaper pair switches to Default without closing Quick Settings or rewriting GTK4 CSS'
    echo "Gerçek pencere görüntüleri: $XDG_CACHE_HOME"
    exit "$failed"
fi
