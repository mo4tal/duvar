#!/usr/bin/env bash
# Run as the desktop user; only an explicitly requested package install elevates.
set -euo pipefail
uuid=user-theme@gnome-shell-extensions.gcampax.github.com
mode=${1:---check}
case "$mode" in --check|--enable|--install) ;; *) echo "Kullanım: bash $0 [--check|--enable|--install]" >&2; exit 2;; esac
if (( EUID == 0 )); then
    echo 'Bu betiği masaüstü kullanıcısı olarak çalıştır.' >&2; exit 2
fi
command -v gnome-shell >/dev/null && command -v gnome-extensions >/dev/null ||
    { echo 'GNOME Shell ve gnome-extensions gerekiyor.' >&2; exit 1; }
gnome-shell --version
if ! gnome-shell --version | grep -Eq '(^| )51([. ]|$)'; then
    echo 'Bu Duvar sürümü GNOME Shell 51 için paketlenmiştir.' >&2; exit 1
fi
declare -a packages=()
if ! command -v gjs >/dev/null; then
    packages+=(gjs gdk-pixbuf2)
elif ! gjs -c 'imports.gi.GdkPixbuf.Pixbuf;' >/dev/null 2>&1; then
    packages+=(gdk-pixbuf2)
fi
listed() { gnome-extensions list "$@" | grep -Fx "$uuid" >/dev/null; }
theme_on_disk() {
    local dir
    local -a roots=("${XDG_DATA_HOME:-$HOME/.local/share}")
    IFS=: read -ra dirs <<< "${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
    roots+=("${dirs[@]}")
    for dir in "${roots[@]}"; do
        [[ -r "$dir/gnome-shell/extensions/$uuid/metadata.json" ]] && return 0
    done
    return 1
}
if ! listed && ! theme_on_disk; then packages+=(gnome-shell-extensions); fi
if ((${#packages[@]})); then
    echo "İsteğe bağlı/eksik bileşenler: ${packages[*]}"
    echo "Not: User Themes olmadan da Duvar doğrudan Shell stilini uygulayabilir."
    if [[ $mode != --install ]]; then
        echo "Arch tabanlı sistemlerde (pacman) kurmak için: bash \"$0\" --install"; exit 1
    fi
    command -v pacman >/dev/null && command -v pkexec >/dev/null ||
        { echo 'Otomatik kurulum yalnızca pacman + Polkit bulunan sistemlerde desteklenir; diğer dağıtımlarda paket yöneticinle kur.' >&2; exit 1; }
    pkexec /usr/bin/pacman -S --needed --noconfirm -- "${packages[@]}"
fi
if ! listed; then
    echo 'User Themes dosyaları var ama mevcut Shell oturumu henüz görmüyor. İlk kurulumdan sonra yeniden oturum açmak gerekebilir.' >&2
    exit 1
fi
if ! listed --enabled; then
    if [[ $mode == --check ]]; then
        echo "User Themes kurulu ama kapalı. Etkinleştirmek için: bash \"$0\" --enable"; exit 1
    fi
    gnome-extensions enable "$uuid"
fi
if command -v systemctl >/dev/null && systemctl --user is-active --quiet rewaita-guard.service; then
    echo 'Rewaita Guard çalışıyor. Birlikte tema yazmalarını önlemek için Duvar temasını henüz açma.'
fi
echo 'Bağımlılıklar hazır. Blur My Shell isteğe bağlıdır ve ayarlarına dokunulmadı.'
