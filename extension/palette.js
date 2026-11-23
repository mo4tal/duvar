// IDs are persisted. Keep their order; hue anchors pull toward a family
// rather than rotating warm/cool hues indiscriminately around the wheel.
const characters = [
    // Name, description, hue anchor (or offset), pull, accent saturation,
    // surface saturation, dark/light base lightness, state tint.
    ['Dengeli', 'Görselin ana tonu, ölçülü renk ve net katmanlar.', 0, 0, 48, 5, 10, 93, .12],
    ['Pastel', 'Sütlü yüzeyler, pudra tonlu kartlar ve yumuşak seçimler.', 0, 0, 32, 7, 12, 94, .11],
    ['Canlı', 'Doygun vurgu, renkli kenarlar ve belirgin etkileşimler.', 0, 0, 62, 7, 10, 92, .19],
    ['Toprak', 'Kil ve taş tonları; sıcak, mat yüzeyler.', 32, .52, 40, 8, 12, 90, .14],
    ['Serin', 'Maviye yaklaşan kenarlar ve soğuk, ferah yüzeyler.', 210, .48, 50, 7, 10, 93, .16],
    ['Sıcak', 'Kehribara yaklaşan detaylar ve krem tonlu yüzeyler.', 42, .48, 52, 8, 11, 91, .17],
    ['Analog', 'Komşu ton: yüzeyde görselin rengi, odakta yan tonu.', 30, 0, 55, 6, 11, 92, .13],
    ['Derin', 'Daha koyu taban, tok vurgu ve güçlü katman ayrımı.', 0, 0, 58, 6, 8, 89, .19],
    ['Yumuşak', 'Soluk taş tonları ve daha sakin geçişler.', 9, 0, 20, 3, 11, 93, .08],
    ['Mono', 'Renksiz yüzey ve kontroller; ayrımı ton farkı sağlar.', 0, 0, 0, 0, 9, 91, .11],
];
export const profiles = characters.map(([name, description]) => [name, description]);
export const paletteRevision = 12;
export const PANEL_STYLES = [
    ['capsule', 'Yüzen kapsül', 'Açık ve koyu temada her üst panel öğesini uyumlu cam kapsül içine alır.'],
    ['smart', 'Akıllı kapsül', 'Temiz bölgelerde uyarlanabilir mürekkep, karmaşık bölgelerde otomatik kapsül kullanır.'],
    ['scrim', 'Üst perde', 'Şeffaf üst panelde arka planı dengeleyen yumuşak geçiş perdesi uygular.'],
    ['plain', 'Sade mürekkep', 'Kapsül ve perde olmadan her öğenin rengini arka plana göre ayarlar.'],
];
export const CAPSULE_STRENGTHS = [
    ['subtle', 'Hafif cam', 'Daha geçirgen, duvar kâğıdını öne çıkaran ince cam kapsüller.'],
    ['balanced', 'Dengeli cam', 'Yüksek okunabilirlik ve zarif yarı saydamlık dengesi.'],
    ['crisp', 'Belirgin kapsül', 'Çok hareketli duvar kâğıtlarında en net ayrım ve maksimum kontrast.'],
];
export const validPanelStyle = value => PANEL_STYLES.some(([id]) => id === value);
export const validCapsuleStrength = value => CAPSULE_STRENGTHS.some(([id]) => id === value);
export const normalizePanelOptions = (opts = {}) => ({
    panelStyle: validPanelStyle(opts?.panelStyle) ? opts.panelStyle : 'smart',
    capsuleStrength: validCapsuleStrength(opts?.capsuleStrength) ? opts.capsuleStrength : 'balanced',
});
export const wallpaperPair = (light, dark, fallbackColor = '') => {
    const a = light || dark;
    const b = dark || light;
    if (a && b) return [a, b];
    const solid = typeof fallbackColor === 'string' && /^(#[0-9a-f]{6}|#[0-9a-f]{3}|rgba?\([^)]+\))$/i.test(fallbackColor.trim())
        ? `color:${fallbackColor.trim()}` : '';
    return [a || solid, b || solid];
};

const unit = v => Number.isFinite(v) && v >= 0 && v <= 1;
const validHue = h => h === null || (Number.isFinite(h) && h >= 0 && h < 360);
const validLumaStats = s => Boolean(s && typeof s === 'object' && unit(s.luma) && unit(s.p15) && unit(s.p85));
const validTopBins = bins => Array.isArray(bins) && bins.length >= 3 && bins.length <= 32 &&
    bins.every(b => Array.isArray(b) && b.length === 3 && b.every(unit));
const validTopStats = t => Boolean(validLumaStats(t) &&
    Array.isArray(t.zones) && t.zones.length === 3 && t.zones.every(unit) && unit(t.spread) &&
    (t.bins === undefined || validTopBins(t.bins)));

export const validAnalysisEntry = entry => Boolean(entry && typeof entry === 'object' &&
    validHue(entry.hue) && unit(entry.chroma ?? 0) &&
    validLumaStats(entry.scene) && validTopStats(entry.top));

export const validAnalysis = value => Array.isArray(value) && value.length === 2 &&
    value.every(validAnalysisEntry);

export const validHues = value => Array.isArray(value) && value.length === 2 &&
    value.every(item => validHue(item) || validAnalysisEntry(item));

function extractHue(input) {
    if (validAnalysisEntry(input)) return input.hue;
    return input;
}

function extractOptical(input) {
    if (!validAnalysisEntry(input)) return null;
    return {
        chroma: input.chroma ?? 0,
        scene: {luma: input.scene.luma, p15: input.scene.p15, p85: input.scene.p85},
        top: {
            luma: input.top.luma,
            p15: input.top.p15,
            p85: input.top.p85,
            zones: [...input.top.zones],
            ...(validTopBins(input.top.bins) ? {bins: input.top.bins.map(b => [...b])} : {}),
            spread: input.top.spread,
        },
    };
}

function hsl(h, s, l) {
    const a = s * Math.min(l, 100 - l) / 10000;
    const f = n => {
        const k = (n + ((h % 360 + 360) % 360) / 30) % 12;
        return Math.round(255 * (l / 100 - a * Math.max(-1, Math.min(k - 3, 9 - k, 1))))
            .toString(16).padStart(2, '0');
    };
    return `#${f(0)}${f(8)}${f(4)}`;
}

const rgb = hex => [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16));
const mix = (base, tint, strength) => `#${rgb(base).map((v, i) =>
    Math.round(v + (rgb(tint)[i] - v) * strength).toString(16).padStart(2, '0')).join('')}`;
const linToSrgb = v => {
    const c = Math.max(0, Math.min(1, v));
    return c <= 0.0031308 ? 12.92 * c : 1.055 * (c ** (1 / 2.4)) - 0.055;
};
const luminance = hex => rgb(hex).reduce((sum, value, i) => {
    const v = value / 255;
    return sum + [0.2126, 0.7152, 0.0722][i] * (v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4);
}, 0);
const contrastLuma = (a, b) => (Math.max(a, b) + .05) / (Math.min(a, b) + .05);

export function contrast(a, b) {
    return contrastLuma(luminance(a), luminance(b));
}

function compositeOverLuma(hex, alpha, bgLuma) {
    const bg = Math.round(linToSrgb(bgLuma) * 255);
    return `#${rgb(hex).map(v => Math.round(v * alpha + bg * (1 - alpha))
        .toString(16).padStart(2, '0')).join('')}`;
}

function solveAlpha(surfaceHex, inkHex, bgLuma, minRatio, minAlpha, maxAlpha) {
    const lo = Math.round(minAlpha * 100);
    const hi = Math.round(maxAlpha * 100);
    for (let step = lo; step <= hi; step += 2) {
        const alpha = step / 100;
        if (contrast(compositeOverLuma(surfaceHex, alpha, bgLuma), inkHex) >= minRatio)
            return alpha;
    }
    return maxAlpha;
}

const rgba = (hex, opacity) => `rgba(${rgb(hex).join(',')},${Number(opacity.toFixed(2))})`;

function accentFillFor(h, s, i) {
    let lightness = i === 1 ? 62 : i === 7 ? 56 : 60;
    let fill = hsl(h, s, lightness);
    while (lightness > 16 && contrast(fill, '#f5f5f5') < 4.55) {
        lightness -= 1;
        fill = hsl(h, s, lightness);
    }
    return fill;
}

export function colors(hueOrAnalysis, i, dark) {
    const hue = extractHue(hueOrAnalysis);
    const optical = extractOptical(hueOrAnalysis);
    const [, , target, pull, saturation, chroma, night, day] = characters[i];
    const neutral = hue === null;
    const h = neutral ? 0 : pull ? hue + ((target - hue + 540) % 360 - 180) * pull : hue + target;
    const ground = i === 6 && !neutral ? hue : h;
    const lightChroma = Math.min(26, Math.round(chroma * 2.4));
    const surfaceChroma = neutral ? 0 : (dark ? chroma : lightChroma);
    const surface = hsl(ground, surfaceChroma, dark ? night : day);
    const inverse = dark ? '#191919' : '#f5f5f5';
    let lightness = dark ? (i === 1 ? 75 : i === 7 ? 64 : 68) : (i === 1 ? 40 : i === 7 ? 31 : 36);
    const s = neutral ? 0 : saturation;
    let accent = hsl(h, s, lightness);
    while (Math.min(contrast(accent, surface), contrast(accent, inverse)) < (dark ? 5.0 : 4.9)) {
        lightness += dark ? 1 : -1;
        accent = hsl(h, s, lightness);
    }
    const accentBg = dark ? accentFillFor(h, s, i) : accent;
    return {
        accent, accentBg, surface,
        text: dark ? '#f5f5f5' : '#191919', profile: i,
        dye: hsl(ground, surfaceChroma, dark ? 44 : 52),
        ...(optical ? {optical} : {}),
    };
}

const isHex = value => typeof value === 'string' && /^#[0-9a-f]{6}$/.test(value);

// State fills lean further into the wallpaper hue without sacrificing label contrast.
function ink(accent, background, foreground, minimum = 4.5) {
    for (let weight = 1; weight > 0; weight -= .05) {
        const color = mix(foreground, accent, weight);
        if (contrast(color, background) >= minimum) return color;
    }
    return foreground;
}

export function layers({surface, text, accent, accentBg, dye, profile, optical}, dark) {
    const strength = characters[Number.isInteger(profile) ? profile : 0]?.[8] ?? .12;
    const tint = isHex(dye) ? dye : surface;
    const chrome = dark ? mix(surface, '#ffffff', .022) : mix('#ffffff', surface, .64);
    const sidebar = dark ? mix(surface, '#ffffff', .038) : mix(surface, tint, .095);
    const view = dark ? mix(surface, '#000000', .16) : mix('#ffffff', tint, .028);
    const popover = dark ? mix(surface, '#ffffff', .036) : mix('#ffffff', surface, .58);
    const card = dark ? mix(surface, '#ffffff', .072) : mix('#ffffff', tint, .014);
    const control = dark ? mix(popover, '#ffffff', .075) : mix(mix(surface, tint, .08), '#000000', .038);
    const hover = mix(control, accent, .07 + strength * .12);
    const pressed = mix(hover, accent, .08);
    const selected = mix(sidebar, accent, .11 + strength * .22);
    const panel = dark ? mix(surface, '#000000', .26) : mix('#ffffff', surface, .70);
    const fill = isHex(accentBg) ? accentBg : accent;
    const accentFg = contrast(fill, '#f5f5f5') >= 4.5 ? '#f5f5f5' : '#191919';
    const detail = (bg, ratio) => ink(mix(tint, accent, .42), bg, text, ratio);
    const stateGround = [hover, pressed, selected].sort((a, b) => contrast(a, text) - contrast(b, text))[0];
    // Protect frosted popups symmetrically: light popups over dark wallpapers
    // paint darker, and dark popups over bright wallpapers paint lighter.
    const scene = validLumaStats(optical?.scene) ? optical.scene : null;
    const worstBg = scene ? (dark ? scene.p85 : scene.p15) : (dark ? 0.35 : 0.08);
    const estAlpha = scene
        ? solveAlpha(popover, text, worstBg, 7.2, dark ? 0.62 : 0.72, 0.88)
        : (dark ? 0.62 : 0.72);
    const glassGround = compositeOverLuma(popover, estAlpha, worstBg);
    const secondaryGround = contrast(glassGround, text) < contrast(stateGround, text) ? glassGround : stateGround;
    return {
        chrome, sidebar, view, card, popover, control, hover, selected, panel, pressed,
        accentBg: fill, accentFg,
        secondary: detail(secondaryGround, 4.65),
        border: mix(mix(surface, tint, .10), text, dark ? .11 : .12),
        frame: mix(mix(surface, tint, .15), text, dark ? .19 : .24),
        controlEdge: mix(mix(control, tint, .10), text, dark ? .12 : .16),
        focus: ink(accent, stateGround, text, 3.3),
    };
}

// Compute wallpaper-aware panel ink, adaptive readability shadows, gradient polarity
// and frosted surface opacities so opposite-brightness wallpapers stay crisp.
export function opticalProfile(tokens, dark) {
    const {text, accent, surface} = tokens;
    const l = layers(tokens, dark);
    const hasOptical = Boolean(validTopStats(tokens.optical?.top) && validLumaStats(tokens.optical?.scene));
    const top = hasOptical
        ? tokens.optical.top
        : (dark
            ? {luma: 0.10, p15: 0.04, p85: 0.20, zones: [0.10, 0.10, 0.10], spread: 0.16}
            : {luma: 0.80, p15: 0.66, p85: 0.90, zones: [0.80, 0.80, 0.80], spread: 0.24});
    const scene = hasOptical
        ? tokens.optical.scene
        : (dark ? {luma: 0.12, p15: 0.04, p85: 0.24} : {luma: 0.78, p15: 0.60, p85: 0.90});

    const LIGHT_INK = '#f5f5f5';
    const DARK_INK = '#191919';
    const lightY = luminance(LIGHT_INK);
    const darkY = luminance(DARK_INK);
    const topSamples = [...top.zones, top.p15, top.p85, top.luma];

    const evalInk = inkY => {
        const zoneMin = Math.min(...top.zones.map(y => contrastLuma(inkY, y)));
        const worst = Math.min(...topSamples.map(y => contrastLuma(inkY, y)));
        const mean = contrastLuma(inkY, top.luma);
        return {zoneMin, worst, mean, score: worst * 0.50 + zoneMin * 0.35 + mean * 0.15};
    };

    const lightEval = evalInk(lightY);
    const darkEval = evalInk(darkY);
    const tieBias = dark ? 0.20 : -0.20;
    const useLightTopInk = hasOptical
        ? (lightEval.score + tieBias >= darkEval.score)
        : dark;

    const topInk = useLightTopInk ? LIGHT_INK : DARK_INK;
    const chosenEval = useLightTopInk ? lightEval : darkEval;
    const topWorstBg = useLightTopInk ? Math.max(top.p85, ...top.zones) : Math.min(top.p15, ...top.zones);

    // Never paint fuzzy text-shadow/icon-shadow halos behind panel labels; per-button
    // spatial ink and selective floating glass pills handle mixed backgrounds cleanly.
    const topTextShadow = 'none';
    const topIconShadow = 'none';

    const topPillTint = useLightTopInk ? mix('#ffffff', accent, .22) : mix('#191919', accent, .24);
    const topPillHover = rgba(topPillTint, useLightTopInk ? .15 : .11);
    const topPillActive = rgba(topPillTint, useLightTopInk ? .24 : .18);
    const topPillActiveHover = rgba(topPillTint, useLightTopInk ? .30 : .23);

    // Gradient panel scrim matches topInk polarity so dark wallpapers never get milky white fog
    // and bright wallpapers never get muddy black bands when dark ink is clearer.
    const gradientBase = useLightTopInk
        ? (dark ? l.panel : mix('#141518', accent, .12))
        : (dark ? mix('#f2f3f6', accent, .08) : l.panel);
    const gradientAlpha = hasOptical
        ? solveAlpha(gradientBase, topInk, topWorstBg, 5.2, 0.48, 0.74)
        : (dark ? 0.48 : 0.52);

    // Dynamic opacities for BMS tinted panels and blurred popups/dialogs/dash.
    const modeWorstTop = dark ? Math.max(top.p85, ...top.zones) : Math.min(top.p15, ...top.zones);
    const modeWorstScene = dark ? scene.p85 : scene.p15;
    // Modal dialogs sit over GNOME's darkened lightbox backdrop (~60% black).
    const modalWorstBg = dark ? modeWorstScene : Math.min(modeWorstScene, 0.08);

    const panelBgAlpha = hasOptical
        ? solveAlpha(l.panel, text, modeWorstTop, dark ? 7.2 : 8.0, dark ? 0.48 : 0.54, 0.84)
        : (dark ? 0.48 : 0.54);
    const contrastedPillAlpha = hasOptical
        ? solveAlpha(l.panel, text, modeWorstTop, dark ? 7.5 : 8.5, dark ? 0.68 : 0.78, 0.92)
        : (dark ? 0.68 : 0.78);

    const popoverAlpha = hasOptical
        ? Math.max(
            solveAlpha(l.popover, text, modeWorstScene, dark ? 8.0 : 10.2, dark ? 0.62 : 0.72, 0.88),
            solveAlpha(l.popover, l.secondary, modeWorstScene, 4.75, dark ? 0.62 : 0.72, 0.88))
        : (dark ? 0.62 : 0.72);
    const cardAlpha = hasOptical
        ? Math.max(
            solveAlpha(l.card, text, modeWorstScene, dark ? 7.5 : 9.2, dark ? 0.52 : 0.62, 0.86),
            solveAlpha(l.card, l.secondary, modeWorstScene, 4.70, dark ? 0.52 : 0.62, 0.86))
        : (dark ? 0.52 : 0.62);
    const controlAlpha = hasOptical
        ? Math.max(
            solveAlpha(l.control, text, modeWorstScene, dark ? 7.0 : 8.6, dark ? 0.50 : 0.58, 0.84),
            solveAlpha(l.control, l.secondary, modeWorstScene, 4.65, dark ? 0.50 : 0.58, 0.84))
        : (dark ? 0.50 : 0.58);
    const modalAlpha = hasOptical
        ? Math.max(
            solveAlpha(l.popover, text, modalWorstBg, dark ? 8.5 : 10.8, dark ? 0.72 : 0.84, 0.92),
            solveAlpha(l.popover, l.secondary, modalWorstBg, 4.85, dark ? 0.72 : 0.84, 0.92))
        : (dark ? 0.72 : 0.84);
    const searchAlpha = hasOptical
        ? Math.max(
            solveAlpha(l.popover, text, modeWorstScene, dark ? 7.5 : 9.5, dark ? 0.58 : 0.70, 0.88),
            solveAlpha(l.popover, l.secondary, modeWorstScene, 4.65, dark ? 0.58 : 0.70, 0.88))
        : (dark ? 0.58 : 0.70);
    const dashAlpha = hasOptical
        ? solveAlpha(l.panel, text, modeWorstScene, dark ? 7.0 : 8.0, dark ? 0.50 : 0.58, 0.84)
        : (dark ? 0.50 : 0.58);

    // Overview with BMS blur dims the wallpaper; protect explicit light overview theme on dark wallpapers.
    const overviewLightInk = hasOptical && top.luma < 0.22 && top.p85 < 0.35 ? LIGHT_INK : text;
    const overviewLightPills = overviewLightInk === LIGHT_INK
        ? ['rgba(255,255,255,0.14)', 'rgba(255,255,255,0.22)', 'rgba(255,255,255,0.27)']
        : ['rgba(0,0,0,0.10)', 'rgba(0,0,0,0.17)', 'rgba(0,0,0,0.22)'];

    return {
        hasOptical,
        topInk,
        useLightTopInk,
        topWorstContrast: Number(chosenEval.worst.toFixed(2)),
        topTextShadow,
        topIconShadow,
        topPillHover,
        topPillActive,
        topPillActiveHover,
        gradientBase,
        gradientAlpha,
        panelBgAlpha,
        contrastedPillAlpha,
        popoverAlpha,
        cardAlpha,
        controlAlpha,
        modalAlpha,
        searchAlpha,
        dashAlpha,
        overviewLightInk,
        overviewLightPills,
        surface,
    };
}

// Decide crisp per-button ink or selective floating glass pill from the exact
// horizontal top-strip slices behind an individual panel item ([x0..x1] in [0..1]).
export function panelSliceMode(optical, x0, x1, dark, style = 'smart') {
    const modeStyle = validPanelStyle(style) ? style : 'smart';
    if (modeStyle === 'capsule')
        return dark ? 'pill-dark' : 'pill-light';

    const top = optical?.top;
    if (!validTopStats(top)) return dark ? 'light' : 'dark';
    const loX = Math.max(0, Math.min(1, Number.isFinite(x0) ? x0 : 0));
    const hiX = Math.max(loX, Math.min(1, Number.isFinite(x1) ? x1 : loX));
    let mean, p15, p85;
    if (validTopBins(top.bins)) {
        const n = top.bins.length;
        const start = Math.max(0, Math.min(n - 1, Math.floor(Math.max(0, loX - 0.004) * n)));
        const end = Math.max(start, Math.min(n - 1, Math.floor(Math.min(0.9999, hiX + 0.004) * n)));
        let sum = 0;
        let count = 0;
        p15 = 1;
        p85 = 0;
        for (let i = start; i <= end; i++) {
            const [m, lo, hi] = top.bins[i];
            sum += m;
            count++;
            if (lo < p15) p15 = lo;
            if (hi > p85) p85 = hi;
        }
        mean = count ? sum / count : top.luma;
    } else {
        const mid = (loX + hiX) / 2;
        const zone = mid < 0.28 ? top.zones[0] : mid < 0.72 ? top.zones[1] : top.zones[2];
        mean = zone;
        p15 = Math.min(zone, top.p15);
        p85 = Math.max(zone, top.p85);
    }

    const lightY = luminance('#f5f5f5');
    const darkY = luminance('#191919');
    const lightWorst = contrastLuma(lightY, p85);
    const darkWorst = contrastLuma(darkY, p15);
    const lightMean = contrastLuma(lightY, mean);
    const darkMean = contrastLuma(darkY, mean);

    // Clean single-ink region (zero pill, zero shadow).
    if (lightWorst >= 4.5 && (lightWorst >= darkWorst || darkWorst < 4.5))
        return 'light';
    if (darkWorst >= 4.5 && (darkWorst > lightWorst || lightWorst < 4.5))
        return 'dark';
    // Allow minor specular detail (<=15% edge pixels) when dominant area is clean.
    if (lightWorst >= 3.8 && lightMean >= 5.2 && lightWorst >= darkWorst)
        return 'light';
    if (darkWorst >= 3.8 && darkMean >= 5.2 && darkWorst > lightWorst)
        return 'dark';
    if (modeStyle === 'plain' || modeStyle === 'scrim')
        return lightMean + (dark ? 0.15 : -0.15) >= darkMean ? 'light' : 'dark';
    // Busy/striped pixels right behind this button: wrap ONLY this item in a sleek
    // theme-harmonized floating glass pill for guaranteed legibility on text & icons.
    return dark ? 'pill-dark' : 'pill-light';
}

export function capsuleAlphas(tokens, dark, capsuleStrength = 'balanced') {
    const {panel: panelColor} = layers(tokens, dark);
    const isNeutral = /^#(..)\1\1$/.test(tokens.accent) && /^#(..)\1\1$/.test(tokens.surface);
    const tint = isHex(tokens.dye) ? tokens.dye : tokens.surface;
    const ambient = mix(tint, tokens.accent, .55);
    // Kiss the frosted capsule softly with the wallpaper's ambient hue while keeping
    // neutral/Mono palettes strictly achromatic.
    const rawLight = dark ? mix('#f2f3f6', tokens.accent, .08) : panelColor;
    const rawDark = dark ? panelColor : mix('#16171a', tokens.accent, .12);
    const lightGlass = isNeutral ? rawLight : mix(rawLight, ambient, dark ? .10 : .13);
    const darkGlass = isNeutral ? rawDark : mix(rawDark, ambient, dark ? .16 : .14);
    const top = validTopStats(tokens.optical?.top) ? tokens.optical.top : null;
    const worstForLightPill = top ? Math.min(top.p15, ...top.zones) : 0.12;
    const worstForDarkPill = top ? Math.max(top.p85, ...top.zones) : 0.82;
    const presets = {
        subtle: {lightMin: 0.44, darkMin: 0.38, ratio: 4.8, max: 0.72},
        balanced: {lightMin: 0.56, darkMin: 0.50, ratio: 5.8, max: 0.80},
        crisp: {lightMin: 0.72, darkMin: 0.66, ratio: 7.6, max: 0.90},
    };
    const cfg = presets[capsuleStrength] ?? presets.balanced;
    return {
        lightGlass,
        darkGlass,
        lightAlpha: top
            ? solveAlpha(lightGlass, '#191919', worstForLightPill, cfg.ratio, cfg.lightMin, cfg.max)
            : cfg.lightMin,
        darkAlpha: top
            ? solveAlpha(darkGlass, '#f5f5f5', worstForDarkPill, cfg.ratio, cfg.darkMin, cfg.max)
            : cfg.darkMin,
    };
}

function panelButtonInkRules(tokens, dark, scopes, important = '', opts = {}) {
    const {capsuleStrength} = normalizePanelOptions(opts);
    const {accent} = tokens;
    const {hover, pressed} = layers(tokens, dark);
    const {lightGlass, darkGlass, lightAlpha, darkAlpha} = capsuleAlphas(tokens, dark, capsuleStrength);
    const lightInk = '#f5f5f5';
    const darkInk = '#191919';
    const lightTint = mix('#ffffff', accent, .22);
    const darkTint = mix('#191919', accent, .24);
    const lightGlassHover = dark ? mix(lightGlass, '#ffffff', .18) : mix(hover, accent, .08);
    const darkGlassHover = dark ? mix(hover, accent, .10) : mix(darkGlass, '#ffffff', .10);
    const lightGlassActive = dark ? mix(lightGlass, accent, .18) : mix(pressed, accent, .12);
    const darkGlassActive = dark ? mix(pressed, accent, .16) : mix(darkGlass, accent, .18);

    const buildPlain = (cls, inkColor, tint, hovA, actA, actHovA) => {
        const j = parts => scopes.flatMap(s => parts.map(p => `${s} ${p}`)).join(', ');
        const btn = `.panel-button.${cls}`;
        const hov = rgba(tint, hovA);
        const act = rgba(tint, actA);
        const actHov = rgba(tint, actHovA);
        const clockStates = ['', ':hover', ':focus', ':active', ':checked', ':active:hover', ':checked:hover']
            .map(st => `${btn}.clock-display${st}`);
        return `${j([btn, `${btn}.clock-display`, `${btn}.clock-display .clock`, `${btn} .clock`, `${btn} .system-status-icon`, `${btn} StIcon`, `${btn} .netSpeedLabel`])} { color: ${inkColor}${important}; text-shadow: none${important}; icon-shadow: none${important}; }\n` +
            `${j([`${btn}#panelActivities .workspace-dot`, `${btn}#panelActivities:active .workspace-dot`, `${btn}#panelActivities:checked .workspace-dot`, `${btn}#panelActivities:focus .workspace-dot`])} { background-color: ${inkColor}${important}; }\n` +
            `${j([`${btn}:hover`, `${btn}:focus`, `${btn}.clock-display:hover .clock`, `${btn}.clock-display:focus .clock`])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${hov}${important}; color: ${inkColor}${important}; }\n` +
            `${j([`${btn}.duvar-system-menu:active`, `${btn}:active`, `${btn}:checked`, `${btn}.weatherpanel-button.active`, `${btn}.clock-display:active .clock`, `${btn}.clock-display:checked .clock`])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${act}${important}; color: ${inkColor}${important}; }\n` +
            `${j([`${btn}:active:hover`, `${btn}:checked:hover`, `${btn}.weatherpanel-button.active:hover`, `${btn}.clock-display:active:hover .clock`, `${btn}.clock-display:checked:hover .clock`])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${actHov}${important}; color: ${inkColor}${important}; }\n` +
            `${j(clockStates)} { background-color: transparent${important}; box-shadow: none${important}; }\n`;
    };

    const buildPill = (cls, inkColor, baseHex, hovHex, actHex, baseA) => {
        const j = parts => scopes.flatMap(s => parts.map(p => `${s} ${p}`)).join(', ');
        const btn = `.panel-button.${cls}`;
        const base = rgba(baseHex, baseA);
        const hov = rgba(hovHex, Math.min(0.92, baseA + 0.09));
        const act = rgba(actHex, Math.min(0.95, baseA + 0.15));
        const actHov = rgba(actHex, Math.min(0.97, baseA + 0.19));
        const clockStates = ['', ':hover', ':focus', ':active', ':checked', ':active:hover', ':checked:hover']
            .map(st => `${btn}.clock-display${st}`);
        return `${j([btn, `${btn}.clock-display .clock`, `${btn}.clock-display StIcon`])} { background-color: transparent${important}; color: ${inkColor}${important}; box-shadow: inset 0 0 0 100px ${base}${important}; text-shadow: none${important}; icon-shadow: none${important}; transition-duration: 180ms; }\n` +
            `${j([`${btn} .system-status-icon`, `${btn} StIcon`, `${btn} .netSpeedLabel`])} { color: ${inkColor}${important}; text-shadow: none${important}; icon-shadow: none${important}; }\n` +
            `${j([`${btn}#panelActivities .workspace-dot`, `${btn}#panelActivities:active .workspace-dot`, `${btn}#panelActivities:checked .workspace-dot`, `${btn}#panelActivities:focus .workspace-dot`])} { background-color: ${inkColor}${important}; }\n` +
            `${j([`${btn}:hover`, `${btn}:focus`, `${btn}.clock-display:hover .clock`, `${btn}.clock-display:focus .clock`])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${hov}${important}; color: ${inkColor}${important}; }\n` +
            `${j([`${btn}.duvar-system-menu:active`, `${btn}:active`, `${btn}:checked`, `${btn}.weatherpanel-button.active`, `${btn}.clock-display:active .clock`, `${btn}.clock-display:checked .clock`])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${act}${important}; color: ${inkColor}${important}; }\n` +
            `${j([`${btn}:active:hover`, `${btn}:checked:hover`, `${btn}.weatherpanel-button.active:hover`, `${btn}.clock-display:active:hover .clock`, `${btn}.clock-display:checked:hover .clock`])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${actHov}${important}; color: ${inkColor}${important}; }\n` +
            `${j(clockStates)} { background-color: transparent${important}; box-shadow: none${important}; }\n`;
    };

    const allClasses = ['duvar-ink-light', 'duvar-ink-dark', 'duvar-ink-pill-light', 'duvar-ink-pill-dark'];
    const ovStates = [':overview', '.unlock-screen', '.login-screen'];
    const ovScopes = scopes.flatMap(s => ovStates.map(st => `${s}${st}`));
    const ovReset = allClasses.map(cls => {
        const j = parts => ovScopes.flatMap(s => parts.map(p => `${s} ${p}`)).join(', ');
        const btn = `.panel-button.${cls}`;
        return `${j([btn, `${btn}.clock-display`, `${btn}.clock-display .clock`, `${btn} .system-status-icon`, `${btn} StIcon`, `${btn} .netSpeedLabel`])} { color: #f5f5f5${important}; background-color: transparent${important}; box-shadow: inset 0 0 0 100px transparent${important}; text-shadow: none${important}; icon-shadow: none${important}; }\n` +
            `${j([`${btn}#panelActivities .workspace-dot`])} { background-color: #f5f5f5${important}; }\n` +
            `${j([`${btn}:hover`, `${btn}:focus`, `${btn}.clock-display:hover .clock`, `${btn}.clock-display:focus .clock`])} { box-shadow: inset 0 0 0 100px rgba(255,255,255,0.14)${important}; color: #f5f5f5${important}; }\n` +
            `${j([`${btn}.duvar-system-menu:active`, `${btn}:active`, `${btn}:checked`, `${btn}.weatherpanel-button.active`, `${btn}.clock-display:active .clock`, `${btn}.clock-display:checked .clock`])} { box-shadow: inset 0 0 0 100px rgba(255,255,255,0.22)${important}; color: #f5f5f5${important}; }\n` +
            `${j([`${btn}.clock-display`, `${btn}.clock-display:hover`, `${btn}.clock-display:active`, `${btn}.clock-display:checked`])} { box-shadow: none${important}; }\n`;
    }).join('');

    return buildPlain('duvar-ink-light', lightInk, lightTint, .15, .24, .30) +
        buildPlain('duvar-ink-dark', darkInk, darkTint, .11, .18, .23) +
        buildPill('duvar-ink-pill-light', darkInk, lightGlass, lightGlassHover, lightGlassActive, lightAlpha) +
        buildPill('duvar-ink-pill-dark', lightInk, darkGlass, darkGlassHover, darkGlassActive, darkAlpha) +
        ovReset;
}

function toggleCss(scopes, fill, fg, important = '') {
    const toggles = scopes.flatMap(scope =>
        ['.quick-toggle', '.quick-toggle-has-menu .quick-toggle-menu-button'].map(node => `${scope}${node}`));
    const rule = (state, body) => `${toggles.map(node => `${node}${state}`).join(', ')} { ${body} }\n`;
    const active = mix(fill, fg, contrast(mix(fill, fg, .08), fg) >= 4.5 ? .08 : 0);
    const pressed = mix(fill, fg, contrast(mix(fill, fg, .14), fg) >= 4.5 ? .14 : 0);
    const rim = rgba(fg, .14);
    return rule(':checked', `background-color: ${fill}${important}; color: ${fg}${important}; border-color: transparent${important}; box-shadow: inset 0 0 0 1px ${rim}${important};`) +
        rule(':checked:hover', `background-color: ${active}${important}; color: ${fg}${important}; box-shadow: inset 0 0 0 1px ${rim}${important};`) +
        rule(':checked:active', `background-color: ${pressed}${important}; color: ${fg}${important}; box-shadow: inset 0 0 0 1px ${rim}${important};`) +
        rule(':checked:focus', `box-shadow: inset 0 0 0 2px ${fg}${important};`);
}

function menuRoles(tokens, dark, roots = [''], priority = '', bmsOverride = false, optical = null) {
    const {text} = tokens;
    const {popover, card, control, hover, pressed, accentBg, accentFg, secondary, border, focus} = layers(tokens, dark);
    const opt = optical ?? opticalProfile(tokens, dark);
    const quick = roots.map(root => `${root}.quick-settings `);
    const rule = (scopes, nodes, style, imp = priority) => `${scopes.flatMap(scope => nodes.map(node => `${scope}${node}`)).join(', ')} { ${
        Object.entries(style).map(([key, value]) => `${key}: ${value}${imp};`).join(' ')} }\n`;
    const fill = color => ({'background-color': color, color: text});
    const ring = color => ({'box-shadow': `inset 0 0 0 2px ${color}`});
    const buttons = ['.quick-toggle', '.quick-toggle-has-menu .quick-toggle-menu-button',
        '.quick-slider .icon-button', '.quick-slider .button', '.quick-settings-system-item .icon-button'];
    const cards = ['.world-clocks-button', '.weather-button', '.events-button'];
    const btnA = opt.controlAlpha;
    const cardA = opt.cardAlpha;
    const btnBase = bmsOverride ? rgba(control, btnA) : control;
    const btnHover = bmsOverride ? rgba(hover, Math.min(0.92, btnA + 0.16)) : hover;
    const btnActive = bmsOverride ? rgba(pressed, Math.min(0.96, btnA + 0.24)) : pressed;
    const cardBase = bmsOverride ? rgba(card, Math.max(0.46, cardA - 0.06)) : card;
    const cardHover = bmsOverride ? rgba(hover, Math.min(0.90, cardA + 0.12)) : hover;
    const cardActive = bmsOverride ? rgba(pressed, Math.min(0.94, cardA + 0.20)) : pressed;
    const softEdge = `inset 0 0 0 1px ${rgba(text, dark ? .07 : .08)}`;
    const hoverEdge = `inset 0 0 0 1px ${rgba(text, dark ? .11 : .12)}`;
    const interactive = (scopes, nodes, base, hov, act) =>
        rule(scopes, nodes, {...fill(base), 'border-color': 'transparent', 'box-shadow': softEdge, 'transition-duration': '180ms'}) +
        rule(scopes, nodes.map(node => `${node}:hover`), {...fill(hov), 'box-shadow': hoverEdge}) +
        rule(scopes, nodes.map(node => `${node}:active`), {...fill(act), 'box-shadow': hoverEdge}) +
        rule(scopes, nodes.map(node => `${node}:focus`), ring(focus), ' !important');
    const divider = bmsOverride ? rgba(text, .12) : border;
    const track = bmsOverride ? rgba(text, dark ? .20 : .18) : mix(popover, text, dark ? .20 : .18);
    const labelScope = bmsOverride ? ['.quick-settings '] : quick;
    const labels =
        rule(labelScope, buttons.map(node => `${node}:insensitive`), {color: secondary}) +
        rule(labelScope, ['.avatar-realname-label', '.quick-toggle .title', '.quick-toggle-menu .header .title'], {color: 'inherit'}) +
        rule(labelScope, ['.avatar-username-label', '.avatar-username-label.with-real-name',
            '.quick-toggle .subtitle', '.quick-toggle-menu .header .subtitle'], {color: secondary}) +
        rule(labelScope, ['.quick-toggle:checked .subtitle'], {color: accentFg}) +
        rule(labelScope, ['.avatar-picture'], {'border-color': divider});
    return interactive(quick, buttons, btnBase, btnHover, btnActive) +
        rule(quick, ['.avatar-button.no-bg', '.avatar-button.no-bg:hover', '.avatar-button.no-bg:active'],
            {'background-color': 'transparent', 'box-shadow': 'none'}) +
        interactive(roots, cards, cardBase, cardHover, cardActive) +
        labels +
        rule(quick, ['.quick-toggle-separator'], {'background-color': divider}) +
        rule(quick, ['.quick-toggle-has-menu:checked .quick-toggle-separator'],
            {'background-color': mix(accentBg, accentFg, .28)}) +
        rule(quick, ['.slider'], {color: focus, '-barlevel-active-background-color': focus,
            '-barlevel-background-color': track}) +
        rule(quick, ['.quick-slider .slider-bin:focus'], ring(focus), ' !important') +
        toggleCss(quick, accentBg, accentFg, priority);
}

function panelScopeRule(prefixes, inkColor, hoverPill, activePill, activeHoverPill, {
    important = ' !important',
    textShadow = 'none',
    iconShadow = 'none',
} = {}) {
    const j = parts => prefixes.flatMap(p => parts.map(part => `${p} ${part}`)).join(', ');
    const clockStates = ['', ':hover', ':focus', ':active', ':checked', ':active:hover', ':checked:hover']
        .map(s => `.panel-button.clock-display${s}`);
    return `${j(['.panel-button', '.panel-button.clock-display', '.panel-button.clock-display .clock', '.panel-button.clock-display StIcon', '.netSpeedLabel'])} { color: ${inkColor}${important}; text-shadow: ${textShadow}${important}; icon-shadow: ${iconShadow}${important}; }\n` +
        `${j(['.panel-button#panelActivities .workspace-dot', '.panel-button#panelActivities:active .workspace-dot', '.panel-button#panelActivities:checked .workspace-dot', '.panel-button#panelActivities:focus .workspace-dot'])} { background-color: ${inkColor}${important}; }\n` +
        `${j(['.panel-button:hover', '.panel-button:focus', '.panel-button.clock-display:hover .clock', '.panel-button.clock-display:focus .clock'])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${hoverPill}${important}; color: ${inkColor}${important}; }\n` +
        `${j(['.panel-button.duvar-system-menu:active', '.panel-button:active', '.panel-button:checked', '.panel-button.weatherpanel-button.active', '.panel-button.clock-display:active .clock', '.panel-button.clock-display:checked .clock'])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${activePill}${important}; color: ${inkColor}${important}; }\n` +
        `${j(['.panel-button:active:hover', '.panel-button:checked:hover', '.panel-button.weatherpanel-button.active:hover', '.panel-button.clock-display:active:hover .clock', '.panel-button.clock-display:checked:hover .clock'])} { background-color: transparent${important}; box-shadow: inset 0 0 0 100px ${activeHoverPill}${important}; color: ${inkColor}${important}; }\n` +
        `${j(clockStates)} { background-color: transparent${important}; box-shadow: none${important}; }\n`;
}

export function shellCss(tokens, dark, opts = {}) {
    const panelOpts = normalizePanelOptions(opts);
    const {text, accent} = tokens;
    const {popover, card, control, hover, pressed, selected, panel, accentBg, accentFg, focus, secondary} = layers(tokens, dark);
    const opt = opticalProfile(tokens, dark);
    const hairline = rgba(text, dark ? .09 : .10);
    const softEdge = `inset 0 0 0 1px ${rgba(text, dark ? .07 : .08)}`;
    const entryFocusBg = mix(control, accent, dark ? .08 : .06);
    const entries = [
        '.popup-menu StEntry', '.modal-dialog StEntry', '.prompt-dialog StEntry',
        '.run-dialog StEntry', '.clipboard-dialog StEntry', '.search-entry',
        '.app-folder-dialog .folder-name-container .folder-name-entry',
    ];
    const entryHints = [
        'StEntry StLabel.hint-text',
        ...entries.map(s => `${s} StLabel.hint-text`),
    ];
    const entryIcons = [
        ...entries.map(s => `${s} StIcon`),
        'StEntry StIcon.peek-password',
        '.search-entry .search-entry-icon',
    ];
    const popupButtons = [
        '.popup-menu .button', '.popup-menu .icon-button',
        '.message-list .message-list-clear-button.button', '.notification-button',
        '.message .message-header .message-expand-button', '.message .message-header .message-close-button',
        '.message-notification-group .message-collapse-button',
        '.modal-dialog .button', '.modal-dialog .modal-dialog-button-box .modal-dialog-button',
        '.background-app-item .icon-button',
        '.screenshot-ui-type-button', '.screenshot-ui-show-pointer-button', '.screenshot-ui-close-button',
    ];
    const flatButtons = [
        '.calendar .calendar-day-heading', '.calendar .calendar-day',
        '.calendar .calendar-month-header .calendar-month-label',
        '.calendar .calendar-month-header .pager-button',
        '.datemenu-today-button', '.popup-menu .icon-button.flat', '.popup-menu .button.flat',
    ];
    const mutedLabels = [
        '.message .message-header', '.message .message-header .message-header-content .event-time',
        '.message .message-time', '.message .message-source-title', '.message-list .message-list-placeholder',
        '.message .message-secondary-bin',
        '.events-button .events-box .events-title', '.events-button .events-box .events-list .event-box .event-time',
        '.events-button .events-box .events-list .event-placeholder',
        '.world-clocks-button .world-clocks-header', '.world-clocks-button .world-clocks-grid .world-clocks-timezone',
        '.weather-button .weather-box .weather-header-box .weather-header',
        '.device-subtitle', '.background-app-item .subtitle',
        '.prompt-dialog .prompt-dialog-description', '.message-dialog-content .message-dialog-description',
        '.run-dialog .run-dialog-description',
        '.dialog-list .dialog-list-box .dialog-list-item .dialog-list-item-description',
    ];
    // Transparent-panel extensions (with or without Blur My Shell) sit directly over the wallpaper.
    const transparentPanelSelectors = [
        '#panel.transparent-panel', '#panel.transparent-top-bar', '#panel.panel-transparency',
    ];
    const capsuleDesktopScope = ['#panel.duvar-panel-capsule'];
    const scrimRule = `#panel.duvar-panel-scrim { background-color: transparent; background-gradient-direction: vertical; background-gradient-start: ${rgba(opt.gradientBase, Math.min(0.66, opt.gradientAlpha))}; background-gradient-end: ${rgba(opt.gradientBase, 0)}; }\n` +
        `#panel.duvar-panel-scrim:overview, #panel.duvar-panel-scrim.unlock-screen, #panel.duvar-panel-scrim.login-screen { background-color: transparent; background-gradient-start: transparent; background-gradient-end: transparent; }\n`;
    const warningInk = dark ? '#f6c244' : '#8f6200';
    // GNOME Shell 51 keeps gnome-shell-dark.css as default_stylesheet when
    // color-scheme is "default". Style popup/calendar/dialog/entry roles directly so
    // light mode stays clean without importing the full 198 KB light sheet.
    return `#panel { background-color: ${panel}; }\n#panel:overview, #panel.unlock-screen, #panel.login-screen { background-color: transparent; }\n` +
        panelStates(tokens, dark) +
        (opt.hasOptical
            ? panelScopeRule(transparentPanelSelectors, opt.topInk, opt.topPillHover, opt.topPillActive, opt.topPillActiveHover, {
                important: '', textShadow: 'none', iconShadow: 'none',
            })
            : '') +
        panelButtonInkRules(tokens, dark, [...transparentPanelSelectors, ...capsuleDesktopScope], '', panelOpts) +
        scrimRule +
        `.popup-menu, .popup-inactive-menu-item, .background-app-item.popup-inactive-menu-item { color: ${text}; }\n` +
        `.popup-menu .popup-ornament { color: ${accent}; }\n` +
        `.popup-menu-content, .candidate-popup-content, .quick-settings, .quick-toggle-menu, .modal-dialog, .run-dialog, .clipboard-dialog, .osd-window, .resize-popup, .switcher-list, .workspace-switcher, .screenshot-ui-panel, .notification-banner, .app-folder-dialog { background-color: ${popover}; color: ${text}; border-color: ${hairline}; }\n` +
        `.app-folder-dialog .folder-name-container .folder-name-label, .app-folder-dialog .folder-name-container .folder-name-entry, .osd-window, .osd-window StLabel, .osd-window StIcon, .switcher-list, .switcher-list .item-box, .workspace-switcher, .screenshot-ui-panel, .candidate-popup-content, .candidate-box, .candidate-page-button, .polkit-dialog-user-layout .polkit-dialog-user-label, .message-dialog-content .message-dialog-title, .dialog-list .dialog-list-title, .dialog-list .dialog-list-box .dialog-list-item, .message .message-title, .message .message-body { color: ${text}; }\n` +
        `.polkit-dialog-user-layout .polkit-dialog-user-root-label { color: ${warningInk}; }\n` +
        `.switcher-list .item-box:selected, .switcher-list .item-box:hover, .candidate-box:selected, .candidate-box:hover { background-color: ${selected}; color: ${text}; }\n` +
        `.prompt-dialog .prompt-dialog-null-label { background-color: transparent; }\n` +
        `.prompt-dialog .prompt-dialog-info-label { background-color: ${rgba(text, dark ? .09 : .07)}; color: ${text}; }\n` +
        `.prompt-dialog .prompt-dialog-error-label, .end-session-dialog .end-session-dialog-battery-warning, .end-session-dialog .dialog-list-title { background-color: rgba(205,147,9,0.14); color: ${warningInk}; }\n` +
        `.osd-window .level { color: ${focus}; -barlevel-active-background-color: ${focus}; -barlevel-background-color: ${rgba(text, dark ? .20 : .18)}; }\n` +
        `.popup-menu .slider { color: ${focus}; -barlevel-active-background-color: ${focus}; -barlevel-background-color: ${rgba(text, dark ? .20 : .18)}; }\n` +
        `.popup-sub-menu { background-color: ${mix(popover, text, dark ? .05 : .045)}; }\n` +
        `.popup-menu-item, .popup-sub-menu .popup-menu-item { transition-duration: 160ms; }\n` +
        `.popup-menu-item:hover, .popup-menu-item:selected, .popup-menu-item:checked, .popup-menu-item:focus, .popup-sub-menu .popup-menu-item:hover, .popup-sub-menu .popup-menu-item:selected, .popup-sub-menu .popup-menu-item:checked, .popup-sub-menu .popup-menu-item:focus { background-color: ${hover}; color: ${text}; }\n` +
        `.popup-menu-item:active, .popup-sub-menu .popup-menu-item:active { background-color: ${pressed}; color: ${text}; }\n` +
        `.popup-menu-item:insensitive, .popup-sub-menu .popup-menu-item:insensitive { color: ${secondary}; }\n` +
        `.popup-separator-menu-item .popup-separator-menu-item-separator, .popup-sub-menu .popup-separator-menu-item .popup-separator-menu-item-separator { background-color: ${rgba(text, .11)}; }\n` +
        `.message-list { border-color: ${rgba(text, .11)}; }\n` +
        `.message { background-color: ${card}; color: ${text}; border-color: transparent; box-shadow: ${softEdge}; transition-duration: 180ms; }\n` +
        `.message:second-in-stack { background-color: ${mix(card, popover, .35)}; }\n` +
        `.message:lower-in-stack { background-color: ${mix(card, popover, .65)}; }\n` +
        `.message:hover, .message:focus { background-color: ${hover}; color: ${text}; }\n` +
        `.message:active { background-color: ${pressed}; color: ${text}; }\n` +
        `.message .message-box .message-icon.message-themed-icon { background-color: ${rgba(text, .08)}; }\n` +
        `.message-media-control { color: ${text}; transition-duration: 160ms; }\n` +
        `.message-media-control:hover { background-color: ${hover}; color: ${text}; }\n` +
        `.message-media-control:active { background-color: ${pressed}; color: ${text}; }\n` +
        `.message-media-control:insensitive { color: ${secondary}; }\n` +
        `${mutedLabels.join(', ')} { color: ${secondary}; }\n` +
        `.world-clocks-button .world-clocks-header.no-world-clocks, .weather-button .weather-box .weather-header-box .weather-header.no-location { color: ${text}; }\n` +
        `${popupButtons.join(', ')} { background-color: ${control}; color: ${text}; border-color: transparent; box-shadow: ${softEdge}; transition-duration: 180ms; }\n` +
        `${popupButtons.map(s => `${s}:hover`).join(', ')} { background-color: ${hover}; color: ${text}; }\n` +
        `${popupButtons.map(s => `${s}:active`).join(', ')} { background-color: ${pressed}; color: ${text}; }\n` +
        `${popupButtons.map(s => `${s}:focus`).join(', ')} { background-color: ${hover}; color: ${text}; box-shadow: inset 0 0 0 2px ${focus} !important; }\n` +
        `${popupButtons.map(s => `${s}:checked`).join(', ')} { background-color: ${accentBg}; color: ${accentFg}; }\n` +
        `${popupButtons.map(s => `${s}:insensitive`).join(', ')} { color: ${secondary}; background-color: ${rgba(control, .55)}; }\n` +
        `${flatButtons.join(', ')} { background-color: transparent; color: ${text}; transition-duration: 160ms; }\n` +
        `.calendar .calendar-month-header .calendar-month-label { color: ${text} !important; }\n` +
        `.calendar .calendar-day-heading, .calendar .calendar-day.calendar-weekend, .datemenu-today-button .day-label { color: ${secondary}; }\n` +
        `.calendar .calendar-day.calendar-other-month, .calendar .calendar-day.calendar-other-month.calendar-weekend { color: ${rgba(text, .45)}; }\n` +
        `.calendar .calendar-week-number { background-color: ${rgba(text, .10)}; color: ${secondary}; }\n` +
        `.datemenu-today-button .date-label { color: ${text}; }\n` +
        `.calendar .calendar-day:hover, .calendar .calendar-month-header .pager-button:hover, .datemenu-today-button:hover, .popup-menu .icon-button.flat:hover, .popup-menu .button.flat:hover { background-color: ${hover}; color: ${text}; }\n` +
        `.calendar .calendar-day:active, .calendar .calendar-month-header .pager-button:active, .datemenu-today-button:active, .popup-menu .icon-button.flat:active, .popup-menu .button.flat:active { background-color: ${pressed}; color: ${text}; }\n` +
        `.calendar .calendar-day:insensitive, .calendar .calendar-month-header .pager-button:insensitive, .datemenu-today-button:insensitive { color: ${secondary}; background-color: transparent; }\n` +
        `.calendar .calendar-day:selected, .app-well-app:focus .overview-icon, .search-provider-icon:focus { background-color: ${selected}; color: ${text}; }\n` +
        `.calendar .calendar-day.calendar-today { background-color: ${accentBg}; color: ${accentFg} !important; }\n` +
        `.calendar .calendar-day.calendar-today:hover { background-color: ${mix(accentBg, accentFg, .10)}; color: ${accentFg} !important; }\n` +
        `.calendar .calendar-day.calendar-day-with-events { background-image: url("resource:///org/gnome/shell/theme/calendar-today${dark ? '' : '-light'}.svg"); }\n` +
        `.calendar .calendar-day.calendar-today.calendar-day-with-events { background-image: url("resource:///org/gnome/shell/theme/calendar-today${accentFg === '#191919' ? '-light' : ''}.svg") !important; }\n` +
        // Cover all Shell entries (Polkit password, Run dialog, Popups, Search, App Folders)
        // including hover, focus, insensitive, placeholder hint-text and peek-password icons.
        `${entries.join(', ')} { background-color: ${control}; color: ${text}; border-color: ${rgba(text, .14)}; selection-background-color: ${accentBg}; selected-color: ${accentFg}; transition-duration: 180ms; }\n` +
        `${entries.map(s => `${s}:hover`).join(', ')} { background-color: ${hover}; color: ${text}; }\n` +
        `${entries.map(s => `${s}:focus`).join(', ')} { background-color: ${entryFocusBg}; color: ${text}; border-color: ${focus}; box-shadow: inset 0 0 0 2px ${focus} !important; }\n` +
        `${entries.map(s => `${s}:insensitive`).join(', ')} { background-color: ${rgba(control, .60)}; color: ${secondary}; }\n` +
        `${entryHints.join(', ')} { color: ${secondary} !important; }\n` +
        `${entryIcons.join(', ')} { color: ${secondary}; }\n` +
        `${entryIcons.map(s => `${s}:hover`).join(', ')} { color: ${text}; }\n` +
        `.popup-menu .toggle-switch, .modal-dialog .toggle-switch { background: ${rgba(text, .16)}; color: ${text}; transition-duration: 180ms; }\n` +
        `.popup-menu .toggle-switch:hover, .modal-dialog .toggle-switch:hover { background: ${rgba(text, .22)}; }\n` +
        `.popup-menu .toggle-switch:checked, .modal-dialog .toggle-switch:checked { background: ${accentBg}; color: ${accentFg}; }\n` +
        `.popup-menu .toggle-switch .handle, .modal-dialog .toggle-switch .handle { background: #ffffff; }\n` +
        `.popup-menu .check-box StIcon, .modal-dialog .check-box StIcon { border-color: ${rgba(text, .20)}; color: ${text}; }\n` +
        `.popup-menu .check-box:checked StIcon, .modal-dialog .check-box:checked StIcon { background-color: ${accentBg}; border-color: ${accentBg}; color: ${accentFg}; }\n` +
        `.popup-menu StScrollBar > .vhandle, .popup-menu StScrollBar > .hhandle { background-color: ${rgba(text, .28)}; }\n` +
        `.popup-menu StScrollBar > .vhandle:hover, .popup-menu StScrollBar > .hhandle:hover { background-color: ${rgba(text, .42)}; }\n` +
        `.popup-menu-item:focus, .calendar .calendar-day:focus, .message:focus, .modal-dialog .button:focus, .search-entry:focus { box-shadow: inset 0 0 0 2px ${focus} !important; }\n` +
        menuRoles(tokens, dark, [''], '', false, opt);
}

function panelStates(tokens, dark, scope = '#panel', important = '') {
    // Use GNOME Shell's inset box-shadow pill model uniformly across hover and
    // active states so panel buttons keep identical geometry and a translucent,
    // theme-harmonized glass feel without opaque jumps or hitbox shrinking.
    const {text, accent} = tokens;
    const pillTint = dark ? mix('#ffffff', accent, .22) : mix('#191919', accent, .24);
    const pillHover = rgba(pillTint, dark ? .14 : .10);
    const pillActive = rgba(pillTint, dark ? .23 : .17);
    const pillActiveHover = rgba(pillTint, dark ? .28 : .22);
    const ovScopes = [':overview', '.unlock-screen', '.login-screen'].map(state => `${scope}${state}`);
    const ovJoin = parts => ovScopes.flatMap(o => parts.map(part => `${o} ${part}`)).join(', ');
    const clockReset = prefix => ['', ':hover', ':focus', ':active', ':checked', ':active:hover', ':checked:hover']
        .map(state => `${prefix} .panel-button.clock-display${state}`).join(', ');
    const alerts = [scope, ...ovScopes];
    const alertRule = (cls, states, color, bgZero = false) =>
        `${alerts.flatMap(p => states.map(s => `${p} .panel-button.${cls}${s}`)).join(', ')} { ` +
        `${bgZero ? `background-color: transparent${important}; ` : ''}box-shadow: inset 0 0 0 100px ${color}${important}; color: #ffffff${important}; }\n`;
    return `${scope} .panel-button, ${scope} .panel-button.clock-display, ${scope} .panel-button.clock-display .clock, ${scope} .netSpeedLabel { color: ${text}${important}; text-shadow: none${important}; icon-shadow: none${important}; background-color: transparent${important}; box-shadow: inset 0 0 0 100px transparent${important}; transition-duration: 150ms; }\n` +
        `${scope} .panel-button#panelActivities .workspace-dot, ${scope} .panel-button#panelActivities:active .workspace-dot, ${scope} .panel-button#panelActivities:checked .workspace-dot, ${scope} .panel-button#panelActivities:focus .workspace-dot { background-color: ${text}${important}; }\n` +
        `${scope} .panel-button:hover, ${scope} .panel-button:focus, ${scope} .panel-button.clock-display:hover .clock, ${scope} .panel-button.clock-display:focus .clock { background-color: transparent${important}; color: ${text}${important}; box-shadow: inset 0 0 0 100px ${pillHover}${important}; text-shadow: none${important}; }\n` +
        `${scope} .panel-button.duvar-system-menu:active, ${scope} .panel-button:active, ${scope} .panel-button:checked, ${scope} .panel-button.weatherpanel-button.active, ${scope} .panel-button.clock-display:active .clock, ${scope} .panel-button.clock-display:checked .clock { background-color: transparent${important}; color: ${text}${important}; box-shadow: inset 0 0 0 100px ${pillActive}${important}; text-shadow: none${important}; }\n` +
        `${scope} .panel-button:active:hover, ${scope} .panel-button:checked:hover, ${scope} .panel-button.weatherpanel-button.active:hover, ${scope} .panel-button.clock-display:active:hover .clock, ${scope} .panel-button.clock-display:checked:hover .clock { background-color: transparent${important}; color: ${text}${important}; box-shadow: inset 0 0 0 100px ${pillActiveHover}${important}; text-shadow: none${important}; }\n` +
        `${clockReset(scope)} { background-color: transparent${important}; box-shadow: none${important}; }\n` +
        // Overview, lock screen, and login screen sit over darkened wallpaper: keep crisp
        // pill-free white ink with delicate frosted glass feedback rather than desktop pills.
        `${ovJoin(['.panel-button', '.panel-button.clock-display', '.panel-button.clock-display .clock', '.panel-button.clock-display StIcon', '.netSpeedLabel'])} { color: #f5f5f5${important}; text-shadow: none${important}; icon-shadow: none${important}; background-color: transparent${important}; box-shadow: inset 0 0 0 100px transparent${important}; }\n` +
        `${ovJoin(['.panel-button#panelActivities .workspace-dot', '.panel-button#panelActivities:active .workspace-dot', '.panel-button#panelActivities:checked .workspace-dot', '.panel-button#panelActivities:focus .workspace-dot'])} { background-color: #f5f5f5${important}; }\n` +
        `${ovJoin(['.panel-button:hover', '.panel-button:focus', '.panel-button.clock-display:hover .clock', '.panel-button.clock-display:focus .clock'])} { background-color: transparent${important}; color: #f5f5f5${important}; box-shadow: inset 0 0 0 100px rgba(255,255,255,0.14)${important}; }\n` +
        `${ovJoin(['.panel-button.duvar-system-menu:active', '.panel-button:active', '.panel-button:checked', '.panel-button.weatherpanel-button.active', '.panel-button.clock-display:active .clock', '.panel-button.clock-display:checked .clock'])} { background-color: transparent${important}; color: #f5f5f5${important}; box-shadow: inset 0 0 0 100px rgba(255,255,255,0.22)${important}; }\n` +
        `${ovJoin(['.panel-button:active:hover', '.panel-button:checked:hover', '.panel-button.weatherpanel-button.active:hover', '.panel-button.clock-display:active:hover .clock', '.panel-button.clock-display:checked:hover .clock'])} { background-color: transparent${important}; color: #f5f5f5${important}; box-shadow: inset 0 0 0 100px rgba(255,255,255,0.27)${important}; }\n` +
        `${ovScopes.map(clockReset).join(', ')} { background-color: transparent${important}; box-shadow: none${important}; }\n` +
        alertRule('screen-recording-indicator', [''], '#c01c28', true) +
        alertRule('screen-recording-indicator', [':hover', ':focus'], '#d61f2d') +
        alertRule('screen-recording-indicator', [':active', ':checked'], '#e02a37') +
        alertRule('screen-recording-indicator', [':active:hover', ':checked:hover'], '#e13340') +
        alertRule('screen-sharing-indicator', [''], '#ff7800', true) +
        alertRule('screen-sharing-indicator', [':hover', ':focus'], '#ff861a') +
        alertRule('screen-sharing-indicator', [':active', ':checked'], '#ff902e') +
        alertRule('screen-sharing-indicator', [':active:hover', ':checked:hover'], '#ff9638');
}

export function blurAccentCss(tokens, dark, opts = {}) {
    const panelOpts = normalizePanelOptions(opts);
    const {text} = tokens;
    const {popover, card, control, hover, pressed, selected, panel: panelColor, accentBg, accentFg, focus, secondary} = layers(tokens, dark);
    const opt = opticalProfile(tokens, dark);
    const pill = rgba(panelColor, opt.contrastedPillAlpha);
    const pillHover = rgba(hover, Math.min(0.94, opt.contrastedPillAlpha + 0.08));
    const pillActive = rgba(pressed, Math.min(0.96, opt.contrastedPillAlpha + 0.14));
    const pillActiveHover = rgba(pressed, Math.min(0.98, opt.contrastedPillAlpha + 0.18));
    const hairline = rgba(text, dark ? .09 : .10);
    const panelTint = dark
        // In dark mode leave explicit #panel.light-panel free for BMS's Light style.
        ? `#panel.dark-panel { background-color: ${rgba(panelColor, opt.panelBgAlpha)} !important; }\n`
        // On GNOME Shell 51 "default", BMS Auto emits .dark-panel; tint both light and auto-dark panels.
        : `#panel.light-panel, #panel.dark-panel { background-color: ${rgba(panelColor, opt.panelBgAlpha)} !important; }\n`;
    const explicitLightPanelInDark = dark
        ? panelScopeRule(['#panel.light-panel', '#panel.light-panel:overview'], '#191919',
            'rgba(0,0,0,0.10)', 'rgba(0,0,0,0.17)', 'rgba(0,0,0,0.22)')
        : panelScopeRule(['#panel.light-panel:overview', '#panel.dark-panel:overview'], text,
            'rgba(0,0,0,0.10)', 'rgba(0,0,0,0.17)', 'rgba(0,0,0,0.22)');
    // Transparent and gradient panels show raw wallpaper behind panel text: adapt ink
    // per panel item (via duvar-ink-*) and fall back to strip-level ink without touching
    // .transparent-panel's transparent background or using fuzzy text-shadow halos.
    const transparentScopes = ['#panel.transparent-panel', '#panel.gradient-panel', '#panel.gradient-panel-reverse'];
    const capsuleScopes = ['#panel.duvar-panel-capsule', ...transparentScopes];
    const adaptiveTransparentPanel =
        (opt.hasOptical
            ? panelScopeRule(
                transparentScopes,
                opt.topInk, opt.topPillHover, opt.topPillActive, opt.topPillActiveHover,
                {textShadow: 'none', iconShadow: 'none'})
            : '') +
        panelButtonInkRules(tokens, dark, capsuleScopes, ' !important', panelOpts) +
        `#panel.duvar-panel-scrim { background-color: transparent !important; background-gradient-direction: vertical !important; background-gradient-start: ${rgba(opt.gradientBase, Math.min(0.66, opt.gradientAlpha))} !important; background-gradient-end: ${rgba(opt.gradientBase, 0)} !important; }\n` +
        `#panel.duvar-panel-scrim:overview, #panel.duvar-panel-scrim.unlock-screen, #panel.duvar-panel-scrim.login-screen { background-color: transparent !important; background-gradient-start: transparent !important; background-gradient-end: transparent !important; }\n`;
    const lightTextOverrideClasses = ['duvar-ink-light', 'duvar-ink-dark', 'duvar-ink-pill-light', 'duvar-ink-pill-dark'];
    const lightTextPerButton = lightTextOverrideClasses.map(cls =>
        `.panel-light-text #panel.transparent-panel .panel-button.${cls}, .panel-light-text #panel.transparent-panel .panel-button.${cls} .clock, .panel-light-text #panel.transparent-panel .panel-button.${cls} .system-status-icon, .panel-light-text #panel.transparent-panel .panel-button.${cls} StIcon { color: #f6f5f4 !important; box-shadow: inset 0 0 0 100px transparent !important; }\n` +
        `.panel-light-text #panel.transparent-panel .panel-button.${cls}#panelActivities .workspace-dot { background-color: #f6f5f4 !important; }\n`
    ).join('');
    const lightTextDarkPanel = dark ? '' :
        `.panel-light-text #panel.dark-panel { background-color: rgba(30,30,33,0.50) !important; }\n` +
        `.panel-light-text #panel.gradient-panel { background-color: transparent !important; background-gradient-direction: vertical !important; background-gradient-start: rgba(0,0,0,0.48) !important; background-gradient-end: rgba(0,0,0,0) !important; }\n` +
        `.panel-light-text #panel.gradient-panel-reverse { background-color: transparent !important; background-gradient-direction: vertical !important; background-gradient-start: rgba(0,0,0,0) !important; background-gradient-end: rgba(0,0,0,0.48) !important; }\n`;
    // Preserve BMS blur, glass translucency, gradient fade-to-0, overview blur, and explicit panel styles.
    // St/libcroco does not support :not(), so cascade order separates desktop, transparent, contrasted, and BMS override modes.
    const alertScopes = [
        '#panel.transparent-panel', '#panel.gradient-panel', '#panel.gradient-panel-reverse',
        '#panel.duvar-panel-capsule', '#panel.duvar-panel-scrim',
        '#panel.contrasted-panel', '#panel.light-panel', '#panel.light-panel:overview',
        '#panel.dark-panel:overview', '.panel-light-text #panel', '.bms-panel-overview-light-theme #panel:overview',
    ];
    const panel =
        panelTint +
        `#panel.gradient-panel { background-color: transparent !important; background-gradient-direction: vertical !important; background-gradient-start: ${rgba(opt.gradientBase, opt.gradientAlpha)} !important; background-gradient-end: ${rgba(opt.gradientBase, 0)} !important; }\n` +
        `#panel.gradient-panel-reverse { background-color: transparent !important; background-gradient-direction: vertical !important; background-gradient-start: ${rgba(opt.gradientBase, 0)} !important; background-gradient-end: ${rgba(opt.gradientBase, opt.gradientAlpha)} !important; }\n` +
        `#panel.unlock-screen, #panel.login-screen { background-color: transparent !important; background-gradient-start: transparent !important; background-gradient-end: transparent !important; }\n` +
        panelStates(tokens, dark, '#panel', ' !important') +
        explicitLightPanelInDark +
        adaptiveTransparentPanel +
        // Contrasted panel pills come after panelStates so hover/active/checked pills are preserved.
        `#panel.contrasted-panel .panel-button, #panel.contrasted-panel .panel-button.clock-display .clock, #panel.contrasted-panel .panel-button.clock-display StIcon { background-color: transparent !important; color: ${text} !important; box-shadow: inset 0 0 0 100px ${pill} !important; }\n` +
        panelScopeRule(['#panel.contrasted-panel'], text, pillHover, pillActive, pillActiveHover) +
        // Respect BMS's explicit user toggles for forced light panel text and light-theme overview ink.
        lightTextDarkPanel +
        panelScopeRule(['.panel-light-text #panel', '.panel-light-text #panel.dark-panel', '.panel-light-text #panel.dark-panel:overview', '.panel-light-text #panel.transparent-panel'],
            '#f6f5f4', 'rgba(255,255,255,0.14)', 'rgba(255,255,255,0.22)', 'rgba(255,255,255,0.27)') +
        lightTextPerButton +
        panelScopeRule(['.bms-panel-overview-light-theme #panel:overview'],
            opt.overviewLightInk, opt.overviewLightPills[0], opt.overviewLightPills[1], opt.overviewLightPills[2]) +
        `${alertScopes.map(p => `${p} .panel-button.screen-recording-indicator`).join(', ')} { background-color: transparent !important; box-shadow: inset 0 0 0 100px #c01c28 !important; color: #ffffff !important; }\n` +
        `${alertScopes.map(p => `${p} .panel-button.screen-sharing-indicator`).join(', ')} { background-color: transparent !important; box-shadow: inset 0 0 0 100px #ff7800 !important; color: #ffffff !important; }\n`;
    const weather = [
        'hero-summary', 'hero-sun', 'tile-caption', 'section-header', 'hour-time', 'hour-rain',
        'day-summary', 'day-rain', 'day-min', 'status-label', 'provider-row', 'offline-label',
        'forecast-loading', 'day-rain-icon', 'provider-row:hover',
    ].map(role => `.weatherpanel-menu .weatherpanel-${role}`).join(', ');
    const targets = [
        '.notification-banner', '.popup-menu-content', '.candidate-popup-content',
        '.quick-settings', '.quick-toggle-menu', '.osd-window', '.resize-popup',
        '.switcher-list', '.workspace-switcher', '.modal-dialog', '.run-dialog',
        '.clipboard-dialog', '.screenshot-ui-panel', '.bms-popup-blur-target', '.message',
    ];
    const scoped = (roots, nodes) => roots.flatMap(root => nodes.map(node => `${root} ${node}.bms-popup-blurred`)).join(', ');
    // In dark mode leave explicit BMS Light popups free; in light mode cover both
    // light and dark roots because BMS Auto emits -dark on GNOME Shell 51 "default".
    const tintedModes = dark ? ['dark'] : ['light', 'dark'];
    const tintedRoots = ['background', 'surface'].flatMap(kind => tintedModes.map(mode => `.bms-popup-${kind}-${mode}`));
    const clearRoots = ['background', 'surface'].map(kind => `.bms-popup-${kind}-transparent`);
    const activeRoots = [...tintedRoots, ...clearRoots];
    const join = (roots, nodes) => roots.flatMap(root => nodes.map(node => `${root} ${node}`)).join(', ');
    const popA = opt.popoverAlpha;
    const msgA = Math.min(0.86, Math.max(opt.cardAlpha, dark ? 0.56 : 0.64));
    const listMsgA = Math.min(0.84, Math.max(opt.cardAlpha - 0.02, dark ? 0.50 : 0.62));
    const hovA = Math.min(0.92, opt.controlAlpha + 0.16);
    const pressA = Math.min(0.95, opt.controlAlpha + 0.24);
    const selA = Math.min(0.92, opt.controlAlpha + 0.22);
    const bmsEntryHosts = [
        '.popup-menu-content', '.quick-settings', '.quick-toggle-menu', '.message',
        '.modal-dialog', '.prompt-dialog', '.run-dialog', '.clipboard-dialog',
    ];
    const bmsEntryA = Math.min(0.94, opt.controlAlpha + 0.16);
    const bmsEntryFocusBg = mix(control, tokens.accent, dark ? .08 : .06);
    const bmsDialogButtons = [
        '.modal-dialog .button',
        '.modal-dialog .modal-dialog-button-box .modal-dialog-button',
        '.notification-button',
    ];
    // Double the BMS root class so specificity strictly beats BMS's !important
    // entry/dialog rules regardless of extension enable order in St.Theme.
    const boostedActiveRoots = activeRoots.map(r => `${r}${r}`);
    const bmsEntriesAndButtons =
        `${join(boostedActiveRoots, bmsEntryHosts.map(h => `${h} StEntry`))} { background-color: ${rgba(control, bmsEntryA)} !important; color: ${text} !important; }\n` +
        `${join(boostedActiveRoots, bmsEntryHosts.map(h => `${h} StEntry:hover`))} { background-color: ${rgba(hover, Math.min(0.95, bmsEntryA + 0.05))} !important; color: ${text} !important; }\n` +
        `${join(boostedActiveRoots, bmsEntryHosts.map(h => `${h} StEntry:focus`))} { background-color: ${rgba(bmsEntryFocusBg, Math.min(0.96, bmsEntryA + 0.06))} !important; color: ${text} !important; box-shadow: inset 0 0 0 2px ${focus} !important; }\n` +
        `${join(boostedActiveRoots, bmsEntryHosts.map(h => `${h} StEntry:insensitive`))} { background-color: ${rgba(control, Math.max(0.45, bmsEntryA - 0.16))} !important; color: ${secondary} !important; }\n` +
        `${join(boostedActiveRoots, bmsEntryHosts.map(h => `${h} StEntry StLabel.hint-text`))} { color: ${secondary} !important; }\n` +
        `${join(boostedActiveRoots, bmsEntryHosts.map(h => `${h} StEntry StIcon`))} { color: ${secondary} !important; }\n` +
        `${join(boostedActiveRoots, bmsEntryHosts.map(h => `${h} StEntry StIcon:hover`))} { color: ${text} !important; }\n` +
        `${join(boostedActiveRoots, bmsDialogButtons)} { background-color: ${rgba(control, opt.controlAlpha)} !important; color: ${text} !important; border-color: transparent !important; }\n` +
        `${join(boostedActiveRoots, bmsDialogButtons.map(b => `${b}:hover`))} { background-color: ${rgba(hover, hovA)} !important; color: ${text} !important; }\n` +
        `${join(boostedActiveRoots, bmsDialogButtons.map(b => `${b}:focus`))} { background-color: ${rgba(hover, hovA)} !important; color: ${text} !important; box-shadow: inset 0 0 0 2px ${focus} !important; }\n` +
        `${join(boostedActiveRoots, bmsDialogButtons.flatMap(b => [`${b}:active`, `${b}:checked`]))} { background-color: ${rgba(pressed, pressA)} !important; color: ${text} !important; }\n` +
        `${join(boostedActiveRoots, bmsDialogButtons.map(b => `${b}:insensitive`))} { background-color: ${rgba(control, Math.max(0.38, opt.controlAlpha - 0.14))} !important; color: ${secondary} !important; }\n`;
    // Leave BMS's Transparent popup style transparent and Liquid Glass borderless;
    // tint blurred surfaces, unblurred fallbacks, and embedded cards/submenus.
    const bmsSurfaces =
        `${scoped(tintedRoots, targets)} { background-color: ${rgba(popover, popA)} !important; color: ${text}; border-color: ${hairline}; }\n` +
        `${scoped(tintedRoots, ['.modal-dialog', '.run-dialog', '.clipboard-dialog'])} { background-color: ${rgba(popover, opt.modalAlpha)} !important; color: ${text}; border-color: ${hairline}; }\n` +
        `${scoped(clearRoots, targets)} { color: ${text}; }\n` +
        `${join(tintedRoots, ['.bms-popup-unblurred'])} { background-color: ${popover} !important; color: ${text}; border-color: ${hairline}; }\n` +
        `${scoped(tintedRoots, ['.message'])} { background-color: ${rgba(card, msgA)} !important; border-color: transparent; }\n` +
        `${tintedRoots.flatMap(root => [`${root} .message.bms-popup-blurred:hover`, `${root} .message.bms-popup-blurred:focus`]).join(', ')} { background-color: ${rgba(hover, hovA)} !important; }\n` +
        `${join(tintedRoots, ['.message-list .message'])} { background-color: ${rgba(card, listMsgA)} !important; color: ${text}; border-color: transparent; }\n` +
        `${join(tintedRoots, ['.message-list .message:hover', '.message-list .message:focus'])} { background-color: ${rgba(hover, hovA)} !important; }\n` +
        `${join(tintedRoots, ['.message-list .message:active'])} { background-color: ${rgba(pressed, pressA)} !important; }\n` +
        `${join(tintedRoots, ['.quick-toggle-menu'])} { background-color: ${rgba(card, opt.cardAlpha)} !important; color: ${text}; border-color: ${hairline}; }\n` +
        `${join(activeRoots, ['.popup-sub-menu > StBoxLayout'])} { background-color: ${rgba(text, dark ? .07 : .06)} !important; }\n` +
        `${join(activeRoots, ['.popup-menu-item', '.popup-sub-menu .popup-menu-item'])} { color: ${text} !important; }\n` +
        `${join(activeRoots, ['.popup-menu-item:hover', '.popup-menu-item:selected', '.popup-menu-item:focus', '.popup-sub-menu .popup-menu-item:hover', '.popup-sub-menu .popup-menu-item:selected', '.popup-sub-menu .popup-menu-item:focus'])} { background-color: ${rgba(hover, hovA)} !important; color: ${text} !important; }\n` +
        `${join(activeRoots, ['.popup-menu-item:active', '.popup-sub-menu .popup-menu-item:active'])} { background-color: ${rgba(pressed, pressA)} !important; color: ${text} !important; }\n` +
        `${join(activeRoots, ['.popup-menu-item:insensitive', '.popup-sub-menu .popup-menu-item:insensitive'])} { color: ${secondary} !important; }\n` +
        bmsEntriesAndButtons +
        `${join(activeRoots, ['.datemenu-today-button StLabel', '.datemenu-today-button .date-label', '.calendar .calendar-month-header .calendar-month-label', '.calendar .calendar-month-header .pager-button', '.calendar .calendar-day'])} { color: ${text} !important; }\n` +
        `${join(activeRoots, ['.datemenu-today-button .day-label', '.calendar .calendar-day-heading', '.calendar .calendar-day.calendar-weekend'])} { color: ${secondary} !important; }\n` +
        `${join(activeRoots, ['.calendar .calendar-day.calendar-other-month', '.calendar .calendar-day.calendar-other-month.calendar-weekend'])} { color: ${rgba(text, .45)} !important; }\n` +
        `${join(activeRoots, ['.datemenu-today-button:hover', '.calendar .calendar-day:hover', '.calendar .calendar-month-header .pager-button:hover'])} { background-color: ${rgba(hover, hovA)} !important; color: ${text} !important; }\n` +
        `${join(activeRoots, ['.datemenu-today-button:active', '.datemenu-today-button:checked', '.calendar .calendar-day:selected', '.calendar .calendar-day:checked', '.calendar .calendar-day:active', '.calendar .calendar-month-header .pager-button:active'])} { background-color: ${rgba(selected, selA)} !important; color: ${text} !important; }\n` +
        `${join(activeRoots, ['.calendar .calendar-day.calendar-today', '.calendar .calendar-day.calendar-today:selected', '.calendar .calendar-day.calendar-today:checked'])} { background-color: ${accentBg} !important; color: ${accentFg} !important; }\n` +
        `${join(activeRoots, ['.calendar .calendar-day.calendar-today:hover', '.calendar .calendar-day.calendar-today:checked:hover', '.calendar .calendar-day.calendar-today:selected:hover'])} { background-color: ${mix(accentBg, accentFg, .10)} !important; color: ${accentFg} !important; }\n` +
        `${join(activeRoots, ['.datemenu-today-button:focus', '.calendar .calendar-day:focus', '.calendar .calendar-month-header .pager-button:focus'])} { box-shadow: inset 0 0 0 2px ${focus} !important; }\n` +
        `${join(activeRoots, ['StScrollBar > .vhandle', 'StScrollBar > .hhandle'])} { background-color: ${rgba(text, .28)}; }\n` +
        `${join(activeRoots, ['StScrollBar > .vhandle:hover', 'StScrollBar > .hhandle:hover'])} { background-color: ${rgba(text, .42)}; }\n` +
        `${activeRoots.map(root => `${root} .message-list .message-list-clear-button.button`).join(', ')} { background-color: ${rgba(control, opt.controlAlpha)} !important; color: ${text} !important; border-color: transparent !important; }\n` +
        `${activeRoots.map(root => `${root} .message-list .message-list-clear-button.button:hover`).join(', ')} { background-color: ${rgba(hover, hovA)} !important; }\n` +
        `${activeRoots.map(root => `${root} .message-list .message-list-clear-button.button:active`).join(', ')} { background-color: ${rgba(pressed, pressA)} !important; }\n` +
        `.bms-popup-blurred.bms-popup-blur-parent { background-color: transparent !important; }\n`;
    // Keep Overview search, App Folder dialogs, and Dash-to-Dock harmonized without
    // overriding BMS Transparent styles (or explicit Light styles when in dark mode).
    const ovClasses = tintedModes.map(m => `.overview-components-${m}`);
    const ovSearch = ovClasses.flatMap(c => [`.search-entry${c}`, `${c} #overviewGroup .search-entry`]);
    const afClasses = tintedModes.map(m => `.appfolder-dialogs-${m}`);
    const dashClasses = tintedModes.map(m => `.${m}-dash`);
    const dashBg = dashClasses.flatMap(c => [
        `${c} .dash-background`, `${c}.dash-background`, `#dash${c} > .dash-background${c}`,
        `#dash${c} .dash-background`, `StBoxLayout > StWidget > #dash${c} > .dash-background`,
        `${c} .multimonitor-dock .dash-background`, `.multimonitor-dock${c} .dash-background`,
        `${c}.plank-like-dock-bg`,
    ]);
    const overviewLight = dark ? '' :
        `${join(ovClasses, ['#overviewGroup .search-section-content', '#overviewGroup .search-section-content StLabel', '#overviewGroup .list-search-result', '#overviewGroup .list-search-result StLabel', '#overviewGroup .list-search-result-title', '#overviewGroup .search-provider-icon', '#overviewGroup .search-provider-icon StLabel', '#overviewGroup .list-search-provider-details', '#overviewGroup .page-navigation-arrow > StIcon', '#overviewGroup .search-section .overview-tile', '#overviewGroup .search-section .overview-tile StLabel', '#overviewGroup .search-section .grid-search-result', '#overviewGroup .search-section .grid-search-result StLabel'])} { color: ${text} !important; }\n` +
        `${join(ovClasses, ['#overviewGroup .list-search-result .list-search-result-description', '#overviewGroup .search-statusbox', '#overviewGroup .search-statusbox StIcon', '#overviewGroup .search-statustext'])} { color: ${secondary} !important; }\n` +
        `${join(ovClasses, ['#overviewGroup .search-section .app-grid-running-dot'])} { background-color: ${secondary} !important; }\n` +
        `${join(dashClasses, ['.dash-separator'])} { background-color: ${rgba(text, .12)} !important; }\n` +
        `${dashClasses.flatMap(c => ['.app-grid-running-dot', '.app-well-app-running-dot', '.running-dot'].flatMap(d => [`${c} ${d}`, `#dash${c} ${d}`, `.multimonitor-dock${c} ${d}`])).join(', ')} { background-color: ${text} !important; color: ${text} !important; }\n` +
        `${dashClasses.flatMap(c => ['.show-apps-icon', '.show-apps .overview-icon'].flatMap(s => [`${c} ${s}`, `#dash${c} ${s}`, `.multimonitor-dock${c} ${s}`])).join(', ')} { color: ${text} !important; }\n`;
    const appFolderLight = dark ? '' :
        `${join(afClasses, ['.folder-name-container .folder-name-entry', '.folder-name-container .folder-name-label', '.folder-name-entry', '.folder-name-entry StLabel', '.folder-name-entry StEntry', '.overview-tile', '.overview-tile StLabel', '.app-well-app', '.app-well-app StLabel'])} { color: ${text} !important; }\n` +
        `${join(afClasses, ['.edit-folder-button', '.icon-button', '.page-navigation-arrow'])} { color: ${text} !important; background-color: ${rgba(control, opt.controlAlpha)} !important; }\n` +
        `${join(afClasses, ['.edit-folder-button StIcon', '.icon-button StIcon', '.page-navigation-arrow StIcon', '.page-navigation-arrow > StIcon', '.page-navigation-arrow StWidget'])} { color: ${text} !important; }\n` +
        `${join(afClasses, ['.edit-folder-button:hover', '.icon-button:hover', '.page-navigation-arrow:hover'])} { background-color: ${rgba(hover, hovA)} !important; }\n` +
        `${join(afClasses, ['.edit-folder-button:active', '.icon-button:active', '.page-navigation-arrow:active'])} { background-color: ${rgba(pressed, pressA)} !important; }\n` +
        `${join(afClasses, ['.overview-tile:hover', '.overview-tile:focus'])} { background-color: ${rgba(hover, Math.min(0.84, opt.cardAlpha + 0.04))} !important; }\n` +
        `${join(afClasses, ['.overview-tile:active'])} { background-color: ${rgba(pressed, hovA)} !important; }\n`;
    const overview =
        `${ovSearch.join(', ')} { background-color: ${rgba(popover, opt.searchAlpha)} !important; color: ${text} !important; border: 1px solid ${rgba(text, .12)} !important; box-shadow: none !important; transition-duration: 180ms; }\n` +
        `${ovSearch.map(s => `${s}:hover`).join(', ')} { background-color: ${rgba(popover, Math.min(0.92, opt.searchAlpha + 0.10))} !important; }\n` +
        `${ovSearch.map(s => `${s}:focus`).join(', ')} { background-color: ${rgba(popover, Math.min(0.95, opt.searchAlpha + 0.16))} !important; border-color: ${focus} !important; box-shadow: inset 0 0 0 1px ${focus} !important; }\n` +
        `${ovSearch.flatMap(s => [`${s} .search-entry-icon`, `${s} StLabel.hint-text`]).join(', ')} { color: ${secondary} !important; }\n` +
        `${join(ovClasses, ['#overviewGroup .search-section-content', '#overviewGroup .page-navigation-arrow'])} { background-color: ${rgba(popover, Math.max(0.56, opt.searchAlpha - 0.04))} !important; border-color: ${hairline}; }\n` +
        `${dashBg.join(', ')} { background-color: ${rgba(panelColor, opt.dashAlpha)} !important; border-color: ${hairline}; }\n` +
        overviewLight +
        `${afClasses.map(c => `.app-folder-dialog${c}`).join(', ')} { background-color: ${rgba(popover, Math.max(0.60, opt.popoverAlpha - 0.04))} !important; color: ${text}; border-color: ${hairline}; }\n` +
        appFolderLight;
    return panel + `${weather} { color: ${secondary}; }\n` + bmsSurfaces + overview +
        menuRoles(tokens, dark, activeRoots.map(root => `${root} `), ' !important', true, opt);
}

export const validColors = c => c && ['accent', 'surface', 'text']
    .every(key => isHex(c[key]));

export function gtkCss(tokens, dark, gtk4, includeBase = true, allowAtRules = includeBase) {
    if (!validColors(tokens)) throw new Error('Invalid GTK palette');
    const {accent, surface, text} = tokens;
    const base = gtk4 ? `Default/gtk-${dark ? 'dark' : 'light'}.css`
        : `Adwaita/gtk-contained${dark ? '-dark' : ''}.css`;
    const {view, chrome, popover, sidebar, card, control, hover, pressed, selected, accentBg, accentFg, secondary, border, frame, focus, controlEdge} = layers(tokens, dark);
    const parts = {window: surface, view, headerbar: chrome, popover, dialog: popover, sidebar, card,
        'secondary-sidebar': mix(sidebar, view, .45), thumbnail: card};
    const headerBackdrop = mix(chrome, surface, .75);
    const sidebarBackdrop = mix(sidebar, surface, .45);
    const secSidebarBackdrop = mix(parts['secondary-sidebar'], surface, .45);
    const vars = Object.entries(parts).map(([part, bg]) => `--${part}-bg-color: ${bg}; --${part}-fg-color: ${text};`).join('\n');
    const defineColors = allowAtRules ? [
        ['accent_color', accent],
        ['accent_bg_color', accentBg],
        ['accent_fg_color', accentFg],
        ['window_bg_color', surface],
        ['window_fg_color', text],
        ['view_bg_color', view],
        ['view_fg_color', text],
        ['headerbar_bg_color', chrome],
        ['headerbar_fg_color', text],
        ['headerbar_border_color', border],
        ['headerbar_backdrop_color', headerBackdrop],
        ['headerbar_shade_color', rgba(border, .32)],
        ['card_bg_color', card],
        ['card_fg_color', text],
        ['card_shade_color', rgba(border, .28)],
        ['dialog_bg_color', popover],
        ['dialog_fg_color', text],
        ['popover_bg_color', popover],
        ['popover_fg_color', text],
        ['popover_shade_color', rgba(border, .30)],
        ['sidebar_bg_color', sidebar],
        ['sidebar_fg_color', text],
        ['sidebar_backdrop_color', sidebarBackdrop],
        ['sidebar_border_color', border],
        ['sidebar_shade_color', rgba(border, .28)],
        ['secondary_sidebar_bg_color', parts['secondary-sidebar']],
        ['secondary_sidebar_fg_color', text],
        ['secondary_sidebar_backdrop_color', secSidebarBackdrop],
        ['thumbnail_bg_color', card],
        ['thumbnail_fg_color', text],
    ].map(([name, val]) => `@define-color ${name} ${val};`).join('\n') + '\n' : '';
    const windowEdge = (includeBase ?
        'window.csd:not(.popup):not(.maximized):not(.fullscreen):not(.tiled):not(.tiled-top):not(.tiled-left):not(.tiled-right):not(.tiled-bottom) { outline-style: solid; outline-width: 1px; outline-offset: -1px; }\n' : '') +
        `window.csd { outline-color: ${frame}; }\nwindow.csd:backdrop { outline-color: ${border}; }\n`;
    if (gtk4 && !includeBase) {
        const contentBar = [
            'navigation-split-view .content-pane toolbarview > .top-bar:not(.raised)',
            'overlay-split-view > widget.content-pane > toolbarview > .top-bar:not(.raised)',
        ];
        // libadwaita already knows which nodes to paint. In particular, the outer
        // popover and dialog host must stay transparent for corners and shadows.
        const fields = 'entry:not(:disabled), spinbutton:not(:disabled), searchentry:not(:disabled)';
        return defineColors +
            `:root {\n${vars}\n--dim-opacity: 76%; --headerbar-backdrop-color: ${headerBackdrop}; --sidebar-backdrop-color: ${sidebarBackdrop};\n` +
            `--secondary-sidebar-backdrop-color: ${secSidebarBackdrop};\n` +
            `--headerbar-border-color: ${rgba(border, .65)}; --headerbar-shade-color: ${rgba(border, .32)};\n` +
            `--sidebar-border-color: ${rgba(border, .65)}; --sidebar-shade-color: ${rgba(border, .28)};\n` +
            `--secondary-sidebar-shade-color: ${mix(parts['secondary-sidebar'], view, .50)};\n` +
            `--card-shade-color: ${rgba(border, .28)}; --popover-shade-color: ${rgba(border, .30)};\n` +
            `--scrollbar-outline-color: ${rgba(surface, .55)};\n` +
            `--accent-color: ${accent}; --accent-standalone-color: ${accent}; --accent-bg-color: ${accentBg}; --accent-fg-color: ${accentFg};\n}\n` +
            windowEdge +
            `${contentBar.join(', ')} { background-color: var(--headerbar-bg-color); }\n` +
            `${contentBar.map(selector => `${selector}:backdrop`).join(', ')} { background-color: var(--headerbar-backdrop-color); }\n` +
            `${fields} { background-color: ${control}; color: ${text}; }\n` +
            `entry > text > placeholder, searchentry > text > placeholder { color: ${secondary}; }\n` +
            `entry:focus-visible, button:focus-visible, row:focus-visible { outline-color: ${focus}; }\n` +
            `.navigation-sidebar > row:selected:not(:disabled), .navigation-sidebar > child:selected:not(:disabled) { background-color: ${selected}; }\n` +
            `.navigation-sidebar > row:selected:hover:not(:disabled), .navigation-sidebar > child:selected:hover:not(:disabled) { background-color: ${mix(selected, text, .07)}; }\n` +
            `tooltip.background { background-color: ${popover}; color: ${text}; border: 1px solid ${border}; }\n`;
    }
    // Standard GTK3 / non-libadwaita GTK4 themes: include full @define-color tokens so
    // third-party toolkits (Firefox, LibreOffice, GIMP, Inkscape, Electron, libhandy)
    // and standard GTK widgets inherit exact surface, border and accent roles.
    const gtk3CompatDefines = gtk4 ? '' : [
        ['theme_bg_color', surface],
        ['theme_fg_color', text],
        ['theme_base_color', view],
        ['theme_text_color', text],
        ['theme_selected_bg_color', accentBg],
        ['theme_selected_fg_color', accentFg],
        ['theme_unfocused_bg_color', headerBackdrop],
        ['theme_unfocused_fg_color', secondary],
        ['theme_unfocused_base_color', view],
        ['theme_unfocused_text_color', text],
        ['theme_unfocused_selected_bg_color', selected],
        ['theme_unfocused_selected_fg_color', text],
        ['insensitive_bg_color', mix(surface, control, .5)],
        ['insensitive_fg_color', secondary],
        ['insensitive_base_color', mix(view, surface, .5)],
        ['borders', border],
        ['unfocused_borders', rgba(border, .75)],
    ].map(([name, val]) => `@define-color ${name} ${val};`).join('\n') + '\n';
    const popup = gtk4 ? 'popover > contents, popover > arrow' : 'popover.background, menu, .menu';
    const focusState = gtk4 ? ':focus-visible' : ':focus';
    const placeholder = gtk4 ? 'entry > text > placeholder, searchentry > text > placeholder' : 'entry placeholder';
    const checks = gtk4 ? 'checkbutton:checked > check, radiobutton:checked > radio' : 'check:checked, radio:checked';
    const gtk3Extras = gtk4 ? '' :
        `menubar, .menubar, toolbar, .toolbar { background-color: ${chrome}; color: ${text}; border-color: ${border}; }\n` +
        `menuitem:hover, menu > menuitem:hover { background-color: ${hover}; color: ${text}; }\n` +
        `notebook > header { background-color: ${sidebar}; border-color: ${border}; }\n` +
        `notebook > header tab:checked { background-color: ${surface}; color: ${text}; border-color: ${accentBg}; }\n` +
        `button:not(.flat):not(.suggested-action):not(.destructive-action):not(:disabled) { background-image: none; background-color: ${control}; color: ${text}; border-color: ${controlEdge}; }\n` +
        `button:not(.flat):not(.suggested-action):not(.destructive-action):hover { background-color: ${hover}; color: ${text}; }\n` +
        `button:not(.flat):not(.suggested-action):not(.destructive-action):active, button:not(.flat):not(.suggested-action):not(.destructive-action):checked { background-color: ${pressed}; color: ${text}; }\n` +
        `button:disabled { color: ${secondary}; }\n` +
        `treeview.view:selected, placessidebar row:selected { background-color: ${selected}; color: ${text}; }\n`;
    return (includeBase ? `@import url("resource:///org/gtk/libgtk/theme/${base}");\n` : '') +
        defineColors + gtk3CompatDefines +
        `window.background { background-color: ${surface}; color: ${text}; }\n` +
        (gtk4 ? windowEdge : `window.csd decoration { border-color: ${frame}; }\nwindow.csd:backdrop decoration { border-color: ${border}; }\n`) +
        `.view, textview > text { background-color: ${view}; color: ${text}; }\n` +
        `headerbar { background-color: ${chrome}; color: ${text}; border-color: ${border}; }\n` +
        `headerbar:backdrop { background-color: ${headerBackdrop}; color: ${secondary}; }\n` +
        `.sidebar, navigation-sidebar, placessidebar { background-color: ${sidebar}; color: ${text}; border-color: ${border}; }\n` +
        `.card, .boxed-list { background-color: ${card}; color: ${text}; border-color: ${border}; }\n` +
        `${popup} { background-color: ${popover}; color: ${text}; border-color: ${border}; }\n` +
        `tooltip.background { background-color: ${popover}; color: ${text}; border-color: ${border}; }\n` +
        `separator { background-color: ${border}; }\n` +
        `row.activatable:hover:not(:selected) { background-color: ${hover}; color: ${text}; }\n` +
        `row:selected, .view:selected { background-color: ${selected}; color: ${text}; }\n` +
        `entry:not(:disabled) { background-image: none; background-color: ${control}; color: ${text}; border-color: ${controlEdge}; }\n` +
        `${placeholder} { color: ${secondary}; }\n` +
        `entry${focusState}, button${focusState} { outline-color: ${focus}; }\n` +
        `scrollbar slider { background-color: ${rgba(text, .28)}; }\n` +
        `scrollbar slider:hover { background-color: ${rgba(text, .42)}; }\n` +
        `progressbar progress, scale highlight, switch:checked, ${checks} { background-image: none; background-color: ${accentBg}; color: ${accentFg}; }\n` +
        `button.suggested-action:not(:disabled), selection { background-image: none; background-color: ${accentBg}; color: ${accentFg}; }\n` +
        gtk3Extras;
}

export function nativeAccent(hex) {
    const [r, g, b] = rgb(hex).map(v => v / 255);
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    const delta = max - min;
    if (!delta || delta / (1 - Math.abs(max + min - 1)) < .15) return 'slate';
    const hue = (360 + 60 * (max === r ? (g - b) / delta :
        max === g ? 2 + (b - r) / delta : 4 + (r - g) / delta)) % 360;
    // GNOME only offers nine presets. Classify by family, not RGB distance:
    // light lavender and dark violet should both land in purple, not pink.
    return [[14, 'red'], [43, 'orange'], [67, 'yellow'], [163, 'green'],
        [200, 'teal'], [254, 'blue'], [308, 'purple'], [344, 'pink'], [360, 'red']]
        .find(([limit]) => hue < limit)[1];
}
