# Duvar Localization Context Map (`locales/CONTEXT.md`)

Complete key-by-key reference showing **where each text appears**, **which file & widget uses it**, and **required `{placeholders}`**.
Both human contributors and AI assistants can rely on this table to translate accurately without guessing UI context.

- **Total keys:** 223
- **Supported locales:** 10 (`tr` default + `en`, `es`, `de`, `fr`, `pt`, `ru`, `ja`, `zh`, `ar`)

## `tabs`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `tabs.explore` | `src/ui.rs` | PreferencesPage title (Tab 1) | — | Keşfet | Explore |
| `tabs.downloads` | `src/ui.rs` | PreferencesPage title (Tab 2) | — | İndirilenler | Downloads |
| `tabs.filters` | `src/ui.rs` | PreferencesPage title (Tab 3) | — | Filtreler | Filters |
| `tabs.settings` | `src/ui.rs` | PreferencesPage title (Tab 4) | — | Ayarlar | Settings |
| `tabs.colors` | `extension/palette-prefs.js` | PreferencesPage title (Tab 5) | — | Renkler | Colors |
## `common`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `common.apply` | `src/ui/*` | Primary confirm button | — | Uygula | Apply |
| `common.cancel` | `src/ui/*` | Dialog cancel button | — | Vazgeç | Cancel |
| `common.clear` | `src/ui/*` | Clear selection/list button | — | Temizle | Clear |
| `common.reset` | `src/ui/preferences.rs` | Reset filters button | — | Sıfırla | Reset |
| `common.retry` | `src/ui/*` | Retry failed action button | — | Tekrar dene | Retry |
| `common.undo` | `src/ui/*` | Undo wallpaper/trash action button | — | Geri al | Undo |
| `common.download` | `src/ui/transfer_ui.rs` | Download wallpaper button | — | İndir | Download |
| `common.downloading` | `src/ui/*` | Active transfer status | — | İndiriliyor… | Downloading… |
| `common.downloaded` | `src/ui/transfer_ui.rs` | Completed item tag | — | İndirildi | Downloaded |
| `common.active` | `extension/palette-prefs.js` | Active mode pill badge | — | Etkin | Active |
| `common.off` | `extension/palette-prefs.js` | Disabled status label | — | Kapalı | Off |
| `common.all` | `src/ui/preferences.rs` | Unfiltered dropdown option | — | Tümü | All |
| `common.filter` | `src/ui/preferences.rs` | Filter menu button label | — | Süz | Filter |
| `common.refresh` | `src/ui.rs` | Reload / refresh button | — | Yeniden yükle | Reload |
| `common.apply_light` | `src/ui/transfer_ui.rs` | Lightbox apply to light theme | — | Açık tema için uygula | Set for Light Theme |
| `common.apply_dark` | `src/ui/transfer_ui.rs` | Lightbox apply to dark theme | — | Koyu tema için uygula | Set for Dark Theme |
| `common.apply_light_short` | `src/ui/transfer_ui.rs` | Compact card apply light button | — | Açığa uygula | Set Light |
| `common.apply_dark_short` | `src/ui/transfer_ui.rs` | Compact card apply dark button | — | Koyuya uygula | Set Dark |
| `common.manage_sources` | `src/ui/*` | Open Sources dialog button | — | Kaynakları yönet | Manage Sources |
| `common.categories` | `src/ui/*` | Open Topics dialog button | — | Kategoriler | Categories |
| `common.open_viewer` | `src/ui/transfer_ui.rs` | Open in default image viewer | — | Görüntüleyicide aç | Open in Viewer |
| `common.open_folder` | `src/ui/transfer_ui.rs` | Open containing folder | — | Klasörü aç | Open Folder |
| `common.open_source` | `src/ui/transfer_ui.rs` | Open upstream source link in browser | — | Kaynağı tarayıcıda aç | Open Source Page |
| `common.trash` | `src/ui/transfer_ui.rs` | Move managed wallpaper to Trash | — | Çöp kutusuna taşı | Move to Trash |
| `common.favorite_toggle` | `src/ui/transfer_ui.rs` | Star button tooltip | — | Favoriye ekle / çıkar | Toggle Favorite |
| `common.source` | `src/ui/preferences.rs` | Source label | — | Kaynak | Source |
| `common.queued` | `src/ui/*` | Queued status label | — | Sırada | Queued |
| `common.paused` | `src/ui/*` | Paused status label | — | Duraklatıldı | Paused |
| `common.done` | `src/ui.rs` | Completed status label | — | Tamamlandı | Completed |
| `common.failed` | `src/ui/preferences.rs` | Failed status label | — | Başarısız | Failed |
| `common.canceled` | `src/ui.rs` | Canceled status label | — | İptal edildi | Canceled |
| `common.ready` | `src/ui/preferences.rs` | Ready status label | — | Hazır | Ready |
## `explore`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `explore.search_placeholder` | `src/ui.rs` | Main gallery SearchEntry placeholder | — | Duvar kâğıdı ara… | Search wallpapers… |
| `explore.sort_recommended` | `src/ui.rs` | Sort dropdown item 0 | — | Önerilen | Recommended |
| `explore.sort_latest` | `src/ui.rs` | Sort dropdown item 1 | — | En yeni | Newest |
| `explore.sort_resolution` | `src/ui.rs` | Sort dropdown item 2 | — | Çözünürlük | Resolution |
| `explore.sort_landscape` | `src/ui.rs` | Sort dropdown item 3 | — | Yatay oran | Widescreen |
| `explore.sort_random` | `src/ui.rs` | Sort dropdown item 4 | — | Rastgele | Random |
| `explore.sort_tooltip` | `src/ui.rs` | Sort dropdown tooltip | — | Sıralama ölçütü | Sort order |
| `explore.favorites_tooltip` | `src/ui.rs` | Favorites toggle tooltip | — | Favorileri göster (en fazla 128) | Show favorites (up to 128) |
| `explore.load_more` | `src/ui.rs` | Accessible load-more button | — | Daha fazla sonuç yükle | Load more results |
| `explore.no_results` | `src/ui.rs` | Empty state title | — | Eşleşen görsel yok | No matching wallpapers |
| `explore.no_favorites` | `src/ui.rs` | Empty favorites title | — | Eşleşen favori yok | No matching favorites |
| `explore.no_sources` | `src/ui.rs` | No sources enabled banner | — | Etkin kaynak yok | No active sources |
| `explore.all_loaded` | `src/ui/preferences.rs` | Footer completion note | — | Tüm sonuçlar yüklendi | All results loaded |
| `explore.status_line` | `src/ui.rs` | Footer summary ({count}, {state}, {sources}) | `{count}`, `{sources}`, `{state}` | {count} görsel · {state} · {sources} kaynak | {count} wallpapers · {state} · {sources} sources |
| `explore.tools_history` | `src/ui/preferences.rs` | Explore menu tooltip | — | Keşfet araçları ve geçmiş | Explore tools and history |
| `explore.recent_searches` | `src/ui/preferences.rs` | Recent searches menu header | — | Son aramalar | Recent Searches |
| `explore.refresh_keep` | `src/ui/preferences.rs` | Menu item refresh keeping results | — | Sonuçları koruyarak yenile | Refresh keeping results |
## `lightbox`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `lightbox.zoom_100` | `src/ui/preferences.rs` | 1:1 pixel zoom toggle tooltip | — | %100 gerçek piksel (1) | 100% native pixels (1) |
| `lightbox.zoom_in` | `src/ui/preferences.rs` | Zoom in button tooltip | — | Yakınlaştır | Zoom In |
| `lightbox.zoom_out` | `src/ui/preferences.rs` | Zoom out button tooltip | — | Uzaklaştır | Zoom Out |
| `lightbox.fit_window` | `src/ui/preferences.rs` | Fit to window button tooltip | — | Ekrana sığdır (0) | Fit to Window (0) |
| `lightbox.prev_image` | `src/ui.rs` | Previous arrow tooltip | — | Önceki görsel (←) | Previous image (←) |
| `lightbox.next_image` | `src/ui.rs` | Next arrow tooltip | — | Sonraki görsel (→) | Next image (→) |
| `lightbox.info_drawer` | `src/ui.rs` | Info sidebar toggle tooltip | — | Görsel bilgileri ve etiketler (I) | Image info and tags (I) |
| `lightbox.info_title` | `src/ui.rs` | Info sidebar heading | — | Görsel bilgileri | Image Details |
| `lightbox.loading_original` | `src/ui.rs` | Lightbox footer while original loads | — | Özgün dosya yükleniyor… | Loading full resolution… |
| `lightbox.status_fit` | `src/ui.rs` | Lightbox footer when fitted ({width}, {height}) | `{height}`, `{width}` | Özgün dosya · {width} × {height} piksel · Ekrana sığdırılıyor | Original · {width} × {height} px · Fit to window |
| `lightbox.status_100` | `src/ui.rs` | Lightbox footer at 100% ({width}, {height}) | `{height}`, `{width}` | Özgün dosya · {width} × {height} piksel · %100 | Original · {width} × {height} px · 100% |
| `lightbox.controls_hint` | `src/ui.rs` | Lightbox footer interaction hint | — | Tekerlek: yakınlaştır · Sürükle: kaydır · Sağ tık: menü | Wheel: zoom · Drag: pan · Right-click: menu |
| `lightbox.toast_light_set` | `src/ui.rs` | Toast after setting light wallpaper | — | Açık temanın duvar kâğıdı ayarlandı. | Light theme wallpaper updated. |
| `lightbox.toast_dark_set` | `src/ui.rs` | Toast after setting dark wallpaper | — | Koyu temanın duvar kâğıdı ayarlandı. | Dark theme wallpaper updated. |
| `lightbox.toast_both_set` | `src/ui.rs` | Toast after setting both wallpapers | — | İki temanın duvar kâğıdı ayarlandı. | Light and dark wallpapers updated. |
| `lightbox.toast_fav_added` | `src/ui.rs` | Toast when favorited | — | Favorilere eklendi. | Added to favorites. |
| `lightbox.toast_fav_removed` | `src/ui.rs` | Toast when unfavorited | — | Favorilerden çıkarıldı. | Removed from favorites. |
| `lightbox.toast_trashed` | `src/ui.rs` | Toast after moving file to Trash | — | Görsel çöp kutusuna taşındı. | Moved image to Trash. |
| `lightbox.toast_restored` | `src/ui.rs` | Toast after undo trash | — | Görsel geri yüklendi. | Image restored. |
## `downloads`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `downloads.title` | `src/ui.rs` | Downloads group title | — | İndirilen görseller | Downloaded Wallpapers |
| `downloads.subtitle` | `src/ui.rs` | Downloads group subtitle | — | Cihazındaki yerel duvar kâğıtları. Etkin içerik izinleri burada da geçerlidir. | Local wallpapers on this device. Active content permissions apply here too. |
| `downloads.empty` | `src/ui.rs` | Empty downloads state | — | Henüz indirilmiş görsel yok. Keşfet’ten bir görsel indir. | No downloaded wallpapers yet. Download one from Explore. |
| `downloads.queue_title` | `src/ui.rs` | Download queue group title | — | İndirme kuyruğu | Download Queue |
| `downloads.queue_search` | `src/ui/preferences.rs` | Queue search placeholder | — | Kuyrukta ara… | Search queue… |
| `downloads.clear_completed` | `src/ui/preferences.rs` | Queue button clear done | — | Tamamlananları temizle | Clear Completed |
| `downloads.retry_failed` | `src/ui/preferences.rs` | Queue button retry failed | — | Başarısızları tekrar dene | Retry Failed |
| `downloads.cancel_all` | `src/ui/preferences.rs` | Queue button cancel all | — | Tümünü iptal et | Cancel All |
| `downloads.refresh_downloads` | `src/ui.rs` | Refresh downloads tooltip | — | İndirilenleri yenile | Refresh Downloads |
## `filters`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `filters.permissions_title` | `src/ui/preferences.rs` | Content permissions group title | — | İçerik izinleri | Content Permissions |
| `filters.permissions_desc` | `src/ui/preferences.rs` | Content permissions group desc | — | Kaynağın güvenlik etiketine göre uygulanır. Kategori seçimi bu izinleri değiştirmez. | Applied using each source’s rating metadata. Selecting categories never enables sensitive ratings. |
| `filters.sfw_title` | `src/ui/preferences.rs` | SFW SwitchRow title | — | SFW | SFW |
| `filters.sfw_desc` | `src/ui/preferences.rs` | SFW SwitchRow subtitle | — | Kaynağın güvenli olarak etiketlediği görseller. | Images marked safe by the source. |
| `filters.questionable_title` | `src/ui/preferences.rs` | Questionable SwitchRow title | — | Şüpheli | Questionable |
| `filters.questionable_desc` | `src/ui/preferences.rs` | Questionable SwitchRow subtitle | — | Hassas veya sınırda etiketlenmiş görseller. | Images tagged suggestive or borderline. |
| `filters.explicit_title` | `src/ui/preferences.rs` | Explicit SwitchRow title | — | Yetişkin | Explicit |
| `filters.explicit_desc` | `src/ui/preferences.rs` | Explicit SwitchRow subtitle | — | Başlangıçta kapalıdır. Wallhaven bu sınıf için API anahtarı ister. | Off by default. Wallhaven requires an API key for this rating. |
| `filters.unknown_title` | `src/ui/preferences.rs` | Unrated SwitchRow title | — | Etiketsiz içerik | Unrated Content |
| `filters.unknown_desc` | `src/ui/preferences.rs` | Unrated SwitchRow subtitle | — | Müze arşivleri güvenlik derecesi sunmadığı için Etiketsiz içerik izni gerektirir. | Museum archives and local folders do not include rating tags and require this permission. |
| `filters.remember_sensitive_title` | `src/ui/preferences.rs` | Persist sensitive permissions toggle | — | Hassas içerik izinlerini hatırla | Remember Sensitive Permissions |
| `filters.remember_sensitive_desc` | `src/ui/preferences.rs` | Persist sensitive permissions subtitle | — | Kapalıyken her yeni pencere yalnızca SFW ile başlar. | When off, every new window starts in SFW-only mode. |
| `filters.specs_title` | `src/ui/preferences.rs` | Resolution/format group title | — | Çözünürlük ve biçim | Resolution and Format |
| `filters.specs_desc` | `src/ui/preferences.rs` | Resolution/format group desc | — | Minimum piksel boyutu, yön ve dosya biçimi tercihleri. | Minimum pixel size, orientation, and file format rules. |
| `filters.min_resolution` | `src/ui/preferences.rs` | Minimum resolution row | — | Minimum çözünürlük | Minimum Resolution |
| `filters.no_limit` | `src/ui/preferences.rs` | No resolution limit option | — | Sınır yok | No limit |
| `filters.orientation` | `src/ui/preferences.rs` | Orientation row | — | Görsel yönü | Orientation |
| `filters.all_orientations` | `src/ui/preferences.rs` | All orientations option | — | Tüm yönler | All orientations |
| `filters.formats` | `src/ui/preferences.rs` | Formats row | — | Dosya biçimleri | File Formats |
| `filters.tag_rules` | `src/ui/preferences.rs` | Tag rules group title | — | Etiket kuralları | Tag Rules |
| `filters.reset_safe` | `src/ui/preferences.rs` | Reset filters action | — | Filtreleri güvenli varsayılana döndür | Reset Filters to Safe Defaults |
| `filters.source_compat` | `src/ui/preferences.rs` | Source compatibility expander | — | Etkin kaynak uyumu | Active Source Compatibility |
## `topics`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `topics.general_tab` | `src/ui/topics.rs` | Topics tab 1 | — | Genel · 100 | General · 100 |
| `topics.sensitive_tab` | `src/ui/topics.rs` | Topics tab 2 | — | NSFW · 100 | NSFW · 100 |
| `topics.bulk_select` | `src/ui/topics.rs` | Bulk actions menu button | — | Toplu seçim | Bulk Select |
| `topics.select_group` | `src/ui/topics.rs` | Bulk action select group | — | Gruptakileri seç | Select Group |
| `topics.clear_group` | `src/ui/topics.rs` | Bulk action clear group | — | Gruptakileri kaldır | Clear Group |
| `topics.invert_group` | `src/ui/topics.rs` | Bulk action invert | — | Seçimi tersine çevir | Invert Selection |
| `topics.only_selected` | `src/ui/topics.rs` | Filter toggle only selected | — | Yalnızca seçili | Selected only |
| `topics.all_groups` | `src/ui/topics.rs` | All topic groups dropdown item | — | Tüm gruplar | All groups |
## `settings`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `settings.group_collections` | `src/ui/preferences.rs` | Settings group 1 | — | Koleksiyonlar | Collections |
| `settings.sources_row` | `src/ui/preferences.rs` | Sources management row | — | Görsel kaynakları | Wallpaper Sources |
| `settings.group_files` | `src/ui/preferences.rs` | Settings group 2 | — | Dosyalar ve duvar kâğıdı | Files and Wallpaper |
| `settings.download_dir` | `src/ui/preferences.rs` | Download folder row | — | İndirme klasörü | Download Folder |
| `settings.local_dir` | `src/ui/preferences.rs` | Local images folder row | — | Resim klasörü | Local Pictures Folder |
| `settings.not_set` | `src/ui/preferences.rs` | Unset folder subtitle | — | Henüz seçilmedi | Not selected yet |
| `settings.pick_folder` | `src/ui/preferences.rs` | Folder chooser tooltip | — | Klasör seç | Choose folder |
| `settings.undo_last` | `src/ui/preferences.rs` | Undo last wallpaper row | — | Son uygulamayı geri al | Undo Last Wallpaper |
| `settings.undo_last_desc` | `src/ui/preferences.rs` | Undo last wallpaper subtitle | — | Önceki açık ve koyu duvar kâğıdı çiftini geri yükler. | Restores the previous light and dark wallpaper pair. |
| `settings.group_appearance` | `src/ui/preferences.rs` | Settings group 3 | — | Görünüm ve gizlilik | Appearance and Privacy |
| `settings.language_title` | `src/ui/preferences.rs` | Language ComboRow title | — | Uygulama dili | Language |
| `settings.language_desc` | `src/ui/preferences.rs` | Language ComboRow subtitle | — | Varsayılan Türkçe’dir; 10 dil desteklenir. | Defaults to Turkish; 10 languages are built in. |
| `settings.lang_default` | `src/i18n.rs` | Default tag next to Turkish | — | Varsayılan | Default |
| `settings.language_changed_toast` | `src/ui/preferences.rs` | Toast after switching language | — | Dil güncellendi. Tüm sekmeler için pencereyi kapatıp aç. | Language updated. Reopen preferences to refresh all tabs. |
| `settings.checkerboard_title` | `src/ui/preferences.rs` | Transparency grid toggle | — | Saydam görsellerde dama zemini | Checkerboard for Transparent Images |
| `settings.history_title` | `src/ui/preferences.rs` | Remember search history toggle | — | Arama geçmişini hatırla | Remember Search History |
| `settings.history_desc` | `src/ui/preferences.rs` | Remember search history subtitle | — | Başlangıçta kapalıdır. Kapattığında kayıtlı geçmiş temizlenir. | Off by default. Turning it off clears saved queries. |
| `settings.group_cache` | `src/ui/preferences.rs` | Settings group 4 | — | Önbellek ve bakım | Cache and Maintenance |
| `settings.cache_desc` | `src/ui/preferences.rs` | Cache group description | — | 256 MiB sınırında küçük resim ve özgün önbelleği. Uygulama kapalıyken arka plan işlemi çalışmaz. | Shared 256 MiB thumbnail and preview cache. No background service runs when closed. |
| `settings.cache_manage` | `src/ui/preferences.rs` | Cache management expander | — | Önbellek yönetimi | Cache Management |
| `settings.clear_thumbs` | `src/ui/preferences.rs` | Clear thumbnail cache button | — | Küçük resim önbelleğini temizle | Clear Thumbnail Cache |
| `settings.clear_sensitive` | `src/ui/preferences.rs` | Clear sensitive cache button | — | Hassas önbelleği temizle | Clear Sensitive Cache |
| `settings.export_diag` | `src/ui/preferences.rs` | Diagnostics export row | — | Tanı olaylarını JSON olarak kaydet | Export Diagnostics as JSON |
| `settings.export_diag_desc` | `src/ui/preferences.rs` | Diagnostics export subtitle | — | Son 256 olay; görseller ve API anahtarları içermez. | Last 256 events; contains no images or API keys. |
| `settings.shortcuts_title` | `src/ui/preferences.rs` | Shortcuts row title | — | Klavye ve fare kısayolları | Keyboard and Mouse Shortcuts |
| `settings.shortcuts_desc` | `src/ui/preferences.rs` | Shortcuts row subtitle | — | Ctrl+F: ara · Ctrl+1–4: sekmeler · Önizleme: tekerlek yakınlaştır · sol sürükle kaydır · sağ tık menü · ←/→ gez · I: bilgiler · 0/1: sığdır/%100 | Ctrl+F: search · Ctrl+1–4: tabs · Lightbox: wheel zoom · drag pan · right-click menu · ←/→ browse · I: info · 0/1: fit/100% |
## `about`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `about.row_title` | `src/ui/preferences.rs` | About ActionRow title | — | Duvar hakkında | About Duvar |
| `about.row_subtitle` | `src/ui/preferences.rs` | About ActionRow subtitle ({version}) | `{version}` | Sürüm {version} · Sürüm notları ve lisans | Version {version} · Release notes and license |
| `about.comments` | `src/ui/preferences.rs` | AdwAboutDialog comments | — | GNOME için çok kaynaklı duvar kâğıdı galerisi ve duvar kâğıdına uyarlanan dinamik renk motoru. | Multi-source wallpaper gallery and adaptive OKLab color engine for GNOME. |
| `about.credits_title` | `src/ui/preferences.rs` | About credit section header | — | Esinlenme ve ekosistem | Inspiration and Ecosystem |
| `about.ack_title` | `src/ui/preferences.rs` | About acknowledgement header | — | Üretim notu | Build Note |
| `about.ack_body` | `src/ui/preferences.rs` | Subtle vibe-coding note | — | Yapay zekâ destekli geliştirme (vibe coding) akışıyla tasarlandı ve doğrulandı | Designed and verified through an AI-assisted development (vibe coding) workflow |
| `about.whats_new_intro` | About dialog | `src/ui/preferences.rs` | About dialog release notes intro | — | `Duvar 0.6.0; baştan tasarlanan Adwaita arayüzü, Pro Lightbox, 10 yerleşik dil (varsayılan Türkçe), cam üst çubuk kapsülleri ve önerilen masaüstü kurulum betiği ile gelir.` | `Duvar 0.6.0 brings a redesigned Adwaita interface, Pro Lightbox, 10 built-in languages (Turkish default), floating glass top-panel capsules, and a recommended desktop setup script.` |
| `about.whats_new_1` | About dialog | `src/ui/preferences.rs` | About dialog release notes bullet | — | `Hem açık hem koyu temada canlı stil ve yoğunluk ayarlı, tema tonlu yüzen cam üst çubuk kapsülleri` | `Ambient-tinted floating glass top-panel capsules in both light and dark modes with live style and density controls` |
| `about.whats_new_2` | About dialog | `src/ui/preferences.rs` | About dialog release notes bullet | — | `OKLab baskın renk motoru, 16 dilimli üst çubuk kontrast analizi ve User Themes olmadan doğrudan Shell stil yükleme` | `OKLab dominant color engine, 16-slice top-bar contrast profiling, and direct Shell stylesheet loading without requiring User Themes` |
| `about.whats_new_3` | About dialog | `src/ui/preferences.rs` | About dialog release notes bullet | — | `8 LIFO küçük resim işçisi, 180 ögelik LRU bellek önbelleği ve kaydırma sıçramasız 6 şeritli Rust motoru` | `6-lane parallel Rust engine with 8 LIFO thumbnail workers, 180-item LRU cache, and zero-splice smooth scrolling` |
| `about.whats_new_4` | About dialog | `src/ui/preferences.rs` | About dialog release notes bullet | — | `İmleç odaklı tekerlek yakınlaştırma, sürükleyerek gezme, sağ tık menüsü ve açılır bilgi çekmeceli Pro Lightbox` | `Pro Lightbox with cursor-anchored wheel zoom, drag-to-pan, right-click menu, and slide-in metadata drawer` |
| `about.whats_new_5` | About dialog | `src/ui/preferences.rs` | About dialog release notes bullet | — | `Özgün Adwaita uygulama ikonu ve belgelenmiş locales kataloğunda varsayılan Türkçe dahil 10 dil desteği` | `Original Adwaita app icon and 10 built-in languages in a documented locales catalog` |
| `about.whats_new_6` | About dialog | `src/ui/preferences.rs` | About dialog release notes bullet | — | `Tek komutluk önerilen GNOME masaüstü kurulum betiği ile Polkit ve Shell diyalog görünüm düzeltmeleri` | `One-command recommended GNOME desktop preset installer plus Polkit and Shell dialog styling fixes` |
## `colors`

| Key | File | Widget / Context | Placeholders | Turkish (`tr`, default) | English (`en`) |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `colors.group_theme_title` | `extension/palette-prefs.js` | Primary Colors group title | — | Duvar kâğıdından tema | Wallpaper-Driven Theme |
| `colors.group_theme_desc` | `extension/palette-prefs.js` | Primary Colors group description | — | Açık ve koyu duvar kâğıtlarından GNOME Shell, üst panel ve uygulamalar için canlı renk üretir. | Generates adaptive colors for GNOME Shell, the top bar, and apps from your light and dark wallpapers. |
| `colors.enable_title` | `extension/palette-prefs.js` | Master enable SwitchRow title | — | Duvar temasını kullan | Use Duvar Theme |
| `colors.enable_desc` | `extension/palette-prefs.js` | Master enable SwitchRow subtitle | — | User Themes olmadan da çalışır; başka bir Shell tema yöneticisiyle birlikte kullanma. | Works even without User Themes; avoid running alongside another Shell theme manager. |
| `colors.status_title` | `extension/palette-prefs.js` | Theme status ActionRow title | — | Tema durumu | Theme Status |
| `colors.preview_title` | `extension/palette-prefs.js` | Live preview group title | — | Önizleme | Preview |
| `colors.last_saved` | `extension/palette-prefs.js` | Saved preview group title | — | Son kayıt | Saved Palette |
| `colors.light_preview` | `extension/palette-prefs.js` | Light preview card title | — | Açık görünüm | Light Appearance |
| `colors.dark_preview` | `extension/palette-prefs.js` | Dark preview card title | — | Koyu görünüm | Dark Appearance |
| `colors.card_label` | `extension/palette-prefs.js` | Preview inner card primary label | — | Kart | Card |
| `colors.card_secondary` | `extension/palette-prefs.js` | Preview inner card secondary label | — | İkincil | Secondary |
| `colors.chip_passive` | `extension/palette-prefs.js` | Preview chip 1 | — | Pasif | Idle |
| `colors.chip_selected` | `extension/palette-prefs.js` | Preview chip 2 | — | Seçim | Selected |
| `colors.chip_focus` | `extension/palette-prefs.js` | Preview chip 3 | — | Odak | Focus |
| `colors.panel_group_title` | `extension/palette-prefs.js` | Top panel capsules group title | — | Üst panel ve kapsül görünümü | Top Bar and Floating Capsules |
| `colors.panel_group_desc` | `extension/palette-prefs.js` | Top panel capsules group desc | — | Hem açık hem koyu temada üst panel öğelerinin okunabilirlik stilini anlık olarak seç. | Instantly adjust top-bar readability in both light and dark modes. |
| `colors.panel_style_title` | `extension/palette-prefs.js` | Top panel style ComboRow | — | Üst panel stili | Top Bar Style |
| `colors.capsule_strength_title` | `extension/palette-prefs.js` | Capsule density ComboRow | — | Kapsül cam yoğunluğu | Capsule Glass Density |
| `colors.style_capsule` | `extension/palette-prefs.js` | Panel style option: capsule | — | Yüzen kapsül | Floating Capsule |
| `colors.style_capsule_desc` | `extension/palette-prefs.js` | Panel style desc: capsule | — | Açık ve koyu temada her üst panel öğesini uyumlu cam kapsül içine alır. | Ambient-tinted glass pills in both light and dark modes. |
| `colors.style_smart` | `extension/palette-prefs.js` | Panel style option: smart | — | Akıllı kapsül | Smart Capsule |
| `colors.style_smart_desc` | `extension/palette-prefs.js` | Panel style desc: smart | — | Temiz bölgelerde uyarlanabilir mürekkep, karmaşık bölgelerde otomatik kapsül kullanır. | Uses adaptive ink over clean regions and automatic capsules over busy regions. |
| `colors.style_scrim` | `extension/palette-prefs.js` | Panel style option: scrim | — | Üst perde | Top Scrim |
| `colors.style_scrim_desc` | `extension/palette-prefs.js` | Panel style desc: scrim | — | Şeffaf üst panelde arka planı dengeleyen yumuşak geçiş perdesi uygular. | Applies a soft gradient scrim behind the transparent top bar to balance contrast. |
| `colors.style_plain` | `extension/palette-prefs.js` | Panel style option: plain | — | Sade mürekkep | Plain Ink |
| `colors.style_plain_desc` | `extension/palette-prefs.js` | Panel style desc: plain | — | Kapsül ve perde olmadan her öğenin rengini arka plana göre ayarlar. | Adjusts each item’s ink color directly against the wallpaper without capsules or scrim. |
| `colors.strength_subtle` | `extension/palette-prefs.js` | Capsule density: subtle | — | Hafif cam | Subtle Glass |
| `colors.strength_subtle_desc` | `extension/palette-prefs.js` | Capsule density desc: subtle | — | Daha geçirgen, duvar kâğıdını öne çıkaran ince cam kapsüller. | More translucent glass pills that keep the wallpaper front and center. |
| `colors.strength_balanced` | `extension/palette-prefs.js` | Capsule density: balanced | — | Dengeli cam | Balanced Glass |
| `colors.strength_balanced_desc` | `extension/palette-prefs.js` | Capsule density desc: balanced | — | Yüksek okunabilirlik ve zarif yarı saydamlık dengesi. | Balanced contrast and refined translucency for everyday wallpapers. |
| `colors.strength_crisp` | `extension/palette-prefs.js` | Capsule density: crisp | — | Belirgin kapsül | Crisp Capsule |
| `colors.strength_crisp_desc` | `extension/palette-prefs.js` | Capsule density desc: crisp | — | Çok hareketli duvar kâğıtlarında en net ayrım ve maksimum kontrast. | Strongest separation and maximum contrast over very detailed wallpapers. |
| `colors.character_group_title` | `extension/palette-prefs.js` | Palette character group title | — | Renk karakteri | Palette Character |
| `colors.palette_row` | `extension/palette-prefs.js` | Palette profile ComboRow title | — | Palet | Palette |
| `colors.save_palette` | `extension/palette-prefs.js` | Save selected palette button | — | Seçili paleti kaydet | Save Selected Palette |
| `colors.prepare_palettes` | `extension/palette-prefs.js` | Analyze wallpapers button | — | 10 paleti hazırla | Generate 10 Palettes |
| `colors.analyzing` | `extension/palette-prefs.js` | Worker running status | — | Açık ve koyu görseller inceleniyor… | Analyzing light and dark wallpapers… |
| `colors.compare_prompt` | `extension/palette-prefs.js` | Prompt when 10 palettes are ready | — | Üstteki açık ve koyu önizlemeleri karşılaştırıp istediğin karakteri kaydet. | Compare the light and dark previews above, then save your preferred character. |
| `colors.prepare_prompt` | `extension/palette-prefs.js` | Prompt before analysis | — | Bu duvar kâğıtlarına ait renkleri görmek için paletleri hazırla. | Generate palettes to preview colors from your current wallpapers. |
| `colors.wallpapers_group` | `extension/palette-prefs.js` | Active wallpapers group title | — | Duvar kâğıtları | Active Wallpapers |
| `colors.wallpapers_desc_gallery` | `extension/palette-prefs.js` | Wallpapers desc when gallery present | — | Her düğme yalnızca adını taşıdığı temanın görselini değiştirir. | Each button updates only the theme mode shown on its label. |
| `colors.wallpapers_desc_store` | `extension/palette-prefs.js` | Wallpapers desc in store-only mode | — | Duvar kâğıtlarını GNOME Ayarları üzerinden değiştirdiğinde palet otomatik yenilenir. | Change your wallpapers in GNOME Settings and Duvar updates palettes automatically. |
| `colors.light_theme` | `extension/palette-prefs.js` | Light wallpaper row title | — | Açık tema | Light Theme |
| `colors.dark_theme` | `extension/palette-prefs.js` | Dark wallpaper row title | — | Koyu tema | Dark Theme |
| `colors.no_image` | `extension/palette-prefs.js` | Missing wallpaper subtitle | — | Görsel yok | No image |
| `colors.layers_group_title` | `extension/palette-prefs.js` | Layers group title | — | Uygulama katmanları ve takip | Desktop Layers and Sync |
| `colors.layers_group_desc` | `extension/palette-prefs.js` | Layers group description | — | Renklerin hangi masaüstü katmanlarına yansıtılacağını belirle. | Choose which desktop layers receive your adaptive wallpaper palette. |
| `colors.follow_title` | `extension/palette-prefs.js` | Follow wallpaper SwitchRow | — | Duvar kâğıdı değişince renkleri güncelle | Update Colors When Wallpaper Changes |
| `colors.follow_desc` | `extension/palette-prefs.js` | Follow wallpaper subtitle | — | Yeni görsel seçildiğinde mevcut palet karakteriyle renkleri otomatik yeniler. | Automatically re-extracts colors using your saved character whenever a wallpaper changes. |
| `colors.gtk_title` | `extension/palette-prefs.js` | GTK theme SwitchRow | — | GTK uygulamalarını da renklendir | Theme GTK Applications |
| `colors.gtk_desc` | `extension/palette-prefs.js` | GTK theme subtitle | — | GTK 3 ve klasik GTK 4 uygulamalarına uyumlu palet uygular. | Applies matching light and dark palettes to GTK 3 and classic GTK 4 apps. |
| `colors.adwaita_title` | `extension/palette-prefs.js` | Libadwaita SwitchRow | — | Libadwaita uygulamalarını renklendir | Theme Libadwaita Applications |
| `colors.adwaita_desc` | `extension/palette-prefs.js` | Libadwaita subtitle | — | GTK 4.12+ · Kullanıcı CSS’sine geri alınabilir bir bağlantı ekler. | GTK 4.12+ · Adds a clean, reversible import to your user GTK 4 CSS. |
| `colors.adwaita_refresh_title` | `extension/palette-prefs.js` | Hot-reload Libadwaita SwitchRow | — | Açık libadwaita pencerelerini yenile | Live-Refresh Open Libadwaita Windows |
| `colors.adwaita_refresh_desc` | `extension/palette-prefs.js` | Hot-reload Libadwaita subtitle | — | Yüksek kontrastı anlık açıp kapatarak açık pencereleri tazeler. | Briefly pulses high-contrast mode to reload CSS in already-open Libadwaita apps. |
| `colors.toast_saved` | `extension/palette-prefs.js` | Toast when palette saved (active) | — | Renk karakteri kaydedildi. | Palette character saved. |
| `colors.toast_saved_enable` | `extension/palette-prefs.js` | Toast when palette saved (disabled) | — | Karakter kaydedildi. Kullanmak için Duvar temasını aç. | Palette saved. Enable Use Duvar Theme to apply it. |
| `colors.pick_for_theme` | `extension/gallery-wallpaper.js` | Apply button label in Colors ({title}) | `{title}` | {title} için uygula | Apply for {title} |
| `colors.choose_for_theme` | `extension/gallery-wallpaper.js` | FileDialog title ({title}) | `{title}` | {title} için görsel seç | Choose image for {title} |
| `colors.profile_0_name` | `extension/palette.js` | Profile 0 name | — | Dengeli | Balanced |
| `colors.profile_0_desc` | `extension/palette.js` | Profile 0 desc | — | Görselin ana tonu, ölçülü renk ve net katmanlar. | Main wallpaper hue with measured saturation and crisp surfaces. |
| `colors.profile_1_name` | `extension/palette.js` | Profile 1 name | — | Pastel | Pastel |
| `colors.profile_1_desc` | `extension/palette.js` | Profile 1 desc | — | Sütlü yüzeyler, pudra tonlu kartlar ve yumuşak seçimler. | Soft milky surfaces, powdery cards, and gentle selections. |
| `colors.profile_2_name` | `extension/palette.js` | Profile 2 name | — | Canlı | Vibrant |
| `colors.profile_2_desc` | `extension/palette.js` | Profile 2 desc | — | Doygun vurgu, renkli kenarlar ve belirgin etkileşimler. | Rich accent color, tinted borders, and expressive states. |
| `colors.profile_3_name` | `extension/palette.js` | Profile 3 name | — | Toprak | Earth |
| `colors.profile_3_desc` | `extension/palette.js` | Profile 3 desc | — | Kil ve taş tonları; sıcak, mat yüzeyler. | Clay and stone tones with warm matte surfaces. |
| `colors.profile_4_name` | `extension/palette.js` | Profile 4 name | — | Serin | Cool |
| `colors.profile_4_desc` | `extension/palette.js` | Profile 4 desc | — | Maviye yaklaşan kenarlar ve soğuk, ferah yüzeyler. | Blue-leaning edges with crisp, airy cool surfaces. |
| `colors.profile_5_name` | `extension/palette.js` | Profile 5 name | — | Sıcak | Warm |
| `colors.profile_5_desc` | `extension/palette.js` | Profile 5 desc | — | Kehribara yaklaşan detaylar ve krem tonlu yüzeyler. | Amber-leaning details with creamy warm surfaces. |
| `colors.profile_6_name` | `extension/palette.js` | Profile 6 name | — | Analog | Analogous |
| `colors.profile_6_desc` | `extension/palette.js` | Profile 6 desc | — | Komşu ton: yüzeyde görselin rengi, odakta yan tonu. | Neighboring hue: wallpaper tint on surfaces, adjacent hue on focus. |
| `colors.profile_7_name` | `extension/palette.js` | Profile 7 name | — | Derin | Deep |
| `colors.profile_7_desc` | `extension/palette.js` | Profile 7 desc | — | Daha koyu taban, tok vurgu ve güçlü katman ayrımı. | Deeper base, grounded accent, and bold layer separation. |
| `colors.profile_8_name` | `extension/palette.js` | Profile 8 name | — | Yumuşak | Muted |
| `colors.profile_8_desc` | `extension/palette.js` | Profile 8 desc | — | Soluk taş tonları ve daha sakin geçişler. | Subdued stone tones and calmer transitions. |
| `colors.profile_9_name` | `extension/palette.js` | Profile 9 name | — | Mono | Monochrome |
| `colors.profile_9_desc` | `extension/palette.js` | Profile 9 desc | — | Renksiz yüzey ve kontroller; ayrımı ton farkı sağlar. | Neutral surfaces and controls separated purely by luminance. |
