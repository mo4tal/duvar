export function requestUserTheme(manager, uuid) {
    return manager.enableExtension(uuid);
}

export const missingUserTheme =
    'User Themes etkin değil. Kurulum aracını çalıştır veya Uzantılar’dan etkinleştir.';
