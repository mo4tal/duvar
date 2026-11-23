import GLib from 'gi://GLib';

export const SUPPORTED_LANGUAGES = [
    'tr', 'en', 'es', 'de', 'fr', 'pt', 'ru', 'ja', 'zh', 'ar',
];

const cache = new Map();

function loadBundle(extPath, code) {
    const key = `${extPath}:${code}`;
    if (cache.has(key)) return cache.get(key);
    try {
        const [ok, bytes] = GLib.file_get_contents(`${extPath}/locales/${code}.json`);
        if (ok) {
            const parsed = JSON.parse(new TextDecoder().decode(bytes));
            cache.set(key, parsed);
            return parsed;
        }
    } catch {}
    return null;
}

function lookup(bundle, dottedKey) {
    if (!bundle) return null;
    const dot = dottedKey.indexOf('.');
    if (dot < 0) return typeof bundle[dottedKey] === 'string' ? bundle[dottedKey] : null;
    const section = bundle[dottedKey.slice(0, dot)];
    const field = dottedKey.slice(dot + 1);
    return section && typeof section[field] === 'string' ? section[field] : null;
}

export function resolveLanguage(settings) {
    const override = GLib.getenv('DUVAR_LANG')?.trim().toLowerCase();
    if (override && SUPPORTED_LANGUAGES.includes(override)) return override;
    try {
        if (settings?.settings_schema?.has_key('ui-language')) {
            const saved = settings.get_string('ui-language')?.trim().toLowerCase();
            if (saved && SUPPORTED_LANGUAGES.includes(saved)) return saved;
        }
    } catch {}
    return 'tr';
}

export function createI18n(extPath, settings) {
    const fallback = loadBundle(extPath, 'tr') ?? {};
    let code = resolveLanguage(settings);
    let current = code === 'tr' ? fallback : (loadBundle(extPath, code) ?? fallback);

    const refresh = () => {
        code = resolveLanguage(settings);
        current = code === 'tr' ? fallback : (loadBundle(extPath, code) ?? fallback);
        return code;
    };

    const t = (key, vars = null) => {
        let text = lookup(current, key) ?? lookup(fallback, key) ?? key;
        if (vars) {
            for (const [name, value] of Object.entries(vars))
                text = text.replaceAll(`{${name}}`, String(value));
        }
        return text;
    };

    return {
        t,
        refresh,
        lang: () => code,
    };
}
