#!/usr/bin/env -S gjs -m
import Gio from 'gi://Gio';
import GdkPixbuf from 'gi://GdkPixbuf';

// Precomputed sRGB [0..255] -> linear [0..1] lookup table (avoids pow() in pixel loop).
const SRGB_TO_LIN = new Float32Array(256);
for (let i = 0; i < 256; i++) {
    const v = i / 255;
    SRGB_TO_LIN[i] = v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

const BUCKETS = 72;
const HIST_BINS = 128;
const TOP_SLICES = 16;
const SLICE_HIST_BINS = 32;
const KERNEL = [0.15, 0.45, 0.80, 1.0, 0.80, 0.45, 0.15];
const MAX_XML_BYTES = 512 * 1024;
const round3 = v => Math.round(v * 1000) / 1000;

function rgbToOklab(lr, lg, lb) {
    const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
    const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
    const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
    return [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ];
}

function oklabToHslHue(L, a, b) {
    const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
    const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
    const s_ = L - 0.0894841775 * a - 1.2914855480 * b;
    const l = l_ * l_ * l_;
    const m = m_ * m_ * m_;
    const s = s_ * s_ * s_;
    const linToSrgb = v => {
        const c = Math.max(0, Math.min(1, v));
        return c <= 0.0031308 ? 12.92 * c : 1.055 * (c ** (1 / 2.4)) - 0.055;
    };
    const r = linToSrgb(+4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s);
    const g = linToSrgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s);
    const bl = linToSrgb(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s);
    const max = Math.max(r, g, bl);
    const min = Math.min(r, g, bl);
    const d = max - min;
    if (d < 1e-6) return null;
    const h = max === r ? (g - bl) / d : max === g ? 2 + (bl - r) / d : 4 + (r - g) / d;
    return ((h * 60) % 360 + 360) % 360;
}

function histQuantile(hist, total, q, offset = 0, bins = HIST_BINS) {
    if (!total) return 0;
    const target = total * q;
    let acc = 0;
    for (let i = 0; i < bins; i++) {
        acc += hist[offset + i];
        if (acc >= target)
            return (i + 0.5) / bins;
    }
    return 1;
}

function parseColorSpec(uri) {
    const raw = uri.slice(6).trim();
    let r, g, b;
    const hex = raw.match(/^#([0-9a-f]{6}|[0-9a-f]{3})$/i);
    if (hex) {
        const v = hex[1].length === 3
            ? hex[1].split('').map(ch => ch + ch).join('')
            : hex[1];
        r = parseInt(v.slice(0, 2), 16);
        g = parseInt(v.slice(2, 4), 16);
        b = parseInt(v.slice(4, 6), 16);
    } else {
        const rgbMatch = raw.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
        if (!rgbMatch) return null;
        [r, g, b] = rgbMatch.slice(1, 4).map(n => Math.max(0, Math.min(255, Number(n))));
    }
    const lr = SRGB_TO_LIN[r], lg = SRGB_TO_LIN[g], lb = SRGB_TO_LIN[b];
    const y = round3(0.2126 * lr + 0.7152 * lg + 0.0722 * lb);
    const [L, a, bl] = rgbToOklab(lr, lg, lb);
    const C = Math.hypot(a, bl);
    const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min;
    const neutral = d < 6 || C < 0.011;
    const hue = neutral ? null : oklabToHslHue(L, a, bl);
    return {
        hue: hue === null ? null : Math.round(hue * 100) / 100,
        chroma: neutral ? 0 : round3(Math.min(1, C / 0.22)),
        scene: {luma: y, p15: y, p85: y},
        top: {
            luma: y, p15: y, p85: y,
            zones: [y, y, y],
            bins: Array.from({length: TOP_SLICES}, () => [y, y, y]),
            spread: 0,
        },
    };
}

function resolveXmlWallpaper(xmlPath) {
    const file = Gio.File.new_for_path(xmlPath);
    const info = file.query_info('standard::size', Gio.FileQueryInfoFlags.NONE, null);
    if (info.get_size() > MAX_XML_BYTES)
        throw new Error('Dinamik duvar kâğıdı XML dosyası çok büyük.');
    const [, bytes] = file.load_contents(null);
    const xml = new TextDecoder('utf-8').decode(bytes);
    const parent = file.get_parent();
    const candidates = [];
    for (const pattern of [/<static>[\s\S]*?<file>([\s\S]*?)<\/file>/gi, /<(?:file|from)>([^<]+)<\/(?:file|from)>/gi]) {
        let match;
        while ((match = pattern.exec(xml)) !== null) {
            const inner = match[1];
            const sized = [...inner.matchAll(/<size\b[^>]*width="(\d+)"[^>]*height="(\d+)"[^>]*>([^<]+)<\/size>/gi)];
            if (sized.length) {
                sized.sort((a, b) => Number(b[1]) * Number(b[2]) - Number(a[1]) * Number(a[2]));
                for (const s of sized) candidates.push(s[3].trim());
            } else if (!inner.includes('<')) {
                candidates.push(inner.trim());
            }
        }
    }
    for (const raw of candidates) {
        if (!raw) continue;
        const resolved = raw.startsWith('/') ? Gio.File.new_for_path(raw) : parent?.resolve_relative_path(raw);
        const path = resolved?.get_path();
        if (path && resolved.query_exists(null) && !path.toLowerCase().endsWith('.xml'))
            return path;
    }
    throw new Error('Dinamik duvar kâğıdı içindeki görsel bulunamadı.');
}

function analyzeImage(path) {
    const [, width, height] = GdkPixbuf.Pixbuf.get_file_info(path);
    if (!width || !height || width * height > 48_000_000)
        throw new Error('Görsel okunamadı veya 48 megapiksel sınırını aşıyor.');
    const p = GdkPixbuf.Pixbuf.new_from_file_at_scale(path, 128, 128, true);
    const w = p.width;
    const h = p.height;
    const stride = p.rowstride;
    const channels = p.n_channels;
    const hasAlpha = p.has_alpha;
    const data = p.get_pixels();

    // Estimate the visible top-panel strip accounting for GNOME's default zoom
    // center-crop on portrait wallpapers.
    let topStart = 0;
    let visibleHeight = h;
    if (h > w * 1.15 && h >= 8) {
        visibleHeight = Math.max(4, Math.round(w * 0.625));
        topStart = Math.max(0, Math.floor((h - visibleHeight) / 2));
    }
    const topEnd = Math.min(h, topStart + Math.max(1, Math.round(visibleHeight * 0.075)));
    const z1 = Math.max(1, Math.floor(w * 0.28));
    const z2 = Math.max(z1, Math.floor(w * 0.72));

    const buckets = Array.from({length: BUCKETS}, () => ({weight: 0, count: 0, L: 0, a: 0, b: 0}));
    const sceneHist = new Uint32Array(HIST_BINS);
    const topHist = new Uint32Array(HIST_BINS);
    const sliceHist = new Uint16Array(TOP_SLICES * SLICE_HIST_BINS);
    const sliceSum = new Float64Array(TOP_SLICES);
    const sliceCount = new Uint32Array(TOP_SLICES);
    const zoneSum = [0, 0, 0];
    const zoneCount = [0, 0, 0];

    let samples = 0;
    let sceneLumaSum = 0;
    let topSamples = 0;
    let topLumaSum = 0;
    let chromatic = 0;
    let chromaSum = 0;

    for (let y = 0; y < h; y++) {
        const rowOff = y * stride;
        const inTop = y >= topStart && y < topEnd;
        const dy = h > 1 ? Math.abs(y / (h - 1) - 0.5) * 2 : 0;
        for (let x = 0; x < w; x++) {
            const idx = rowOff + x * channels;
            if (hasAlpha && data[idx + 3] < 128)
                continue;
            const r = data[idx];
            const g = data[idx + 1];
            const b = data[idx + 2];
            const lr = SRGB_TO_LIN[r];
            const lg = SRGB_TO_LIN[g];
            const lb = SRGB_TO_LIN[b];
            const lum = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;

            samples++;
            sceneLumaSum += lum;
            const bin = Math.min(HIST_BINS - 1, (lum * HIST_BINS) | 0);
            sceneHist[bin]++;

            if (inTop) {
                topSamples++;
                topLumaSum += lum;
                topHist[bin]++;
                const zone = x < z1 ? 0 : x < z2 ? 1 : 2;
                zoneSum[zone] += lum;
                zoneCount[zone]++;
                const slice = Math.min(TOP_SLICES - 1, Math.floor((x * TOP_SLICES) / w));
                const sBin = Math.min(SLICE_HIST_BINS - 1, (lum * SLICE_HIST_BINS) | 0);
                sliceHist[slice * SLICE_HIST_BINS + sBin]++;
                sliceSum[slice] += lum;
                sliceCount[slice]++;
            }

            const max = r > g ? (r > b ? r : b) : (g > b ? g : b);
            const min = r < g ? (r < b ? r : b) : (g < b ? g : b);
            const d = max - min;
            if (d < 5)
                continue;

            const [L, a, ob] = rgbToOklab(lr, lg, lb);
            const C = Math.hypot(a, ob);
            chromaSum += C;
            const minC = L < 0.26 ? 0.010 : 0.014;
            const minD = L < 0.26 ? 5 : 7;
            if (C < minC || d < minD)
                continue;

            chromatic++;
            const dx = w > 1 ? Math.abs(x / (w - 1) - 0.5) * 2 : 0;
            const centerBias = 1.14 - 0.26 * Math.min(1, Math.hypot(dx, dy) * 0.75);
            const relC = C / Math.max(0.06, Math.min(L, 1 - L) * 0.52 + 0.035);
            const shadowTaper = L < 0.06 ? 0.45 + 0.55 * (L / 0.06) : 1;
            const highlightTaper = L > 0.94 ? Math.max(0.25, (1 - L) / 0.06) : 1;
            const weight = (C ** 1.55) * (Math.min(relC, 2.5) ** 0.85) * centerBias * shadowTaper * highlightTaper;

            const angle = Math.atan2(ob, a);
            const norm = ((angle / (2 * Math.PI)) + 1) % 1;
            const bucket = buckets[Math.floor(norm * BUCKETS) % BUCKETS];
            bucket.weight += weight;
            bucket.count++;
            bucket.L += L * weight;
            bucket.a += a * weight;
            bucket.b += ob * weight;
        }
    }

    const sceneMean = samples ? sceneLumaSum / samples : 0.5;
    const sceneP15 = histQuantile(sceneHist, samples, 0.15);
    const sceneP85 = histQuantile(sceneHist, samples, 0.85);
    const topMean = topSamples ? topLumaSum / topSamples : sceneMean;
    const topP15 = topSamples ? histQuantile(topHist, topSamples, 0.15) : sceneP15;
    const topP85 = topSamples ? histQuantile(topHist, topSamples, 0.85) : sceneP85;
    const zones = [0, 1, 2].map(i => round3(zoneCount[i] ? zoneSum[i] / zoneCount[i] : topMean));
    const bins = Array.from({length: TOP_SLICES}, (_, s) => {
        const c = sliceCount[s];
        if (!c) return [round3(topMean), round3(topP15), round3(topP85)];
        const off = s * SLICE_HIST_BINS;
        return [
            round3(sliceSum[s] / c),
            round3(histQuantile(sliceHist, c, 0.15, off, SLICE_HIST_BINS)),
            round3(histQuantile(sliceHist, c, 0.85, off, SLICE_HIST_BINS)),
        ];
    });
    const spread = round3(Math.max(0, topP85 - topP15, Math.max(...zones) - Math.min(...zones)));

    let hue = null;
    let dominantChroma = 0;

    if (samples > 0 && chromatic > 0 && !(chromatic < samples * 0.035 && chromaSum < 0.011 * samples)) {
        const half = (KERNEL.length - 1) >> 1;
        let bestIdx = -1;
        let bestScore = 0;

        for (let i = 0; i < BUCKETS; i++) {
            let wSum = 0;
            let cSum = 0;
            for (let j = 0; j < KERNEL.length; j++) {
                const bkt = buckets[(i + j - half + BUCKETS) % BUCKETS];
                const k = KERNEL[j];
                wSum += bkt.weight * k;
                cSum += bkt.count * k;
            }
            if (wSum <= 0) continue;
            // Suppress tiny 1-2% saturated outlier specks unless they are the only color.
            const share = cSum / samples;
            const coherence = share < 0.03
                ? Math.max(0.18, (share / 0.03) ** 1.45) * 0.72
                : Math.min(1, Math.sqrt(share / 0.055));
            const score = wSum * coherence;
            if (score > bestScore) {
                bestScore = score;
                bestIdx = i;
            }
        }

        if (bestIdx >= 0 && bestScore > 0) {
            let totalW = 0, sumL = 0, sumA = 0, sumB = 0;
            for (let j = 0; j < KERNEL.length; j++) {
                const bkt = buckets[(bestIdx + j - half + BUCKETS) % BUCKETS];
                const k = KERNEL[j];
                totalW += bkt.weight * k;
                sumL += bkt.L * k;
                sumA += bkt.a * k;
                sumB += bkt.b * k;
            }
            if (totalW > 0) {
                const cL = sumL / totalW;
                const cA = sumA / totalW;
                const cB = sumB / totalW;
                const converted = oklabToHslHue(cL, cA, cB);
                if (converted !== null) {
                    hue = Math.round(converted * 100) / 100;
                    dominantChroma = round3(Math.min(1, Math.hypot(cA, cB) / 0.22));
                }
            }
        }
    }

    return {
        hue,
        chroma: dominantChroma,
        scene: {luma: round3(sceneMean), p15: round3(sceneP15), p85: round3(sceneP85)},
        top: {luma: round3(topMean), p15: round3(topP15), p85: round3(topP85), zones, bins, spread},
    };
}

// Decoding is isolated from both Shell and preferences.
const unique = [...new Set(ARGV)];
const result = unique.map(uri => {
    if (uri.startsWith('color:')) {
        const solid = parseColorSpec(uri);
        if (!solid) throw new Error('Geçersiz düz renk biçimi.');
        return solid;
    }
    const file = Gio.File.new_for_uri(uri);
    let path = file.get_path();
    if (!path)
        throw new Error('Renk analizi için yerel bir duvar kâğıdı gerekiyor.');
    const info = file.query_info('standard::type,standard::size', Gio.FileQueryInfoFlags.NONE, null);
    if (info.get_file_type() !== Gio.FileType.REGULAR || info.get_size() > 64 * 1024 * 1024)
        throw new Error('Görsel normal bir dosya olmalı ve 64 MiB sınırını aşmamalı.');
    if (path.toLowerCase().endsWith('.xml'))
        path = resolveXmlWallpaper(path);
    return analyzeImage(path);
});

const byUri = new Map(unique.map((uri, i) => [uri, result[i]]));
print(JSON.stringify(ARGV.map(uri => byUri.get(uri))));
