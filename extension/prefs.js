import GIRepository from 'gi://GIRepository?version=3.0';
import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';
import {addPalettePage} from './palette-prefs.js';
import {addWallpaperAction} from './gallery-wallpaper.js';

export default class DuvarPreferences extends ExtensionPreferences {
    async fillPreferencesWindow(window) {
        const repository = GIRepository.Repository.dup_default();
        repository.prepend_search_path(`${this.path}/native`);
        repository.prepend_library_path(`${this.path}/native`);
        const {default: Duvar} = await import('gi://Duvar?version=1.0');

        // Shell keeps its metadata object across on-disk extension updates. Read the
        // installed package instead; checking cached Shell metadata rejects a valid update.
        const [loaded, contents] = this.dir.get_child('metadata.json').load_contents(null);
        const installed = loaded ? JSON.parse(new TextDecoder().decode(contents)) : null;
        const expected = installed?.['version-name']?.split('-')[0];
        const actual = Duvar.version();
        if (!expected)
            throw new Error('Kurulu Duvar paketinin sürüm bilgisi okunamadı.');
        if (actual !== expected) {
            throw new Error(`Duvar paketinin sürümü ${expected}, yüklenen Rust kitaplığının sürümü ${actual}. Açık Duvar ayar pencerelerini kapatıp yeniden aç; sorun sürerse paketi tekrar kur.`);
        }
        Duvar.populate(window, this.path);
        addPalettePage(window, this.getSettings(), this.path, addWallpaperAction);
    }
}
