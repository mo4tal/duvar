import {
    CAPSULE_STRENGTHS, PANEL_STYLES, blurAccentCss, capsuleAlphas, colors, contrast,
    gtkCss, layers, nativeAccent, opticalProfile, panelSliceMode, profiles, shellCss,
    validAnalysis, validHues,
} from '../extension/palette.js';

for (const [hex, expected] of [['#2c607d', 'blue'], ['#bea4df', 'purple'],
    ['#59417d', 'purple'], ['#807d80', 'slate'], ['#e672b2', 'pink']]) {
    if (nativeAccent(hex) !== expected)
        throw new Error(`${hex}: expected native accent ${expected}, got ${nativeAccent(hex)}`);
}

if (!validHues([null, 359.9]) || [[], [null], ['90', 0], [360, 0], [0, NaN]].some(validHues))
    throw new Error('Invalid hue validation');

const blackScene = {
    hue: null, chroma: 0,
    scene: {luma: 0.004, p15: 0.002, p85: 0.01},
    top: {luma: 0.004, p15: 0.002, p85: 0.01, zones: [0.003, 0.005, 0.003], spread: 0.008},
};
const whiteScene = {
    hue: null, chroma: 0,
    scene: {luma: 0.97, p15: 0.94, p85: 0.99},
    top: {luma: 0.97, p15: 0.94, p85: 0.99, zones: [0.96, 0.98, 0.96], spread: 0.05},
};
const mixedTopScene = {
    hue: 215, chroma: 0.45,
    scene: {luma: 0.18, p15: 0.05, p85: 0.48},
    top: {luma: 0.14, p15: 0.03, p85: 0.42, zones: [0.05, 0.14, 0.34], spread: 0.39},
};
if (!validAnalysis([blackScene, whiteScene]) || validAnalysis([null, 120]))
    throw new Error('Invalid analysis validation');

for (const hue of [null, ...Array.from({length: 360}, (_, i) => i)]) {
    for (const dark of [false, true]) {
        const all = profiles.map((_, i) => colors(hue, i, dark));
        if (new Set(all.map(c => JSON.stringify({...layers(c, dark), accent: c.accent}))).size !== profiles.length ||
            (hue !== null && new Set(all.map(c => c.accent)).size !== profiles.length))
            throw new Error(`Repeated profile at hue ${hue}, dark=${dark}`);
        for (const c of all) {
            const l = layers(c, dark);
            const inverse = dark ? '#191919' : '#f5f5f5';
            for (const bg of [c.surface, l.chrome, l.sidebar, l.view, l.card, l.popover, l.control, l.hover, l.selected, l.pressed, l.panel]) {
                if (contrast(bg, c.text) < 4.5) throw new Error(`Low text contrast at hue ${hue}`);
            }
            if (contrast(c.accent, inverse) < 4.5 || contrast(c.accent, c.surface) < 4.5)
                throw new Error(`Low accent contrast at hue ${hue}`);
            for (const [fg, bg, min] of [
                ['card', 'surface', 1.10], ['border', 'surface', 1.35], ['border', 'card', 1.15],
                ['frame', 'surface', 1.70], ['frame', 'border', 1.20],
                ...['card', 'popover', 'control', 'hover', 'pressed', 'selected'].map(bg => ['secondary', bg, 4.5]),
                ...['control', 'hover', 'pressed', 'selected', 'card', 'panel'].map(bg => ['focus', bg, 3]),
                ['controlEdge', 'control', 1.40], ['accent', 'control', 3],
            ]) {
                const ratio = contrast(l[fg] ?? c[fg], l[bg] ?? c[bg]);
                if (ratio < min) throw new Error(`${fg}/${bg} ${ratio} at hue ${hue}, profile ${c.profile}, dark=${dark}`);
            }
            if (contrast(l.frame, c.surface) > 2.6)
                throw new Error(`Harsh frame outline ${contrast(l.frame, c.surface)} at hue ${hue}, profile ${c.profile}, dark=${dark}`);
            if (l.secondary === c.text)
                throw new Error(`Secondary ink collapsed to primary text at hue ${hue}, profile ${c.profile}, dark=${dark}`);
            if ((hue === null || c.profile === 9) &&
                [c.surface, c.accent, ...Object.values(l)].some(hex => !/^#(..)\1\1$/.test(hex)))
                throw new Error(`Non-neutral role at hue ${hue}, profile ${c.profile}`);
        }
    }
}

// Verify extreme opposite-brightness wallpapers (black in light mode, white in dark mode, mixed top bar).
const lightOnBlack = colors(blackScene, 0, false);
const optLightOnBlack = opticalProfile(lightOnBlack, false);
if (optLightOnBlack.topInk !== '#f5f5f5' || optLightOnBlack.topTextShadow !== 'none')
    throw new Error(`Light mode over black wallpaper must use crisp #f5f5f5 top ink without shadow: ${JSON.stringify(optLightOnBlack)}`);
if (optLightOnBlack.popoverAlpha < 0.78 || optLightOnBlack.panelBgAlpha < 0.68)
    throw new Error(`Light mode over black wallpaper must raise frosted opacity: ${JSON.stringify(optLightOnBlack)}`);

const darkOnWhite = colors(whiteScene, 0, true);
const optDarkOnWhite = opticalProfile(darkOnWhite, true);
if (optDarkOnWhite.topInk !== '#191919' || optDarkOnWhite.topTextShadow !== 'none')
    throw new Error(`Dark mode over white wallpaper must use crisp #191919 top ink without shadow: ${JSON.stringify(optDarkOnWhite)}`);
if (optDarkOnWhite.popoverAlpha < 0.76 || optDarkOnWhite.panelBgAlpha < 0.68)
    throw new Error(`Dark mode over white wallpaper must raise frosted opacity: ${JSON.stringify(optDarkOnWhite)}`);

const lightOnMixed = colors(mixedTopScene, 0, false);
const optLightOnMixed = opticalProfile(lightOnMixed, false);
if (optLightOnMixed.topInk !== '#f5f5f5' || optLightOnMixed.topTextShadow !== 'none' || optLightOnMixed.topIconShadow !== 'none')
    throw new Error(`Mixed top strip must never use fuzzy text/icon shadow glow: ${JSON.stringify(optLightOnMixed)}`);

// Verify 16-bin per-button spatial ink + selective floating glass pill decision:
// Left dark -> 'light', center high-contrast striped -> 'pill-light'/'pill-dark', right bright -> 'dark'.
const splitBinsScene = {
    hue: 165, chroma: 0.4,
    scene: {luma: 0.25, p15: 0.04, p85: 0.72},
    top: {
        luma: 0.30, p15: 0.03, p85: 0.78, zones: [0.04, 0.35, 0.76], spread: 0.75,
        bins: [
            [0.04, 0.02, 0.08], [0.05, 0.02, 0.09], [0.06, 0.03, 0.11], [0.07, 0.03, 0.12],
            [0.35, 0.02, 0.85], [0.38, 0.03, 0.82], [0.40, 0.02, 0.88], [0.36, 0.04, 0.80],
            [0.08, 0.03, 0.14], [0.09, 0.04, 0.15], [0.10, 0.04, 0.16], [0.09, 0.03, 0.14],
            [0.65, 0.52, 0.78], [0.72, 0.60, 0.84], [0.76, 0.64, 0.88], [0.78, 0.68, 0.90],
        ],
    },
};
const splitLight = colors(splitBinsScene, 0, false);
const splitDark = colors(splitBinsScene, 0, true);
if (panelSliceMode(splitLight.optical, 0.00, 0.08, false) !== 'light')
    throw new Error('Dark left slice must select clean light ink');
if (panelSliceMode(splitLight.optical, 0.28, 0.44, false) !== 'pill-light')
    throw new Error('Striped high-contrast slice in light mode must select pill-light');
if (panelSliceMode(splitDark.optical, 0.28, 0.44, true) !== 'pill-dark')
    throw new Error('Striped high-contrast slice in dark mode must select pill-dark');
if (panelSliceMode(splitLight.optical, 0.52, 0.72, false) !== 'light')
    throw new Error('Dark green right-center slice must select clean light ink');
if (panelSliceMode(splitLight.optical, 0.82, 0.99, false) !== 'dark')
    throw new Error('Bright far-right slice must select clean dark ink');
// Verify user-selectable top-panel modes (capsule / smart / scrim / plain) in both light & dark modes.
if (panelSliceMode(splitLight.optical, 0.00, 0.08, false, 'capsule') !== 'pill-light' ||
    panelSliceMode(splitDark.optical, 0.82, 0.99, true, 'capsule') !== 'pill-dark' ||
    panelSliceMode(null, 0, 1, false, 'capsule') !== 'pill-light' ||
    panelSliceMode(null, 0, 1, true, 'capsule') !== 'pill-dark')
    throw new Error('Capsule mode must emit pill-light in light mode and pill-dark in dark mode');
if (panelSliceMode(splitLight.optical, 0.28, 0.44, false, 'plain') === 'pill-light' ||
    panelSliceMode(splitDark.optical, 0.28, 0.44, true, 'scrim') === 'pill-dark')
    throw new Error('Plain and scrim modes must not emit glass pills');
for (const dark of [false, true]) {
    const sample = colors(splitBinsScene, 0, dark);
    const subtle = capsuleAlphas(sample, dark, 'subtle');
    const balanced = capsuleAlphas(sample, dark, 'balanced');
    const crisp = capsuleAlphas(sample, dark, 'crisp');
    if (!(subtle.lightAlpha <= balanced.lightAlpha && balanced.lightAlpha <= crisp.lightAlpha) ||
        !(subtle.darkAlpha <= balanced.darkAlpha && balanced.darkAlpha <= crisp.darkAlpha))
        throw new Error(`Capsule strengths must be monotonic (dark=${dark})`);
    for (const [style] of PANEL_STYLES) {
        for (const [strength] of CAPSULE_STRENGTHS) {
            const sCss = shellCss(sample, dark, {panelStyle: style, capsuleStrength: strength});
            const bCss = blurAccentCss(sample, dark, {panelStyle: style, capsuleStrength: strength});
            if (sCss.includes(':not(') || bCss.includes(':not('))
                throw new Error(`Unsupported :not() in ${style}/${strength} (dark=${dark})`);
            if (!sCss.includes('#panel.duvar-panel-capsule') || !bCss.includes('#panel.duvar-panel-scrim'))
                throw new Error(`Missing host panel style rules in ${style}/${strength}`);
        }
    }
}

const lightBlackBms = blurAccentCss(lightOnBlack, false);
if (!/#panel\.transparent-panel \.panel-button[^}]*color: #f5f5f5 !important;/.test(lightBlackBms))
    throw new Error('Light mode on black wallpaper did not emit #f5f5f5 transparent-panel ink');
for (const cls of ['duvar-ink-light', 'duvar-ink-dark', 'duvar-ink-pill-light', 'duvar-ink-pill-dark']) {
    if (!lightBlackBms.includes(`.panel-button.${cls}`))
        throw new Error(`Missing per-button panel ink CSS rule for .${cls}`);
}
if (/text-shadow:\s+(?!none\b)/.test(lightBlackBms) || /icon-shadow:\s+(?!none\b)/.test(lightBlackBms))
    throw new Error('Transparent panel CSS must not emit text-shadow/icon-shadow glow');
const darkWhiteBms = blurAccentCss(darkOnWhite, true);
if (!/#panel\.transparent-panel \.panel-button[^}]*color: #191919 !important;/.test(darkWhiteBms))
    throw new Error('Dark mode on white wallpaper did not emit #191919 transparent-panel ink');

// Verify Polkit / modal StEntry hint-text & focus rules in light mode (fixes white-on-light placeholder bug).
const lightShell = shellCss(colors(200, 0, false), false);
for (const required of [
    'StEntry StLabel.hint-text',
    '.modal-dialog StEntry StLabel.hint-text',
    '.modal-dialog StEntry:focus',
    '.prompt-dialog StEntry',
    '.polkit-dialog-user-layout .polkit-dialog-user-label',
    '.prompt-dialog .prompt-dialog-null-label { background-color: transparent; }',
    '.polkit-dialog-user-layout .polkit-dialog-user-root-label',
    '.prompt-dialog .prompt-dialog-error-label',
]) {
    if (!lightShell.includes(required))
        throw new Error(`Light shell CSS missing dialog/entry rule: ${required}`);
}
if (/\.prompt-dialog-null-label\s*\{[^}]*background-color:\s+(?!transparent\b)/.test(lightShell))
    throw new Error('Polkit/prompt empty spacer label (.prompt-dialog-null-label) must stay transparent');
if (/\.dialog-list-item\s*\{[^}]*background-color:\s+(?!transparent\b)/.test(lightShell))
    throw new Error('Dialog list rows (.dialog-list-item) must not paint card backgrounds');

// Verify GTK3/GTK4/Libadwaita token completeness and @media safety.
const gtk3Light = gtkCss(colors(200, 0, false), false, false, true);
for (const token of ['@define-color theme_bg_color', '@define-color theme_selected_bg_color', '@define-color accent_bg_color']) {
    if (!gtk3Light.includes(token))
        throw new Error(`GTK3 CSS missing ${token}`);
}
const adwMediaDark = gtkCss(colors(200, 0, true), true, true, false, false);
if (adwMediaDark.includes('@define-color'))
    throw new Error('Libadwaita @media CSS must not contain top-level @define-color rules');
if (!adwMediaDark.includes('--accent-standalone-color:') || !adwMediaDark.includes('--window-bg-color:'))
    throw new Error('Libadwaita CSS missing expected CSS custom properties');

// Verify BMS compatibility, St/libcroco safety, and light-mode surface harmony.
for (const dark of [false, true]) {
    const tokens = colors(200, 2, dark);
    const shell = shellCss(tokens, dark);
    const bms = blurAccentCss(tokens, dark);
    if (shell.includes(':not(') || bms.includes(':not('))
        throw new Error(`Unsupported :not() selector in Shell CSS (dark=${dark})`);
    if (!/#panel\.gradient-panel \{[^}]*background-color: transparent !important;[^}]*background-gradient-end: rgba\(\d+,\d+,\d+,0\) !important;/.test(bms))
        throw new Error(`Gradient panel must fade cleanly to 0 opacity (dark=${dark})`);
    if (/^#panel:overview(?:\s*,|\s*\{)/m.test(bms))
        throw new Error(`blurAccentCss must not unconditionally force #panel:overview transparent (dark=${dark})`);
    for (const forbidden of [
        /#panel\.transparent-panel\s*\{[^}]*background(?:-color)?:/,
        /\.bms-popup-(?:background|surface)-transparent\s+\.popup-menu-content\.bms-popup-blurred\s*\{[^}]*background-color:/,
        /\.overview-components-transparent\b/,
        /\.appfolder-dialogs-transparent\b/,
        /\.transparent-dash\b/,
    ]) {
        if (forbidden.test(bms))
            throw new Error(`BMS transparent style overridden (${forbidden}, dark=${dark})`);
    }
    if (dark) {
        if (/^#panel\.light-panel\s*\{[^}]*background-color:/m.test(bms) ||
            /\.bms-popup-(?:background|surface)-light\b/.test(bms) ||
            /\.overview-components-light\b/.test(bms) ||
            /\.appfolder-dialogs-light\b/.test(bms) ||
            /\.light-dash\b/.test(bms))
            throw new Error('Dark mode must leave explicit BMS Light styles free');
    } else {
        for (const required of [
            '#panel.light-panel, #panel.dark-panel',
            '.bms-popup-surface-dark .popup-menu-content.bms-popup-blurred',
            '.bms-popup-surface-dark .osd-window.bms-popup-blurred',
            '.bms-popup-surface-dark .modal-dialog.bms-popup-blurred',
            '.bms-popup-surface-dark .bms-popup-unblurred',
            '.bms-popup-surface-dark .message-list .message',
            '.bms-popup-surface-dark .quick-toggle-menu',
            '.overview-components-dark #overviewGroup .search-section-content',
            '.app-folder-dialog.appfolder-dialogs-dark',
            '.dark-dash .dash-background',
        ]) {
            if (!bms.includes(required))
                throw new Error(`Light mode missing BMS Auto harmony rule: ${required}`);
        }
        const lightLayers = layers(tokens, false);
        for (const key of ['chrome', 'sidebar', 'popover', 'panel', 'control']) {
            if (/^#(..)\1\1$/.test(lightLayers[key]))
                throw new Error(`Light mode ${key} lost hue tint (${lightLayers[key]})`);
        }
    }
}
print('PASS: 10 profiles × 2 modes × (360 hues + neutral), extreme wallpaper contrast, dialog entries, GTK3/GTK4/Adw tokens and BMS compatibility');
