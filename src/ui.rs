mod preferences;
mod topics;
mod transfer_ui;
use crate::{
    engine::{Event, Job, Pool, Request},
    model::{Config, DownloadRecord, Wallpaper},
    store::Pixels,
};
use adw::prelude::*;
use gtk::gdk;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashSet, VecDeque},
    path::Path,
    rc::{Rc, Weak},
    sync::mpsc::Receiver,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Cursor {
    rejected: usize,
    page: u32,
    loading: bool,
    exhausted: bool,
    failed: bool,
}
impl Cursor {
    fn begin(&mut self, retry: bool) -> Option<u32> {
        if self.loading || self.exhausted || (self.failed && !retry) || (retry && !self.failed) {
            return None;
        }
        self.loading = true;
        Some(self.page + 1)
    }
    fn finish(&mut self, more: bool, failed: bool) {
        self.loading = false;
        self.failed = failed;
        if !failed {
            self.page += 1;
            self.exhausted = !more;
        }
    }
}

const MAX_RESULTS: u32 = 5000;
const MAX_TEXTURES: usize = 180;

const GALLERY_CSS: &str = r#"
.duvar-gallery-grid {
    background-color: transparent;
}
.duvar-gallery-grid > child,
.duvar-gallery-grid > child:hover,
.duvar-gallery-grid > child:active,
.duvar-gallery-grid > child:focus {
    padding: 5px;
    border-radius: 14px;
    background-color: transparent;
    outline: none;
    box-shadow: none;
}
flowbox.duvar-clean-flow > flowboxchild,
flowbox.duvar-clean-flow > flowboxchild:hover,
flowbox.duvar-clean-flow > flowboxchild:active,
flowbox.duvar-clean-flow > flowboxchild:focus {
    background-color: transparent;
    box-shadow: none;
    outline: none;
    padding: 0;
    margin: 0;
}
.duvar-card {
    border-radius: 12px;
    background-color: var(--card-bg-color, alpha(currentColor, 0.05));
    border: 1px solid alpha(currentColor, 0.08);
    padding: 6px;
    transition: border-color 160ms ease, background-color 160ms ease;
}
.duvar-card:hover {
    border-color: alpha(var(--accent-bg-color, currentColor), 0.34);
    background-color: var(--card-bg-color, alpha(currentColor, 0.07));
}
.duvar-thumb-frame {
    border-radius: 9px;
    background-color: alpha(currentColor, 0.07);
}
.duvar-card-badge {
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 0.76em;
    font-weight: 700;
}
.duvar-card-osd-btn {
    min-width: 30px;
    min-height: 30px;
    padding: 0;
}
.duvar-fav-active {
    color: #f6d32d;
}
.duvar-card-title {
    font-weight: 600;
    font-size: 0.92em;
}
.duvar-card-sub {
    font-size: 0.78em;
}
.duvar-card-apply-btn {
    min-height: 29px;
    padding: 2px 5px;
    font-size: 0.78em;
    font-weight: 600;
}
.duvar-toolbar-row {
    margin-bottom: 4px;
}
.duvar-toolbar-row > button,
.duvar-toolbar-row > togglebutton,
.duvar-toolbar-row > menubutton > togglebutton,
.duvar-toolbar-row > dropdown > button {
    min-height: 34px;
}
.duvar-compat-note {
    border-radius: 10px;
    padding: 8px 12px;
    background-color: alpha(currentColor, 0.06);
}
.duvar-lightbox-canvas {
    background-color: alpha(#000000, 0.16);
}
.duvar-lightbox-pill {
    border-radius: 999px;
    padding: 6px 12px;
}
.duvar-lightbox-bottom {
    padding: 8px 14px 10px 14px;
    border-top: 1px solid alpha(currentColor, 0.09);
}
.duvar-action-btn {
    min-height: 34px;
    padding: 0 12px;
    border-radius: 9px;
    font-weight: 600;
    font-size: 0.88em;
}
.duvar-icon-btn {
    min-width: 34px;
    min-height: 34px;
    padding: 0;
}
.duvar-nav-btn {
    min-width: 38px;
    min-height: 38px;
    opacity: 0.78;
}
.duvar-nav-btn:hover {
    opacity: 1.0;
}
"#;

struct PreviewTarget {
    token: u64,
    toasts: glib::WeakRef<adw::ToastOverlay>,
    retry: glib::WeakRef<gtk::Button>,
    signature: RefCell<String>,
    item: Wallpaper,
    ring: transfer_ui::Ring,
    actions: glib::WeakRef<gtk::Box>,
    details: glib::WeakRef<gtk::Label>,
    id: String,
    picture: glib::WeakRef<gtk::Picture>,
    loading_pill: glib::WeakRef<gtk::Box>,
    spinner: glib::WeakRef<gtk::Spinner>,
    label: glib::WeakRef<gtk::Label>,
    zoom: glib::WeakRef<gtk::ToggleButton>,
    dimensions: Rc<Cell<(i32, i32)>>,
}

struct Entry {
    wallpaper: Wallpaper,
    rank: u64,
    texture: RefCell<Option<gdk::Texture>>,
}

#[allow(
    deprecated,
    reason = "GNOME 51 supplies an AdwPreferencesWindow host, not a PreferencesDialog"
)]
struct State {
    window: glib::WeakRef<adw::PreferencesWindow>,
    settings: gio::Settings,
    config: RefCell<Config>,
    pool: Pool,
    receiver: Receiver<Event>,
    request: RefCell<Request>,
    poll: RefCell<Option<glib::SourceId>>,
    model: gio::ListStore,
    gallery_grid: glib::WeakRef<gtk::GridView>,
    downloaded_grid: glib::WeakRef<gtk::GridView>,
    status: gtk::Label,
    spinner: gtk::Spinner,
    empty: adw::StatusPage,
    explore_scroll: glib::WeakRef<gtk::ScrolledWindow>,
    search: gtk::SearchEntry,
    compat_bar: gtk::Box,
    compat_label: gtk::Label,
    cursors: RefCell<BTreeMap<String, Cursor>>,
    seen: RefCell<HashSet<String>>,
    thumbnail_pending: RefCell<HashSet<String>>,
    texture_cache: RefCell<BTreeMap<String, gdk::Texture>>,
    texture_lru: RefCell<VecDeque<String>>,
    action_cancel: Arc<AtomicBool>,
    queue: RefCell<Vec<DownloadRecord>>,
    latest_apply: Cell<crate::queue::ApplyOrder>,
    queue_cancels: RefCell<BTreeMap<u64, Arc<AtomicBool>>>,
    queue_label: gtk::Label,
    queue_list: gtk::ListBox,
    downloads_model: gio::ListStore,
    retry: gtk::Button,
    more: gtk::Button,
    key_entry: glib::WeakRef<adw::PasswordEntryRow>,
    key_edited: Cell<bool>,
    key_save: glib::WeakRef<gtk::Button>,
    favorites: Cell<bool>,
    api_key: RefCell<String>,
    key_looked: Cell<bool>,
    key_registered: Cell<bool>,
    key_status: glib::WeakRef<adw::ActionRow>,
    closed: Cell<bool>,
    errors: RefCell<Vec<String>>,
    preview_picture: RefCell<Option<PreviewTarget>>,
    topic_summary: glib::WeakRef<adw::ActionRow>,
    sources_summary: glib::WeakRef<adw::ActionRow>,
    filters_page: glib::WeakRef<adw::PreferencesPage>,
    topic_bar: gtk::FlowBox,
    preview_cancel: RefCell<Arc<AtomicBool>>,
    library: RefCell<crate::library::Index>,
    facts: RefCell<BTreeMap<String, crate::runtime::Facts>>,
    cards: RefCell<Vec<transfer_ui::CardTarget>>,
    queue_rows: RefCell<BTreeMap<u64, glib::WeakRef<adw::ActionRow>>>,
    queue_paused: Cell<bool>,
    library_summary: gtk::Label,
    last_saved: RefCell<String>,
    last_trash: RefCell<Option<crate::files::TrashTicket>>,
    save_source: RefCell<Option<glib::SourceId>>,
    css_provider: RefCell<Option<gtk::CssProvider>>,
}

impl State {
    #[allow(
        deprecated,
        reason = "Toast belongs to GNOME 51 PreferencesWindow host"
    )]
    fn toast(&self, text: &str) {
        self.add_toast(adw::Toast::new(text));
    }
    #[allow(deprecated, reason = "GNOME preferences host")]
    fn add_toast(&self, toast: adw::Toast) {
        if let Some(overlay) = self
            .preview_picture
            .borrow()
            .as_ref()
            .and_then(|t| t.toasts.upgrade())
        {
            overlay.add_toast(toast);
        } else if let Some(window) = self.window.upgrade() {
            window.add_toast(toast);
        }
    }
    fn cache_texture(&self, id: String, tex: gdk::Texture) {
        let mut lru = self.texture_lru.borrow_mut();
        if let Some(pos) = lru.iter().position(|x| x == &id) {
            lru.remove(pos);
        }
        lru.push_back(id.clone());
        let mut cache = self.texture_cache.borrow_mut();
        cache.insert(id, tex);
        while lru.len() > MAX_TEXTURES {
            if let Some(old) = lru.pop_front() {
                cache.remove(&old);
                // Clear Entry reference only if not currently bound to a visible card
                let visible = self
                    .cards
                    .borrow()
                    .iter()
                    .any(|c| c.item.id == old && c.root.upgrade().is_some());
                if !visible {
                    for model in [&self.model, &self.downloads_model] {
                        for i in 0..model.n_items() {
                            if let Some(o) = model.item(i).and_downcast::<glib::BoxedAnyObject>() {
                                let e = o.borrow::<Entry>();
                                if e.wallpaper.id == old {
                                    e.texture.replace(None);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fn cached_texture(&self, id: &str) -> Option<gdk::Texture> {
        let tex = self.texture_cache.borrow().get(id).cloned()?;
        let mut lru = self.texture_lru.borrow_mut();
        if let Some(pos) = lru.iter().position(|x| x == id) {
            lru.remove(pos);
            lru.push_back(id.to_string());
        }
        Some(tex)
    }
    fn sync_compat_hints(&self) {
        let cfg = self.config.borrow();
        let notes = crate::registry::compatibility_notes(&cfg);
        if notes.is_empty() || self.favorites.get() {
            self.compat_bar.set_visible(false);
        } else {
            self.compat_label.set_text(&notes.join(" · "));
            self.compat_bar.set_visible(true);
        }
        if let Some(row) = self.sources_summary.upgrade() {
            let active = crate::registry::enabled(&cfg)
                .iter()
                .map(|s| s.label)
                .collect::<Vec<_>>();
            row.set_subtitle(&if active.is_empty() {
                "Etkin kaynak yok".into()
            } else {
                format!("{} etkin kaynak · {}", active.len(), active.join(", "))
            });
        }
    }
    fn save(self: &Rc<Self>) {
        if !crate::store::config_writable() {
            self.toast("Daha yeni ayar şeması korunuyor; kayıt kapalı.");
            return;
        }
        self.config.borrow_mut().validate();
        let text = serde_json::to_string(&self.config.borrow().persistent()).unwrap_or_default();
        if *self.last_saved.borrow() == text {
            return;
        }
        self.last_saved.replace(text);
        if let Some(id) = self.save_source.borrow_mut().take() {
            id.remove();
        }
        let w = weak(self);
        let id = glib::timeout_add_local_once(Duration::from_millis(200), move || {
            if let Some(s) = w.upgrade() {
                s.save_source.borrow_mut().take();
                let cfg = s.config.borrow().persistent();
                if let Ok(text) = serde_json::to_string(&cfg)
                    && let Err(e) = s.settings.set_string("configuration", &text)
                {
                    s.toast(&format!("GNOME ayar kaydı başarısız: {e}"));
                }
                if !s
                    .pool
                    .push(Job::SaveConfig(crate::store::config_revision(), cfg))
                {
                    s.last_saved.borrow_mut().clear();
                    s.toast("Ayar kaydı sıraya alınamadı; tekrar dene.");
                }
                s.ensure_poll();
            }
        });
        self.save_source.replace(Some(id));
    }
    fn current(&self) -> Request {
        self.request.borrow().clone()
    }
    fn changed(self: &Rc<Self>) {
        self.save();
        self.sync_compat_hints();
        self.render_library();
        self.load();
    }
    fn load(self: &Rc<Self>) {
        self.request.borrow().cancel.store(true, Ordering::Relaxed);
        self.pool.cancel_queued();
        let number = self.request.borrow().number + 1;
        self.request.replace(Request {
            number,
            cancel: Arc::new(AtomicBool::new(false)),
        });
        self.model.remove_all();
        if let Some(scroll) = self.explore_scroll.upgrade() {
            scroll.vadjustment().set_value(0.0);
        }
        self.seen.borrow_mut().clear();
        self.thumbnail_pending.borrow_mut().clear();
        self.errors.borrow_mut().clear();
        self.status.set_tooltip_text(None);
        let cfg = self.config.borrow().clone();
        self.cursors.borrow_mut().clear();
        for source in crate::registry::enabled(&cfg) {
            self.cursors
                .borrow_mut()
                .insert(source.id.into(), Cursor::default());
        }
        self.sync_compat_hints();
        if self.favorites.get() {
            self.add_items(
                cfg.favorites
                    .iter()
                    .filter(|w| {
                        cfg.policy.accepts(w)
                            && (cfg.query.is_empty()
                                || crate::catalog::search_key(&format!(
                                    "{} {}",
                                    w.title,
                                    w.tags.as_ref().map(|t| t.join(" ")).unwrap_or_default()
                                ))
                                .contains(&crate::catalog::search_key(&cfg.query)))
                    })
                    .cloned()
                    .collect(),
            );
            self.update_status();
        } else {
            self.load_more(false);
        }
        if !self.key_looked.replace(true) {
            self.pool.push(Job::LoadKey("wallhaven".into()));
        }
        self.ensure_poll();
    }
    fn load_more(self: &Rc<Self>, retry: bool) {
        if self.favorites.get() || self.model.n_items() >= MAX_RESULTS {
            return;
        }
        let cfg = self.config.borrow().clone();
        let request = self.current();
        for (name, cursor) in self.cursors.borrow_mut().iter_mut() {
            let Some(page) = cursor.begin(retry) else {
                continue;
            };
            if !self.pool.push(Job::Search(
                request.clone(),
                name.clone(),
                cfg.clone(),
                page,
                if name == "Wallhaven" {
                    self.api_key.borrow().clone()
                } else {
                    String::new()
                },
            )) {
                cursor.loading = false;
            }
        }
        self.update_status();
        self.ensure_poll();
    }
    fn update_status(&self) {
        let cursors = self.cursors.borrow();
        let busy = cursors.values().any(|c| c.loading);
        let failures = cursors
            .iter()
            .filter(|(_, c)| c.failed)
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        self.retry.set_visible(!failures.is_empty());
        let count = self.model.n_items();
        self.more.set_visible(
            !self.favorites.get()
                && count < MAX_RESULTS
                && cursors.values().any(|c| !c.exhausted && !c.failed),
        );
        self.more.set_sensitive(!busy);
        let message = if cursors.is_empty() && !self.favorites.get() {
            "Kaynak seçilmemiş. Ayarlar > Görsel kaynakları bölümünden bir koleksiyon aç.".into()
        } else if !failures.is_empty() {
            format!(
                "{count} görsel · {} yanıt vermedi; diğer sonuçlar korunuyor",
                failures.join(", ")
            )
        } else if busy {
            format!("{count} görsel · Yükleniyor…")
        } else if count >= MAX_RESULTS {
            format!("{count} görsel · Bellek sınırına ulaşıldı; yeni arama başlatabilirsin")
        } else if count == 0 {
            "Sonuç yok. Kaynakları ve filtreleri kontrol et.".into()
        } else if self.favorites.get() || cursors.values().all(|c| c.exhausted) {
            format!("{count} görsel · Tüm sonuçlar yüklendi")
        } else {
            format!("{count} görsel · Devamı için aşağı kaydır")
        };
        let cfg = self.config.borrow();
        self.empty.set_visible(count == 0);
        if let Some(scroll) = self.explore_scroll.upgrade() {
            scroll.set_visible(count > 0);
        }
        self.empty.set_title(if busy {
            "Kaynaklardan sonuç bekleniyor"
        } else if !failures.is_empty() {
            "Kaynaklara erişilemedi"
        } else if self.favorites.get() {
            "Eşleşen favori yok"
        } else {
            "Eşleşen görsel yok"
        });
        self.empty.set_description(Some(if busy {
            "İlk sonuçlar geldikçe galeri açılır. Diğer işler arka planda devam eder."
        } else {
            "Kaynakları ve içerik izinlerini kontrol et; sorguyu veya filtreleri değiştir."
        }));
        let rejected: usize = cursors.values().map(|c| c.rejected).sum();
        let active_sources = crate::registry::enabled(&cfg).len();
        let active_filters = cfg.policy.active_filters();
        let mut parts = vec![message, format!("{active_sources} kaynak")];
        if active_filters > 0 {
            parts.push(format!("{active_filters} filtre"));
        }
        if rejected > 0 {
            parts.push(format!("{rejected} elendi"));
        }
        self.status.set_text(&parts.join(" · "));
    }
    fn request_thumbnail(self: &Rc<Self>, item: Wallpaper, downloaded: bool) {
        let request = if downloaded {
            Request {
                number: u64::MAX,
                cancel: self.action_cancel.clone(),
            }
        } else {
            self.current()
        };
        let pending_id = format!("{}:{}", request.number, item.id);
        if self.thumbnail_pending.borrow().contains(&pending_id) {
            return;
        }
        if self.pool.push(Job::Thumbnail(request, item)) {
            self.thumbnail_pending.borrow_mut().insert(pending_id);
            self.ensure_poll();
        }
    }
    fn add_items(self: &Rc<Self>, items: Vec<Wallpaper>) {
        let order = self.config.borrow().gallery_order.clone();
        let seed = self.current().number;
        let scrolled_down = self
            .explore_scroll
            .upgrade()
            .is_some_and(|sc| sc.vadjustment().value() > 40.0);
        let mut batch = Vec::new();
        for wallpaper in items {
            if self.model.n_items() + batch.len() as u32 >= MAX_RESULTS {
                break;
            }
            if let Some(reason) = self.config.borrow().policy.explain(&wallpaper) {
                crate::diagnostics::log(Some(seed), "filter", "rejected", reason, None);
                continue;
            }
            if !self.seen.borrow_mut().insert(wallpaper.id.clone()) {
                continue;
            }
            let cached = self.cached_texture(&wallpaper.id);
            let rank = (self.model.n_items() as usize + batch.len()) as u64;
            batch.push(glib::BoxedAnyObject::new(Entry {
                wallpaper,
                rank,
                texture: RefCell::new(cached),
            }));
        }
        if batch.is_empty() {
            return;
        }
        // Never insert new infinite-scroll pages ABOVE a scrolled viewport (causes scroll jump!).
        if order == "source" || (scrolled_down && self.model.n_items() > 0) {
            if order != "source" {
                batch.sort_by(|a, b| compare(a.upcast_ref(), b.upcast_ref(), &order, seed));
            }
            let pos = self.model.n_items();
            self.model.splice(pos, 0, &batch);
        } else {
            for entry in batch {
                self.model
                    .insert_sorted(&entry, |a, b| compare(a, b, &order, seed));
            }
        }
    }
    fn sort_gallery(&self) {
        self.resort_gallery(true);
    }
    fn resort_gallery(&self, top: bool) {
        let focus = self.gallery_grid.upgrade().is_some_and(|g| g.has_focus());
        let order = self.config.borrow().gallery_order.clone();
        let mut items = (0..self.model.n_items())
            .filter_map(|i| self.model.item(i))
            .collect::<Vec<_>>();
        items.sort_by(|a, b| compare(a, b, &order, self.current().number));
        self.model.splice(0, self.model.n_items(), &items);
        if top
            && !items.is_empty()
            && let Some(grid) = self.gallery_grid.upgrade()
        {
            if let Some(sc) = self.explore_scroll.upgrade() {
                sc.vadjustment().set_value(0.0);
            }
            grid.scroll_to(
                0,
                if focus {
                    gtk::ListScrollFlags::FOCUS
                } else {
                    gtk::ListScrollFlags::empty()
                },
                None,
            );
        }
    }
    fn ensure_poll(self: &Rc<Self>) {
        if self.poll.borrow().is_some() || self.closed.get() {
            return;
        }
        self.spinner.start();
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local(Duration::from_millis(16), move || {
            let Some(state) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            for p in state.pool.progress() {
                state.update_progress(p);
            }
            let started = std::time::Instant::now();
            let mut drained = false;
            // Higher per-frame batch (16) capped at 6ms keeps 8 thumbnail workers silky at 60 FPS.
            for _ in 0..16 {
                if started.elapsed() >= Duration::from_millis(6) {
                    break;
                }
                match state.receiver.try_recv() {
                    Ok(event) => state.event(event),
                    Err(_) => {
                        drained = true;
                        break;
                    }
                }
            }
            if state.closed.get() || (state.pool.pending() == 0 && drained) {
                state.spinner.stop();
                state.poll.borrow_mut().take();
                state.update_status();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        *self.poll.borrow_mut() = Some(id);
    }
    #[allow(deprecated, reason = "GNOME 51 PreferencesWindow toast host")]
    fn event(self: &Rc<Self>, event: Event) {
        match event {
            Event::Library(index) => {
                self.library.replace(index);
                self.render_library();
                // Refresh only cards whose action signature changed (zero focus/scroll disruption).
                self.refresh_cards(None, true);
            }
            Event::Metadata(id, facts) => {
                for f in &mut self.config.borrow_mut().favorites {
                    if f.id == id {
                        f.width = facts.width;
                        f.height = facts.height;
                    }
                }
                self.facts.borrow_mut().insert(id.clone(), facts.clone());
                for model in [&self.model, &self.downloads_model] {
                    for i in 0..model.n_items() {
                        if let Some(o) = model.item(i).and_downcast::<glib::BoxedAnyObject>() {
                            let mut e = o.borrow_mut::<Entry>();
                            if e.wallpaper.id == id {
                                e.wallpaper.width = facts.width;
                                e.wallpaper.height = facts.height;
                            }
                        }
                    }
                }
                // Update in place — NEVER splice/re-sort Discover while the user is browsing!
                self.refresh_cards(Some(&id), false);
            }
            Event::Notice(text) => self.toast(&text),
            Event::Trashed(ticket) => {
                self.last_trash.replace(Some(ticket.clone()));
                self.library.borrow_mut().remove_path(&ticket.record.path);
                self.render_library();
                self.refresh_cards(Some(&ticket.record.item.id), true);
                if let Some(a) = self
                    .preview_picture
                    .borrow()
                    .as_ref()
                    .and_then(|t| t.actions.upgrade())
                {
                    a.child_focus(gtk::DirectionType::TabForward);
                } else if let Some(w) = self.window.upgrade() {
                    let downloaded = w
                        .visible_page()
                        .is_some_and(|p| p.title() == "İndirilenler");
                    if let Some(grid) = if downloaded {
                        self.downloaded_grid.upgrade()
                    } else {
                        self.gallery_grid.upgrade()
                    } {
                        grid.grab_focus();
                    }
                }
                if self.window.upgrade().is_some() {
                    let toast = adw::Toast::builder()
                        .title("Görsel çöp kutusuna taşındı.")
                        .button_label("Geri al")
                        .priority(adw::ToastPriority::High)
                        .timeout(15)
                        .build();
                    let weak = weak(self);
                    toast.connect_button_clicked(move |_| {
                        if let Some(s) = weak.upgrade() {
                            s.pool.push(Job::Restore(ticket.clone()));
                            s.ensure_poll();
                        }
                    });
                    self.add_toast(toast);
                }
            }
            Event::ConfigSaved => {}
            Event::ConfigFailed(error) => {
                self.last_saved.borrow_mut().clear();
                self.toast(&format!("Ayar kaydedilemedi: {error}"));
            }
            Event::Restored(record) => {
                self.last_trash.borrow_mut().take();
                self.facts.borrow_mut().insert(record.item.id, record.facts);
                self.refresh_downloads();
                self.toast("Görsel geri yüklendi.");
            }
            Event::DownloadStopped(id) => {
                self.queue_cancels.borrow_mut().remove(&id);
                if self
                    .queue
                    .borrow()
                    .iter()
                    .any(|r| r.id == id && r.status == "resume_requested")
                {
                    self.start_download(id);
                    self.queue_changed();
                }
                self.refresh_downloads();
            }
            Event::Items(u64::MAX, items) => {
                self.downloads_model.remove_all();
                for wallpaper in items {
                    if self.config.borrow().policy.accepts(&wallpaper) {
                        let cached = self.cached_texture(&wallpaper.id);
                        self.downloads_model
                            .append(&glib::BoxedAnyObject::new(Entry {
                                wallpaper,
                                rank: self.downloads_model.n_items() as u64,
                                texture: RefCell::new(cached),
                            }));
                    }
                }
            }
            Event::Items(n, items) if n == self.current().number => self.add_items(items),
            Event::SearchFinished(n, provider, more, rejected, error)
                if n == self.current().number =>
            {
                if let Some(cursor) = self.cursors.borrow_mut().get_mut(&provider) {
                    cursor.rejected += rejected;
                    cursor.finish(more, error.is_some());
                }
                if let Some(error) = error {
                    if self.errors.borrow().len() == 8 {
                        self.errors.borrow_mut().remove(0);
                    }
                    self.errors.borrow_mut().push(error.clone());
                    self.status
                        .set_tooltip_text(Some(&self.errors.borrow().join("\n")));
                    self.toast(&error);
                }
                self.update_status();
            }
            Event::ThumbnailFailed(n, id) if n == self.current().number || n == u64::MAX => {
                self.thumbnail_pending
                    .borrow_mut()
                    .remove(&format!("{n}:{id}"));
            }
            Event::Thumbnail(n, id, pixels) if n == self.current().number || n == u64::MAX => {
                self.thumbnail_pending
                    .borrow_mut()
                    .remove(&format!("{n}:{id}"));
                let tex = texture(pixels);
                self.cache_texture(id.clone(), tex.clone());
                // IN-PLACE update: NEVER call model.splice() on thumbnail arrival!
                for model in [&self.model, &self.downloads_model] {
                    for i in 0..model.n_items() {
                        if let Some(o) = model.item(i).and_downcast::<glib::BoxedAnyObject>() {
                            let e = o.borrow::<Entry>();
                            if e.wallpaper.id == id {
                                e.texture.replace(Some(tex.clone()));
                            }
                        }
                    }
                }
                for target in self.cards.borrow().iter() {
                    if target.item.id == id
                        && let Some(picture) = target.picture.upgrade()
                    {
                        picture.set_paintable(Some(&tex));
                    }
                }
            }
            Event::Preview(token, id, pixels) => {
                if let Some(target) = self
                    .preview_picture
                    .borrow()
                    .as_ref()
                    .filter(|t| t.id == id && t.token == token)
                {
                    if let Some(retry) = target.retry.upgrade() {
                        retry.set_visible(false);
                    }
                    let dimensions = (pixels.width as i32, pixels.height as i32);
                    target.dimensions.set(dimensions);
                    if let Some(picture) = target.picture.upgrade() {
                        picture.set_paintable(Some(&texture(pixels)));
                        picture.update_state(&[gtk::accessible::State::Busy(false)]);
                    }
                    if let Some(spinner) = target.spinner.upgrade() {
                        spinner.stop();
                    }
                    if let Some(pill) = target.loading_pill.upgrade() {
                        pill.set_visible(false);
                    }
                    if let Some(zoom) = target.zoom.upgrade() {
                        zoom.set_sensitive(true);
                    }
                    if let Some(label) = target.label.upgrade() {
                        label.set_text(&format!(
                            "Özgün dosya · {} × {} piksel · Ekrana sığdırılıyor",
                            dimensions.0, dimensions.1
                        ));
                    }
                }
            }
            Event::PreviewFailed(token, id, error) => {
                if let Some(target) = self
                    .preview_picture
                    .borrow()
                    .as_ref()
                    .filter(|t| t.id == id && t.token == token)
                {
                    if let Some(spinner) = target.spinner.upgrade() {
                        spinner.stop();
                    }
                    if let Some(pill) = target.loading_pill.upgrade() {
                        pill.set_visible(false);
                    }
                    if let Some(picture) = target.picture.upgrade() {
                        picture.update_state(&[gtk::accessible::State::Busy(false)]);
                    }
                    if let Some(label) = target.label.upgrade() {
                        label.set_text(&format!("Özgün önizleme açılamadı: {error}"));
                        if let Some(retry) = target.retry.upgrade() {
                            retry.set_visible(true);
                        }
                    }
                }
            }
            Event::ActionFailure(id, error) => {
                if let Some(record) =
                    self.queue.borrow_mut().iter_mut().find(|r| {
                        r.id == id && crate::runtime::Phase::from_status(&r.status).pending()
                    })
                {
                    record.status = "failed".into();
                    record.error = crate::errors::redact(&error.message, true);
                    record.last_error_code = format!("{:?}", error.code);
                    record.updated_at = crate::runtime::now();
                }
                self.queue_cancels.borrow_mut().remove(&id);
                self.queue_changed();
                self.toast(&format!("İşlem tamamlanamadı: {error}"));
            }
            Event::DownloadStarted(id) => {
                if let Some(record) = self.queue.borrow_mut().iter_mut().find(|r| r.id == id)
                    && crate::runtime::Phase::from_status(&record.status).pending()
                {
                    record.status = "waiting".into();
                }
                self.queue_changed();
            }
            Event::Downloaded(id, item, path, scope) => {
                if let Some(record) = self.queue.borrow_mut().iter_mut().find(|r| r.id == id) {
                    record.status = "done".into();
                    record.path = path.to_string_lossy().into();
                    record.error.clear();
                }
                self.queue_cancels.borrow_mut().remove(&id);
                self.queue_changed();
                self.refresh_downloads();
                self.refresh_cards(Some(&item.id), true);
                if let Some(scope) = scope
                    .as_deref()
                    .and_then(|scope| self.latest_apply.get().pending_scope(scope, id))
                {
                    match apply(&path, scope) {
                        Ok(()) => self.toast(wallpaper_applied(scope)),
                        Err(e) => self.toast(&format!("Uygulanamadı: {e}")),
                    }
                } else {
                    self.toast(&format!("İndirildi: {}", path.display()))
                }
            }
            Event::Failure(n, error) if n == 0 || n == self.current().number => {
                if n == 0
                    && let Some(button) = self.key_save.upgrade()
                {
                    button.set_sensitive(true);
                }
                if self.errors.borrow().len() < 4 {
                    if self.errors.borrow().len() == 8 {
                        self.errors.borrow_mut().remove(0);
                    }
                    self.errors.borrow_mut().push(error.clone());
                }
                self.status
                    .set_tooltip_text(Some(&self.errors.borrow().join("\n")));
                self.toast(&error)
            }
            Event::KeyFailure(provider, error) => {
                if let Some(button) = self.key_save.upgrade() {
                    button.set_sensitive(true);
                }
                self.toast(&format!("{provider}: {error}"));
            }
            Event::KeyRemoved => {
                self.key_registered.set(false);
                if let Some(r) = self.key_status.upgrade() {
                    r.set_subtitle("Anahtarlıkta Duvar anahtarı yok");
                }
                self.api_key.borrow_mut().clear();
                self.key_edited.set(true);
                if let Some(e) = self.key_entry.upgrade() {
                    e.set_text("");
                }
                self.toast("Duvar API anahtarı kaldırıldı; anahtarlık parolası değişmedi.");
                self.load();
            }
            Event::KeySaved => {
                self.key_registered.set(true);
                if let Some(r) = self.key_status.upgrade() {
                    r.set_subtitle("Mevcut anahtarlığa kaydedildi; değer gizli");
                }
                if let Some(button) = self.key_save.upgrade() {
                    button.set_sensitive(true);
                }
                self.toast("API anahtarı mevcut anahtarlığa kaydedildi.");
                self.load();
            }
            Event::KeyLoaded(key) if !key.is_empty() && !self.key_edited.get() => {
                self.api_key.replace(key.clone());
                if let Some(entry) = self.key_entry.upgrade() {
                    entry.set_text(&key);
                }
                if self.config.borrow().policy.adult {
                    self.load();
                }
            }
            _ => {}
        }
    }
    fn favorite(self: &Rc<Self>, item: &Wallpaper) {
        let mut cfg = self.config.borrow_mut();
        if let Some(i) = cfg.favorites.iter().position(|w| w.id == item.id) {
            cfg.favorites.remove(i);
            self.toast("Favorilerden çıkarıldı.")
        } else if cfg.favorites.len() < 128 {
            cfg.favorites.push(item.clone());
            self.toast("Favorilere eklendi.")
        } else {
            self.toast("Favori sınırına ulaşıldı (128).")
        }
        drop(cfg);
        self.save();
        if self.favorites.get() {
            self.load();
        }
    }
    fn refresh_downloads(self: &Rc<Self>) {
        self.pool.push(Job::Downloads(self.config.borrow().clone()));
        self.ensure_poll();
    }
    fn render_library(self: &Rc<Self>) {
        let index = self.library.borrow();
        let summary = index.summary();
        self.library_summary.set_text(&format!(
            "{} dosya · {} · {} eksik",
            summary.files,
            crate::runtime::size(summary.bytes),
            summary.missing
        ));
        let items = index.filtered(&self.config.borrow());
        if items.is_empty() {
            self.library_summary.set_text(if index.entries.is_empty() {
                "Henüz indirilmiş görsel yok. Keşfet’ten bir görsel indir."
            } else {
                "Bu filtrelerle eşleşen dosya yok; filtreleri veya eksik dosyalar görünümünü değiştir."
            });
        }
        drop(index);
        // Hydrate textures directly from LRU cache so downloads refresh instantaneously without flicker.
        let entries = items
            .into_iter()
            .enumerate()
            .map(|(i, wallpaper)| {
                let cached = self.cached_texture(&wallpaper.id);
                glib::BoxedAnyObject::new(Entry {
                    wallpaper,
                    rank: i as u64,
                    texture: RefCell::new(cached),
                })
            })
            .collect::<Vec<_>>();
        self.downloads_model
            .splice(0, self.downloads_model.n_items(), &entries);
    }
    fn queue_changed(self: &Rc<Self>) {
        self.pool.push(Job::SaveQueue(
            crate::store::queue_revision(),
            self.queue.borrow().clone(),
            self.latest_apply.get(),
            self.queue_paused.get(),
        ));
        self.ensure_poll();
        // refresh_cards checks action_signature per card, so unchanged cards are untouched.
        self.refresh_cards(None, true);
        self.queue_rows.borrow_mut().clear();
        let queue = self.queue.borrow();
        let active = queue
            .iter()
            .filter(|r| crate::runtime::Phase::from_status(&r.status).active())
            .count();
        let waiting = queue.iter().filter(|r| r.status == "queued").count();
        self.queue_label.set_text(&format!(
            "{active} etkin · {waiting} sırada · {} başarısız · {} tamamlandı{}",
            queue.iter().filter(|r| r.status == "failed").count(),
            queue.iter().filter(|r| r.status == "done").count(),
            if self.queue_paused.get() {
                " · Duraklatıldı"
            } else {
                ""
            }
        ));
        while let Some(child) = self.queue_list.first_child() {
            self.queue_list.remove(&child);
        }
        let cfg = self.config.borrow();
        let query = crate::catalog::search_key(&cfg.behavior.queue_query);
        for record in queue
            .iter()
            .rev()
            .filter(|r| {
                (cfg.behavior.queue_filter == "all"
                    || r.status == cfg.behavior.queue_filter
                    || (cfg.behavior.queue_filter == "active"
                        && crate::runtime::Phase::from_status(&r.status).active()))
                    && (query.is_empty()
                        || crate::catalog::search_key(&format!("{} {}", r.item.title, r.status))
                            .contains(&query))
            })
            .take(30)
        {
            let subtitle = match record.status.as_str() {
                "paused" => "Duraklatıldı",
                "active" => "İndiriliyor…",
                "queued" => "Sırada",
                "done" => "Tamamlandı",
                "cancelled" => "İptal edildi",
                _ => &record.error,
            };
            let row = adw::ActionRow::builder()
                .title(&record.item.title)
                .subtitle(subtitle)
                .use_markup(false)
                .build();
            let w = weak(self);
            let id = record.id;
            row.set_title_lines(1);
            row.set_subtitle_lines(2);
            row.set_tooltip_text(Some(&format!("{}\n{}", record.item.title, record.error)));
            self.queue_rows.borrow_mut().insert(id, row.downgrade());
            if crate::runtime::Phase::from_status(&record.status).pending() {
                let cancel = gtk::Button::from_icon_name("process-stop-symbolic");
                cancel.set_valign(gtk::Align::Center);
                cancel.add_css_class("flat");
                cancel.set_tooltip_text(Some("İndirmeyi iptal et"));
                cancel.connect_clicked(move |_| {
                    if let Some(s) = w.upgrade() {
                        s.cancel_download(id, false);
                    }
                });
                row.add_suffix(&cancel);
                let pause = gtk::Button::from_icon_name("media-playback-pause-symbolic");
                pause.set_tooltip_text(Some("Duraklat; devamında baştan aktarılır"));
                pause.set_valign(gtk::Align::Center);
                pause.add_css_class("flat");
                let w = weak(self);
                pause.connect_clicked(move |_| {
                    if let Some(s) = w.upgrade() {
                        s.cancel_download(id, true);
                    }
                });
                row.add_suffix(&pause);
                let ahead = gtk::Button::from_icon_name("go-top-symbolic");
                ahead.set_tooltip_text(Some("Bekleyen işi öne al"));
                ahead.set_valign(gtk::Align::Center);
                ahead.add_css_class("flat");
                let w = weak(self);
                ahead.connect_clicked(move |_| {
                    if let Some(s) = w.upgrade() {
                        s.pool.prioritize(id);
                    }
                });
                row.add_suffix(&ahead);
            } else if matches!(record.status.as_str(), "failed" | "paused" | "cancelled") {
                let retry = gtk::Button::from_icon_name("view-refresh-symbolic");
                retry.set_valign(gtk::Align::Center);
                retry.add_css_class("flat");
                retry.set_tooltip_text(Some("Tekrar dene"));
                retry.connect_clicked(move |button| {
                    button.set_visible(false);
                    if let Some(s) = w.upgrade() {
                        s.start_download(id);
                        s.queue_changed();
                    }
                });
                row.add_suffix(&retry);
            }
            self.queue_list.append(&row);
        }
    }
    fn start_download(self: &Rc<Self>, id: u64) {
        if self.queue_paused.get() {
            if let Some(r) = self.queue.borrow_mut().iter_mut().find(|r| r.id == id) {
                r.status = "paused".into();
            }
            return;
        }
        if self.queue_cancels.borrow().contains_key(&id) {
            if let Some(r) = self
                .queue
                .borrow_mut()
                .iter_mut()
                .find(|r| r.id == id && r.status == "paused")
            {
                r.status = "resume_requested".into();
            }
            return;
        }
        let Some(record) = self.queue.borrow().iter().find(|r| r.id == id).cloned() else {
            return;
        };
        let scope = record.scope.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let request = Request {
            number: 0,
            cancel: cancel.clone(),
        };
        if self.pool.push(Job::Download(
            id,
            request,
            record.item,
            record.directory.into(),
            record.scope,
        )) {
            if let Some(scope) = scope {
                self.mark_wallpaper_intent(&scope, id);
            }
            self.queue_cancels.borrow_mut().insert(id, cancel);
            if let Some(record) = self.queue.borrow_mut().iter_mut().find(|r| r.id == id) {
                record.status = "queued".into();
                record.error.clear();
                record.attempts = record.attempts.saturating_add(1);
                record.updated_at = crate::runtime::now();
                record.progress = None;
            }
            self.ensure_poll();
        } else {
            self.toast("İndirme kuyruğu dolu; tekrar dene.");
        }
    }
    fn mark_wallpaper_intent(&self, scope: &str, id: u64) {
        let mut order = self.latest_apply.get();
        order.mark(scope, id);
        self.latest_apply.set(order);
    }
    fn apply_wallpaper(self: &Rc<Self>, path: &Path, scope: &str) -> anyhow::Result<()> {
        apply(path, scope)?;
        self.mark_wallpaper_intent(scope, crate::queue::next_id());
        self.queue_changed();
        self.toast(wallpaper_applied(scope));
        Ok(())
    }
    fn download(self: &Rc<Self>, item: Wallpaper, target: Option<&str>) {
        let cfg = self.config.borrow();
        let scope = target.map(str::to_owned);
        let directory = if scope.is_some() {
            glib::user_data_dir().join("duvar/wallpapers")
        } else if cfg.download_directory.is_empty() {
            crate::store::downloads()
        } else {
            cfg.download_directory.clone().into()
        };
        drop(cfg);
        if self.queue.borrow().iter().any(|r| {
            r.item.id == item.id
                && r.directory == directory.to_string_lossy()
                && r.scope == scope
                && matches!(r.status.as_str(), "queued" | "active")
        }) {
            self.toast("Bu görsel zaten indirme kuyruğunda.");
            return;
        }
        while self.queue.borrow().len() >= 200 {
            let old = self
                .queue
                .borrow()
                .iter()
                .position(|r| matches!(r.status.as_str(), "done" | "cancelled"));
            if let Some(i) = old {
                self.queue.borrow_mut().remove(i);
            } else {
                break;
            }
        }
        if self.queue.borrow().len() >= 200 {
            self.toast("Kuyruk sınırı 200; önce bazı işleri tamamla.");
            return;
        }
        let id = crate::queue::next_id();
        self.queue.borrow_mut().push(DownloadRecord {
            id,
            item,
            directory: directory.to_string_lossy().into(),
            scope,
            status: "queued".into(),
            error: String::new(),
            path: String::new(),
            created_at: crate::runtime::now(),
            updated_at: crate::runtime::now(),
            attempts: 0,
            last_error_code: String::new(),
            progress: None,
        });
        self.start_download(id);
        self.queue_changed();
        self.toast("İndirme kuyruğa eklendi; gezinmeye devam edebilirsin.");
    }
}

fn texture(p: Pixels) -> gdk::Texture {
    gdk::MemoryTexture::new(
        p.width as i32,
        p.height as i32,
        gdk::MemoryFormat::R8g8b8a8,
        &glib::Bytes::from_owned(p.rgba),
        p.width as usize * 4,
    )
    .upcast()
}

fn apply(path: &Path, scope: &str) -> anyhow::Result<()> {
    let _file = crate::files::regular(path)?;
    let settings = gio::Settings::new("org.gnome.desktop.background");
    let uri = gio::File::for_path(path).uri();
    let light = scope == "both" || scope == "light";
    let dark = scope == "both" || scope == "dark";
    anyhow::ensure!(light || dark, "Geçersiz duvar kâğıdı hedefi.");
    if (!light || settings.string("picture-uri") == uri)
        && (!dark || settings.string("picture-uri-dark") == uri)
    {
        return Ok(());
    }
    let before = serde_json::json!({
        "light": settings.string("picture-uri").as_str(),
        "dark": settings.string("picture-uri-dark").as_str(),
        "options": settings.string("picture-options").as_str()
    });
    let backup = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local/state"))
        .join("duvar/previous-wallpaper.json");
    crate::store::atomic(&backup, serde_json::to_string(&before)?.as_bytes())?;
    settings.delay();
    if light {
        settings.set_string("picture-uri", uri.as_str())?
    }
    if dark {
        settings.set_string("picture-uri-dark", uri.as_str())?
    }
    settings.apply();
    Ok(())
}

fn wallpaper_applied(scope: &str) -> &'static str {
    match scope {
        "light" => "Açık temanın duvar kâğıdı ayarlandı.",
        "dark" => "Koyu temanın duvar kâğıdı ayarlandı.",
        _ => "İki temanın duvar kâğıdı ayarlandı.",
    }
}

fn undo_wallpaper() -> anyhow::Result<()> {
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local/state"));
    let path = root.join("duvar/previous-wallpaper.json");
    if !path.exists() {
        anyhow::bail!("Henüz geri alınacak bir duvar kâğıdı yok.")
    }
    if std::fs::metadata(&path)?.len() > 8192 {
        anyhow::bail!("Önceki ayar kaydı geçersiz")
    }
    let before: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let settings = gio::Settings::new("org.gnome.desktop.background");
    settings.delay();
    for (field, key) in [
        ("light", "picture-uri"),
        ("dark", "picture-uri-dark"),
        ("options", "picture-options"),
    ] {
        let value = before[field]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Önceki ayar kaydı eksik"))?;
        settings.set_string(key, value)?;
    }
    settings.apply();
    Ok(())
}

fn weak(state: &Rc<State>) -> Weak<State> {
    Rc::downgrade(state)
}

fn preview(
    state: &Rc<State>,
    item: Wallpaper,
    paintable: Option<gdk::Texture>,
    nav_source: Option<(gio::ListStore, u32)>,
) {
    let Some(window) = state.window.upgrade() else {
        return;
    };
    let source_item = item.clone();
    let item = state.library.borrow().localize(&item);
    let token = crate::queue::next_id();
    let dialog = adw::Dialog::builder()
        .title(&item.title)
        .content_width(1140)
        .content_height(800)
        .build();
    let toasts = adw::ToastOverlay::new();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new(&item.title, &item.provider);
    header.set_title_widget(Some(&title));

    let zoom = gtk::ToggleButton::with_label("%100");
    zoom.set_sensitive(false);
    zoom.set_tooltip_text(Some("Özgün piksel boyutu; kapalıyken ekrana sığdır"));
    zoom.update_property(&[gtk::accessible::Property::Label(
        "Özgün piksel boyutunda görüntüle",
    )]);

    // Right-hand collapsible inspector drawer (OverlaySplitView) keeps 100% of height for the image!
    let split = adw::OverlaySplitView::builder()
        .sidebar_position(gtk::PackType::End)
        .show_sidebar(false)
        .min_sidebar_width(280.0)
        .max_sidebar_width(360.0)
        .build();
    let info_toggle = gtk::ToggleButton::builder()
        .icon_name("dialog-information-symbolic")
        .tooltip_text("Görsel bilgileri ve etiketler (I)")
        .build();
    info_toggle.update_property(&[gtk::accessible::Property::Label(
        "Görsel bilgileri ve etiketler",
    )]);
    split
        .bind_property("show-sidebar", &info_toggle, "active")
        .bidirectional()
        .sync_create()
        .build();
    header.pack_end(&info_toggle);
    toolbar.add_top_bar(&header);

    // Full-bleed visual canvas with instant progressive thumbnail display
    let picture = gtk::Picture::new();
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::ScaleDown);
    picture.set_vexpand(true);
    picture.set_hexpand(true);
    picture.update_property(&[gtk::accessible::Property::Label(&item.title)]);
    picture.update_state(&[gtk::accessible::State::Busy(true)]);
    if let Some(thumb) = paintable.or_else(|| state.cached_texture(&item.id)) {
        picture.set_paintable(Some(&thumb));
    }

    let canvas = gtk::Overlay::new();
    canvas.add_css_class("duvar-lightbox-canvas");
    let background = gtk::DrawingArea::new();
    background.set_hexpand(true);
    background.set_vexpand(true);
    if state.config.borrow().behavior.checkerboard {
        background.set_draw_func(|area, cr, w, h| {
            let c = area.color();
            for y in (0..h).step_by(12) {
                for x in (0..w).step_by(12) {
                    cr.set_source_rgba(
                        c.red() as f64,
                        c.green() as f64,
                        c.blue() as f64,
                        if (x / 12 + y / 12) % 2 == 0 {
                            0.05
                        } else {
                            0.14
                        },
                    );
                    cr.rectangle(x as f64, y as f64, 12.0, 12.0);
                    let _ = cr.fill();
                }
            }
        });
    }
    canvas.set_child(Some(&background));
    canvas.add_overlay(&picture);
    canvas.set_measure_overlay(&picture, true);

    let scroll = gtk::ScrolledWindow::builder()
        .child(&canvas)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .hexpand(true)
        .min_content_height(360)
        .build();

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&scroll));
    overlay.set_vexpand(true);

    // Subtle bottom-left loading pill (never obscures the instant thumbnail preview!)
    let loading_pill = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    loading_pill.add_css_class("osd");
    loading_pill.add_css_class("duvar-lightbox-pill");
    loading_pill.set_halign(gtk::Align::Start);
    loading_pill.set_valign(gtk::Align::End);
    loading_pill.set_margin_start(14);
    loading_pill.set_margin_bottom(14);
    let spinner = gtk::Spinner::new();
    spinner.set_size_request(16, 16);
    spinner.start();
    loading_pill.append(&spinner);
    let loading_lbl = gtk::Label::new(Some("Özgün çözünürlük yükleniyor…"));
    loading_lbl.add_css_class("caption");
    loading_pill.append(&loading_lbl);
    overlay.add_overlay(&loading_pill);

    let ring = transfer_ui::Ring::new();
    ring.widget.set_halign(gtk::Align::End);
    ring.widget.set_valign(gtk::Align::End);
    ring.widget.set_margin_end(14);
    ring.widget.set_margin_bottom(14);
    ring.widget.add_css_class("osd");
    overlay.add_overlay(&ring.widget);

    // Previous / Next floating edge navigation buttons
    let on_navigate: Option<Rc<dyn Fn(i32)>> = nav_source.as_ref().map(|(model, pos)| {
        let m = model.clone();
        let current_pos = *pos;
        let w = weak(state);
        let dlg = dialog.downgrade();
        Rc::new(move |delta: i32| {
            let total = m.n_items();
            if total <= 1 {
                return;
            }
            let next = if delta < 0 {
                if current_pos == 0 {
                    return;
                }
                current_pos - 1
            } else if current_pos + 1 < total {
                current_pos + 1
            } else {
                return;
            };
            if let (Some(s), Some(obj)) = (
                w.upgrade(),
                m.item(next).and_downcast::<glib::BoxedAnyObject>(),
            ) {
                let (next_wp, next_tex) = {
                    let e = obj.borrow::<Entry>();
                    (e.wallpaper.clone(), e.texture.borrow().clone())
                };
                if let Some(d) = dlg.upgrade() {
                    d.close();
                }
                preview(&s, next_wp, next_tex, Some((m.clone(), next)));
            }
        }) as Rc<dyn Fn(i32)>
    });

    if let (Some((model, pos)), Some(nav)) = (&nav_source, &on_navigate) {
        if *pos > 0 {
            let prev_btn = gtk::Button::from_icon_name("go-previous-symbolic");
            prev_btn.set_tooltip_text(Some("Önceki görsel (←)"));
            prev_btn.add_css_class("circular");
            prev_btn.add_css_class("osd");
            prev_btn.add_css_class("duvar-nav-btn");
            prev_btn.set_halign(gtk::Align::Start);
            prev_btn.set_valign(gtk::Align::Center);
            prev_btn.set_margin_start(12);
            let n = nav.clone();
            prev_btn.connect_clicked(move |_| n(-1));
            overlay.add_overlay(&prev_btn);
        }
        if *pos + 1 < model.n_items() {
            let next_btn = gtk::Button::from_icon_name("go-next-symbolic");
            next_btn.set_tooltip_text(Some("Sonraki görsel (→)"));
            next_btn.add_css_class("circular");
            next_btn.add_css_class("osd");
            next_btn.add_css_class("duvar-nav-btn");
            next_btn.set_halign(gtk::Align::End);
            next_btn.set_valign(gtk::Align::Center);
            next_btn.set_margin_end(12);
            let n = nav.clone();
            next_btn.connect_clicked(move |_| n(1));
            overlay.add_overlay(&next_btn);
        }
    }

    split.set_content(Some(&overlay));

    // Info & Tags inspector sidebar
    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    sidebar_box.set_margin_top(14);
    sidebar_box.set_margin_bottom(16);
    sidebar_box.set_margin_start(14);
    sidebar_box.set_margin_end(14);
    let info_hdr = gtk::Label::new(Some("Görsel bilgileri"));
    info_hdr.set_xalign(0.0);
    info_hdr.add_css_class("heading");
    sidebar_box.append(&info_hdr);
    let details = gtk::Label::new(Some(&state.describe(&source_item)));
    details.set_wrap(true);
    details.set_xalign(0.0);
    details.set_selectable(true);
    details.add_css_class("caption");
    sidebar_box.append(&details);
    sidebar_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    preferences::tag_details(state, &sidebar_box, &source_item);
    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .child(&sidebar_box)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    split.set_sidebar(Some(&sidebar_scroll));

    // Compact bottom action & status bar
    let bottom = gtk::Box::new(gtk::Orientation::Vertical, 6);
    bottom.add_css_class("duvar-lightbox-bottom");
    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let quality = gtk::Label::new(Some("Özgün dosya yükleniyor…"));
    quality.set_hexpand(true);
    quality.set_xalign(0.0);
    quality.set_ellipsize(gtk::pango::EllipsizeMode::End);
    quality.add_css_class("caption");
    quality.add_css_class("dim-label");
    status_row.append(&quality);
    let retry = gtk::Button::with_label("Yeniden yükle");
    retry.add_css_class("flat");
    retry.set_visible(false);
    status_row.append(&retry);
    let hint = gtk::Label::new(Some(
        "Tekerlek: yakınlaştır · Sürükle: kaydır · Sağ tık: menü",
    ));
    hint.add_css_class("caption");
    hint.add_css_class("dim-label");
    status_row.append(&hint);
    bottom.append(&status_row);

    let actions = transfer_ui::action_row();
    bottom.append(&actions);
    toolbar.add_bottom_bar(&bottom);

    let dimensions = Rc::new(Cell::new((0, 0)));
    {
        let size = dimensions.clone();
        let p = picture.clone();
        let sc = scroll.clone();
        let q = quality.clone();
        zoom.connect_toggled(move |button| {
            let (width, height) = size.get();
            if button.is_active() {
                p.set_size_request(width, height);
                p.set_can_shrink(false);
                sc.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
                sc.set_cursor_from_name(Some("grab"));
                q.set_text(&format!("Özgün dosya · {width} × {height} piksel · %100"));
            } else {
                p.set_size_request(-1, -1);
                p.set_can_shrink(true);
                sc.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Never);
                sc.set_cursor_from_name(None);
                q.set_text(&format!(
                    "Özgün dosya · {width} × {height} piksel · Ekrana sığdırılıyor"
                ));
            }
        });
    }

    state.preview_cancel.borrow().store(true, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));
    state.preview_cancel.replace(cancel.clone());
    state.preview_picture.replace(Some(PreviewTarget {
        retry: retry.downgrade(),
        signature: RefCell::new(String::new()),
        token,
        toasts: toasts.downgrade(),
        item: source_item.clone(),
        ring,
        actions: actions.downgrade(),
        details: details.downgrade(),
        id: item.id.clone(),
        picture: picture.downgrade(),
        loading_pill: loading_pill.downgrade(),
        spinner: spinner.downgrade(),
        label: quality.downgrade(),
        zoom: zoom.downgrade(),
        dimensions: dimensions.clone(),
    }));

    let w = weak(state);
    dialog.connect_closed(move |_| {
        cancel.store(true, Ordering::Relaxed);
        if let Some(s) = w.upgrade()
            && s.preview_picture
                .borrow()
                .as_ref()
                .is_some_and(|t| t.token == token)
        {
            s.preview_picture.borrow_mut().take();
        }
    });

    state.pool.push(Job::Preview(
        Request {
            number: token,
            cancel: state.preview_cancel.borrow().clone(),
        },
        item.clone(),
    ));
    state.ensure_poll();

    let w = weak(state);
    let it = item.clone();
    retry.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            if let Some(t) = s
                .preview_picture
                .borrow()
                .as_ref()
                .filter(|t| t.token == token)
            {
                if let Some(label) = t.label.upgrade() {
                    label.set_text("Özgün dosya yükleniyor…");
                }
                if let Some(pill) = t.loading_pill.upgrade() {
                    pill.set_visible(true);
                }
                if let Some(spinner) = t.spinner.upgrade() {
                    spinner.start();
                }
            }
            s.pool.push(Job::Preview(
                Request {
                    number: token,
                    cancel: s.preview_cancel.borrow().clone(),
                },
                it.clone(),
            ));
            s.ensure_poll();
        }
    });

    transfer_ui::fill_actions(state, &actions, &source_item, false);
    state.refresh_cards(Some(&item.id), false);
    preferences::preview_zoom(
        state,
        &dialog,
        &header,
        &picture,
        &scroll,
        &zoom,
        dimensions,
        &quality,
        &split,
        source_item,
        on_navigate,
    );

    toolbar.set_content(Some(&split));
    toasts.set_child(Some(&toolbar));
    dialog.set_child(Some(&toasts));
    dialog.present(Some(&window));
}

#[allow(deprecated, reason = "GNOME 51 PreferencesWindow host")]
fn switch(
    state: &Rc<State>,
    group: &adw::PreferencesGroup,
    title: &str,
    subtitle: &str,
    get: fn(&Config) -> bool,
    set: fn(&mut Config, bool),
) {
    let row = adw::SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .active(get(&state.config.borrow()))
        .build();
    let w = weak(state);
    row.connect_active_notify(move |r| {
        if let Some(s) = w.upgrade() {
            set(&mut s.config.borrow_mut(), r.is_active());
            s.changed();
            if let Some(fp) = s.filters_page.upgrade() {
                // Only rebuild filter compat notes if user is not currently toggling inside Filters
                if s.window
                    .upgrade()
                    .and_then(|win| win.visible_page())
                    .is_none_or(|p| p != fp)
                {
                    preferences::rebuild_filters(&s, &fp);
                }
            }
        }
    });
    group.add(&row)
}

#[allow(
    deprecated,
    reason = "Must populate the PreferencesWindow provided by GNOME 51"
)]
pub fn populate(window: &adw::PreferencesWindow, directory: &Path) {
    window.set_title(Some("Duvar"));
    window.set_icon_name(Some("io.github.mo4tal.Duvar"));
    let css_provider = if let Some(display) = gdk::Display::default() {
        gtk::IconTheme::for_display(&display).add_search_path(directory.join("icons"));
        let provider = gtk::CssProvider::new();
        provider.load_from_string(GALLERY_CSS);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        Some(provider)
    } else {
        None
    };
    window.set_default_size(1100, 820);
    window.set_search_enabled(false);
    let source = gio::SettingsSchemaSource::from_directory(
        directory.join("schemas"),
        gio::SettingsSchemaSource::default().as_ref(),
        false,
    )
    .expect("Duvar settings schema");
    let schema = source
        .lookup("org.gnome.shell.extensions.duvar", false)
        .expect("Duvar schema");
    let settings = gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None);
    let mut cfg: Config =
        serde_json::from_str(&settings.string("configuration")).unwrap_or_default();
    let config_error = match crate::store::load_config() {
        Ok(Some(saved)) => {
            cfg = saved;
            None
        }
        Ok(None) => None,
        Err(e) => Some(format!(
            "Ayar dosyası okunamadı: {e}; GNOME kaydı kullanılıyor"
        )),
    };
    cfg.validate();
    cfg = cfg.persistent();
    let (queue, queue_error) = match crate::store::load_queue() {
        Ok(q) => (q, None),
        Err(e) => (vec![], Some(format!("Kuyruk okunamadı: {e}"))),
    };
    let (pool, receiver) = Pool::new();
    let status = gtk::Label::new(None);
    status.add_css_class("dim-label");
    let spinner = gtk::Spinner::new();
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Duvar kâğıdı ara…")
        .text(&cfg.query)
        .hexpand(true)
        .build();
    search.set_search_delay(320);

    let compat_bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    compat_bar.add_css_class("duvar-compat-note");
    compat_bar.set_visible(false);
    compat_bar.append(&gtk::Image::from_icon_name("dialog-information-symbolic"));
    let compat_label = gtk::Label::new(None);
    compat_label.set_hexpand(true);
    compat_label.set_xalign(0.0);
    compat_label.set_wrap(true);
    compat_label.add_css_class("caption");
    compat_label.add_css_class("dim-label");
    compat_bar.append(&compat_label);

    let state = Rc::new(State {
        window: window.downgrade(),
        settings,
        config: RefCell::new(cfg),
        pool,
        receiver,
        request: RefCell::new(Request {
            number: 0,
            cancel: Arc::new(AtomicBool::new(false)),
        }),
        poll: RefCell::new(None),
        model: gio::ListStore::new::<glib::BoxedAnyObject>(),
        gallery_grid: glib::WeakRef::new(),
        downloaded_grid: glib::WeakRef::new(),
        status,
        spinner,
        empty: adw::StatusPage::new(),
        explore_scroll: glib::WeakRef::new(),
        search,
        compat_bar,
        compat_label,
        cursors: RefCell::new(BTreeMap::new()),
        seen: RefCell::new(HashSet::new()),
        thumbnail_pending: RefCell::new(HashSet::new()),
        texture_cache: RefCell::new(BTreeMap::new()),
        texture_lru: RefCell::new(VecDeque::new()),
        action_cancel: Arc::new(AtomicBool::new(false)),
        queue: RefCell::new(queue),
        latest_apply: Cell::new(crate::store::queue_metadata().0),
        queue_cancels: RefCell::new(BTreeMap::new()),
        queue_label: gtk::Label::new(None),
        queue_list: gtk::ListBox::new(),
        downloads_model: gio::ListStore::new::<glib::BoxedAnyObject>(),
        retry: gtk::Button::with_label("Tekrar dene"),
        more: gtk::Button::from_icon_name("list-add-symbolic"),
        key_entry: glib::WeakRef::new(),
        key_edited: Cell::new(false),
        key_save: glib::WeakRef::new(),
        favorites: Cell::new(false),
        api_key: RefCell::new(String::new()),
        key_looked: Cell::new(false),
        key_registered: Cell::new(false),
        key_status: glib::WeakRef::new(),
        closed: Cell::new(false),
        errors: RefCell::new(Vec::new()),
        preview_picture: RefCell::new(None),
        topic_summary: glib::WeakRef::new(),
        sources_summary: glib::WeakRef::new(),
        filters_page: glib::WeakRef::new(),
        topic_bar: transfer_ui::action_box(),
        preview_cancel: RefCell::new(Arc::new(AtomicBool::new(false))),
        library: RefCell::new(crate::library::Index::default()),
        facts: RefCell::new(BTreeMap::new()),
        cards: RefCell::new(Vec::new()),
        queue_rows: RefCell::new(BTreeMap::new()),
        queue_paused: Cell::new(crate::store::queue_metadata().1),
        library_summary: gtk::Label::new(None),
        last_saved: RefCell::new(String::new()),
        last_trash: RefCell::new(None),
        save_source: RefCell::new(None),
        css_provider: RefCell::new(css_provider),
    });

    let wallpaper_actions = gio::SimpleActionGroup::new();
    let apply_file =
        gio::SimpleAction::new("apply-file", Some(glib::VariantTy::new("(ss)").unwrap()));
    let w = weak(&state);
    apply_file.connect_activate(move |_, parameter| {
        if let (Some(s), Some((scope, path))) = (
            w.upgrade(),
            parameter.and_then(|p| p.get::<(String, String)>()),
        ) && let Err(e) = s.apply_wallpaper(Path::new(&path), &scope)
        {
            s.toast(&format!("Duvar kâğıdı uygulanamadı: {e}"));
        }
    });
    wallpaper_actions.add_action(&apply_file);
    window.insert_action_group("wallpaper", Some(&wallpaper_actions));

    let explore = adw::PreferencesPage::builder()
        .title("Keşfet")
        .icon_name("preferences-desktop-wallpaper-symbolic")
        .build();
    let downloaded = adw::PreferencesPage::builder()
        .title("İndirilenler")
        .icon_name("folder-download-symbolic")
        .build();
    let filters = adw::PreferencesPage::builder()
        .title("Filtreler")
        .icon_name("duvar-filter-symbolic")
        .build();
    let settings_page = adw::PreferencesPage::builder()
        .title("Ayarlar")
        .icon_name("preferences-system-symbolic")
        .build();
    state.filters_page.set(Some(&filters));
    for page in [&explore, &downloaded, &filters, &settings_page] {
        window.add(page);
    }

    // ── KEŞFET: Single sleek minimalist command bar (fits 480px narrow through 4K wide) ──
    let group = adw::PreferencesGroup::new();
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.add_css_class("duvar-toolbar-row");
    bar.append(&state.search);

    let sort = gtk::DropDown::from_strings(&[
        "Önerilen",
        "En yeni",
        "Çözünürlük",
        "Yatay oran",
        "Rastgele",
    ]);
    sort.set_selected(
        ["source", "newest", "resolution", "wide", "random"]
            .iter()
            .position(|x| *x == state.config.borrow().gallery_order)
            .unwrap_or(0) as u32,
    );
    sort.update_property(&[gtk::accessible::Property::Label("Sıralama ölçütü")]);
    sort.set_tooltip_text(Some("Sıralama ölçütü"));
    let w = weak(&state);
    sort.connect_selected_notify(move |row| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().gallery_order =
                ["source", "newest", "resolution", "wide", "random"]
                    [row.selected().min(4) as usize]
                    .into();
            s.save();
            s.sync_compat_hints();
            s.sort_gallery();
        }
    });
    bar.append(&sort);

    let topics_button = gtk::Button::with_label("Kategoriler");
    topics_button.set_valign(gtk::Align::Center);
    let w = weak(&state);
    topics_button.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            topics::show(&s);
        }
    });
    bar.append(&topics_button);

    let favorites = gtk::ToggleButton::builder()
        .icon_name("starred-symbolic")
        .tooltip_text("Favoriler · en fazla 128 kayıt")
        .valign(gtk::Align::Center)
        .build();
    favorites.add_css_class("flat");
    favorites.add_css_class("duvar-icon-btn");
    favorites.update_property(&[gtk::accessible::Property::Label(
        "Favorileri göster (en fazla 128)",
    )]);
    let w = weak(&state);
    favorites.connect_toggled(move |b| {
        if b.is_active() {
            b.add_css_class("duvar-fav-active");
        } else {
            b.remove_css_class("duvar-fav-active");
        }
        if let Some(s) = w.upgrade() {
            s.favorites.set(b.is_active());
            s.load();
        }
    });
    bar.append(&favorites);

    let tools_menu = preferences::explore_menu(&state, &filters);
    bar.append(&tools_menu);
    group.add(&bar);

    state.topic_bar.set_halign(gtk::Align::Start);
    group.add(&state.topic_bar);
    group.add(&state.compat_bar);

    let grid = gallery(&state, &state.model, false);
    state.gallery_grid.set(Some(&grid));
    let scroll = gtk::ScrolledWindow::builder()
        .child(&grid)
        .min_content_height(480)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();
    let w = weak(&state);
    scroll.connect_edge_reached(move |_, edge| {
        if edge == gtk::PositionType::Bottom
            && let Some(s) = w.upgrade()
        {
            s.load_more(false);
        }
    });
    let w = weak(&state);
    scroll.vadjustment().connect_value_changed(move |a| {
        if a.value() > 0.0
            && a.upper() - a.page_size() - a.value() < 320.0
            && let Some(s) = w.upgrade()
        {
            s.load_more(false);
        }
    });
    state.explore_scroll.set(Some(&scroll));

    state.empty.set_icon_name(Some("image-x-generic-symbolic"));
    let empty_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    empty_actions.set_halign(gtk::Align::Center);
    let manage = gtk::Button::with_label("Kaynakları yönet");
    manage.add_css_class("pill");
    manage.add_css_class("suggested-action");
    let w = weak(&state);
    manage.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            preferences::sources(&s);
        }
    });
    empty_actions.append(&manage);
    let review = gtk::Button::with_label("Filtreleri gözden geçir");
    review.add_css_class("pill");
    let page = filters.downgrade();
    let host = window.downgrade();
    review.connect_clicked(move |_| {
        if let (Some(page), Some(host)) = (page.upgrade(), host.upgrade()) {
            host.set_visible_page(&page);
        }
    });
    empty_actions.append(&review);
    state.empty.set_child(Some(&empty_actions));
    group.add(&state.empty);
    group.add(&scroll);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_margin_top(6);
    state.spinner.set_valign(gtk::Align::Center);
    footer.append(&state.spinner);
    state.status.set_hexpand(true);
    state.status.set_xalign(0.0);
    state.status.set_wrap(true);
    state.status.add_css_class("caption");
    footer.append(&state.status);
    state.retry.set_visible(false);
    state.retry.set_valign(gtk::Align::Center);
    state.retry.add_css_class("flat");
    let w = weak(&state);
    state.retry.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            s.load_more(true);
        }
    });
    footer.append(&state.retry);
    state.more.set_valign(gtk::Align::Center);
    state.more.add_css_class("flat");
    state.more.set_tooltip_text(Some("Daha fazla sonuç yükle"));
    let w = weak(&state);
    state.more.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            s.load_more(false);
        }
    });
    footer.append(&state.more);
    group.add(&footer);
    explore.add(&group);

    // ── İNDİRİLENLER: Proper Adwaita header suffix + crisp grid + collapsible queue ──
    let collection = adw::PreferencesGroup::builder()
        .title("İndirilen görseller")
        .description(
            "Cihazındaki yerel duvar kâğıtları. Etkin içerik izinleri burada da geçerlidir.",
        )
        .build();
    let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    state.library_summary.set_valign(gtk::Align::Center);
    state.library_summary.add_css_class("caption");
    state.library_summary.add_css_class("dim-label");
    header_box.append(&state.library_summary);
    let reload = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("İndirilenleri yenile")
        .valign(gtk::Align::Center)
        .build();
    reload.add_css_class("flat");
    let w = weak(&state);
    reload.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            s.refresh_downloads();
        }
    });
    header_box.append(&reload);
    collection.set_header_suffix(Some(&header_box));

    preferences::library_controls(&state, &collection);
    let downloads_grid = gallery(&state, &state.downloads_model, true);
    state.downloaded_grid.set(Some(&downloads_grid));
    collection.add(
        &gtk::ScrolledWindow::builder()
            .child(&downloads_grid)
            .min_content_height(420)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build(),
    );
    downloaded.add(&collection);

    let queue_group = adw::PreferencesGroup::new();
    let queue = adw::ExpanderRow::builder()
        .title("İndirme kuyruğu")
        .subtitle("Bekleyen aktarımlar pencereyi yeniden açtığında sürdürülür.")
        .build();
    queue.add_suffix(&state.queue_label);
    state.queue_label.add_css_class("caption");
    state.queue_label.add_css_class("dim-label");
    state
        .queue_list
        .set_selection_mode(gtk::SelectionMode::None);
    preferences::queue_controls(&state, &queue);
    queue.add_row(&state.queue_list);
    queue_group.add(&queue);
    downloaded.add(&queue_group);

    preferences::filters(&state, &filters);
    preferences::settings(&state, &settings_page);

    fn widen(widget: &impl IsA<gtk::Widget>) {
        let widget = widget.as_ref();
        if let Some(clamp) = widget.downcast_ref::<adw::Clamp>() {
            clamp.set_maximum_size(1600);
            clamp.set_tightening_threshold(1200);
        }
        let mut child = widget.first_child();
        while let Some(item) = child {
            widen(&item);
            child = item.next_sibling();
        }
    }
    widen(&explore);
    widen(&downloaded);

    let w = weak(&state);
    state.search.connect_search_changed(move |search| {
        if let Some(s) = w.upgrade() {
            if s.config.borrow().query == search.text().as_str() {
                return;
            }
            s.config.borrow_mut().query = search.text().chars().take(256).collect();
            s.config.borrow_mut().behavior.remember(&search.text());
            s.changed();
        }
    });
    preferences::keyboard(
        &state,
        window,
        [
            explore.clone(),
            downloaded.clone(),
            filters.clone(),
            settings_page.clone(),
        ],
    );

    let owned = state.clone();
    window.connect_close_request(move |_| {
        owned.closed.set(true);
        if let Some(id) = owned.save_source.borrow_mut().take() {
            id.remove();
        }
        owned.action_cancel.store(true, Ordering::Relaxed);
        for c in owned.queue_cancels.borrow().values() {
            c.store(true, Ordering::Relaxed);
        }
        if let Err(e) = crate::store::save_queue_state(
            &owned.queue.borrow(),
            owned.latest_apply.get(),
            owned.queue_paused.get(),
        ) {
            eprintln!("Duvar: kuyruk kaydedilemedi: {e}");
        }
        owned.preview_cancel.borrow().store(true, Ordering::Relaxed);
        owned.current().cancel.store(true, Ordering::Relaxed);
        owned.pool.close();
        if let Some(id) = owned.poll.borrow_mut().take() {
            id.remove();
        }
        if let Some(provider) = owned.css_provider.borrow_mut().take()
            && let Some(display) = gdk::Display::default()
        {
            gtk::style_context_remove_provider_for_display(&display, &provider);
        }
        if let Err(e) = crate::store::save_config(&owned.config.borrow()) {
            eprintln!("Duvar: ayarlar kaydedilemedi: {e}");
        }
        gio::Settings::sync();
        glib::Propagation::Proceed
    });

    let recovered = crate::diagnostics::records()
        .iter()
        .filter(|r| r.component == "queue" && r.event == "recovery")
        .count();
    if recovered > 0 {
        state.toast(&format!(
            "Kuyruk kurtarıldı: {recovered} kayıt düzeltmesi; sağlam işler korundu."
        ));
    }
    state.load();
    state.refresh_downloads();
    let pending = state
        .queue
        .borrow()
        .iter()
        .filter(|r| matches!(r.status.as_str(), "queued" | "active"))
        .map(|r| r.id)
        .collect::<Vec<_>>();
    for id in pending {
        state.start_download(id);
    }
    if queue_error.is_none() {
        state.queue_changed();
    }
    if let Some(e) = queue_error {
        state.toast(&e);
    }
    if let Some(e) = config_error {
        state.toast(&e);
    }
}

#[cfg(test)]
mod paging_tests {
    use super::Cursor;
    #[test]
    fn failed_source_retries_same_page_without_affecting_other_sources() {
        let mut a = Cursor::default();
        let mut b = Cursor::default();
        assert_eq!(a.begin(false), Some(1));
        assert_eq!(a.begin(false), None);
        assert_eq!(b.begin(false), Some(1));
        a.finish(true, true);
        b.finish(true, false);
        assert_eq!(a.begin(false), None);
        assert_eq!(b.begin(false), Some(2));
        assert_eq!(a.begin(true), Some(1));
        a.finish(false, false);
        assert_eq!(a.begin(false), None);
    }
}

fn gallery(state: &Rc<State>, model: &gio::ListStore, downloaded: bool) -> gtk::GridView {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let row = object.downcast_ref::<gtk::ListItem>().unwrap();
        let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
        card.add_css_class("duvar-card");
        card.set_width_request(200);

        // Visual hero: 16:9 thumbnail + clean OSD overlays (resolution badge top-left, fav/dl top-right, progress bottom-right)
        let frame = crate::thumbnail::Thumbnail::new();
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&frame));

        let badge = gtk::Label::new(None);
        badge.add_css_class("osd");
        badge.add_css_class("duvar-card-badge");
        badge.set_halign(gtk::Align::Start);
        badge.set_valign(gtk::Align::Start);
        badge.set_margin_start(6);
        badge.set_margin_top(6);
        overlay.add_overlay(&badge);

        let quick_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        quick_box.set_halign(gtk::Align::End);
        quick_box.set_valign(gtk::Align::Start);
        quick_box.set_margin_end(6);
        quick_box.set_margin_top(6);
        overlay.add_overlay(&quick_box);

        card.append(&overlay);

        // Fixed 2-line metadata block
        let meta = gtk::Box::new(gtk::Orientation::Vertical, 1);
        meta.set_margin_start(3);
        meta.set_margin_end(3);
        let title = gtk::Label::new(None);
        title.set_xalign(0.0);
        title.set_lines(1);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.add_css_class("duvar-card-title");
        meta.append(&title);

        let subtitle = gtk::Label::new(None);
        subtitle.set_xalign(0.0);
        subtitle.set_lines(1);
        subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
        subtitle.add_css_class("dim-label");
        subtitle.add_css_class("duvar-card-sub");
        meta.append(&subtitle);
        card.append(&meta);

        // Fixed single-row linked apply pair (exact uniform height across ALL states!)
        let apply_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        apply_box.set_homogeneous(true);
        apply_box.add_css_class("linked");
        card.append(&apply_box);

        row.set_child(Some(&card));
    });

    let w_bind = weak(state);
    factory.connect_bind(move |_, object| {
        let row = object.downcast_ref::<gtk::ListItem>().unwrap();
        let object = row.item().and_downcast::<glib::BoxedAnyObject>().unwrap();
        let entry = object.borrow::<Entry>();
        let card = row.child().and_downcast::<gtk::Box>().unwrap();
        let overlay = card.first_child().and_downcast::<gtk::Overlay>().unwrap();
        let frame = overlay
            .child()
            .and_downcast::<crate::thumbnail::Thumbnail>()
            .unwrap();
        let mut badge_opt = None;
        let mut quick_opt = None;
        let mut child = overlay.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            if let Some(label) = c.downcast_ref::<gtk::Label>() {
                badge_opt = Some(label.clone());
            } else if let Some(b) = c.downcast_ref::<gtk::Box>() {
                quick_opt = Some(b.clone());
            } else if c.is::<gtk::DrawingArea>() {
                overlay.remove_overlay(&c);
            }
        }
        let badge = badge_opt.expect("card badge");
        let quick_box = quick_opt.expect("card quick box");
        let picture = frame.picture();

        let existing_tex = entry.texture.borrow().clone().or_else(|| {
            w_bind
                .upgrade()
                .and_then(|s| s.cached_texture(&entry.wallpaper.id))
        });
        if let Some(tex) = &existing_tex {
            entry.texture.replace(Some(tex.clone()));
            picture.set_paintable(Some(tex));
        } else {
            picture.set_paintable(None::<&gdk::Texture>);
            if let Some(s) = w_bind.upgrade() {
                s.request_thumbnail(entry.wallpaper.clone(), downloaded);
            }
        }

        let meta = overlay.next_sibling().and_downcast::<gtk::Box>().unwrap();
        let title = meta.first_child().and_downcast::<gtk::Label>().unwrap();
        title.set_text(&entry.wallpaper.title);
        title.set_tooltip_text(Some(&entry.wallpaper.title));
        let subtitle = title.next_sibling().and_downcast::<gtk::Label>().unwrap();
        let apply_box = meta.next_sibling().and_downcast::<gtk::Box>().unwrap();

        if let Some(s) = w_bind.upgrade() {
            let mut cards = s.cards.borrow_mut();
            cards.retain(|t| t.root.upgrade().is_some_and(|r| r != card));
            let ring = transfer_ui::Ring::new();
            ring.widget.set_halign(gtk::Align::End);
            ring.widget.set_valign(gtk::Align::End);
            ring.widget.set_margin_end(6);
            ring.widget.set_margin_bottom(6);
            ring.widget.add_css_class("osd");
            overlay.add_overlay(&ring.widget);
            cards.push(transfer_ui::CardTarget {
                item: entry.wallpaper.clone(),
                root: card.downgrade(),
                picture: picture.downgrade(),
                badge: badge.downgrade(),
                subtitle: subtitle.downgrade(),
                quick_box: quick_box.downgrade(),
                apply_box: apply_box.downgrade(),
                ring,
                signature: RefCell::new(String::new()),
            });
            drop(cards);
            s.refresh_cards(Some(&entry.wallpaper.id), true);
            card.update_property(&[gtk::accessible::Property::Label(&format!(
                "{} · {}",
                entry.wallpaper.title,
                transfer_ui::information(&entry.wallpaper, &s.facts_for(&entry.wallpaper))
            ))]);
        }
    });

    // Cancel stale queued off-screen thumbnails & detach bound cards immediately on unbind!
    let w_unbind = weak(state);
    factory.connect_unbind(move |_, object| {
        let Some(s) = w_unbind.upgrade() else {
            return;
        };
        let row = object.downcast_ref::<gtk::ListItem>().unwrap();
        if let Some(card) = row.child().and_downcast::<gtk::Box>() {
            s.cards
                .borrow_mut()
                .retain(|t| t.root.upgrade().is_some_and(|r| r != card));
        }
        if let Some(obj) = row.item().and_downcast::<glib::BoxedAnyObject>() {
            let id = obj.borrow::<Entry>().wallpaper.id.clone();
            let req_num = if downloaded {
                u64::MAX
            } else {
                s.current().number
            };
            let pending_key = format!("{req_num}:{id}");
            if s.thumbnail_pending.borrow_mut().remove(&pending_key) {
                s.pool.drop_thumbnail(&id);
            }
        }
    });

    let selection = gtk::NoSelection::new(Some(model.clone()));
    let grid = gtk::GridView::new(Some(selection), Some(factory));
    grid.add_css_class("duvar-gallery-grid");
    grid.set_min_columns(1);
    grid.set_max_columns(5);
    grid.set_single_click_activate(true);
    let w = weak(state);
    let model = model.clone();
    grid.connect_activate(move |_, position| {
        if let Some(s) = w.upgrade()
            && let Some(object) = model.item(position).and_downcast::<glib::BoxedAnyObject>()
        {
            let (wp, tex) = {
                let entry = object.borrow::<Entry>();
                (entry.wallpaper.clone(), entry.texture.borrow().clone())
            };
            preview(&s, wp, tex, Some((model.clone(), position)));
        }
    });
    grid
}

fn compare(a: &glib::Object, b: &glib::Object, order: &str, seed: u64) -> std::cmp::Ordering {
    let a = a
        .downcast_ref::<glib::BoxedAnyObject>()
        .unwrap()
        .borrow::<Entry>();
    let b = b
        .downcast_ref::<glib::BoxedAnyObject>()
        .unwrap()
        .borrow::<Entry>();
    match order {
        "newest" => {
            // Partition items with verified dates ahead of dateless sources cleanly.
            let a_has = a.wallpaper.created_at > 0;
            let b_has = b.wallpaper.created_at > 0;
            b_has
                .cmp(&a_has)
                .then_with(|| b.wallpaper.created_at.cmp(&a.wallpaper.created_at))
        }
        "resolution" => {
            let a_px = a.wallpaper.width as u64 * a.wallpaper.height as u64;
            let b_px = b.wallpaper.width as u64 * b.wallpaper.height as u64;
            b_px.cmp(&a_px)
        }
        "wide" => {
            let a_known = a.wallpaper.width > 0 && a.wallpaper.height > 0;
            let b_known = b.wallpaper.width > 0 && b.wallpaper.height > 0;
            b_known.cmp(&a_known).then_with(|| {
                (b.wallpaper.width as f64 / b.wallpaper.height.max(1) as f64)
                    .total_cmp(&(a.wallpaper.width as f64 / a.wallpaper.height.max(1) as f64))
            })
        }
        "random" => crate::store::key(&format!("{seed}:{}", a.wallpaper.id))
            .cmp(&crate::store::key(&format!("{seed}:{}", b.wallpaper.id))),
        _ => a.rank.cmp(&b.rank),
    }
    .then_with(|| a.rank.cmp(&b.rank))
}

#[cfg(test)]
mod sorting_tests {
    use super::*;
    fn entry(rank: u64, width: u32) -> glib::BoxedAnyObject {
        let wallpaper: Wallpaper = serde_json::from_value(serde_json::json!({
            "id": rank.to_string(),
            "provider": "fixture",
            "title": "Fixture",
            "thumbnail": "",
            "image": "",
            "landing": "",
            "rating": "safe",
            "tags": [],
            "width": width,
            "height": 100,
            "author": "",
            "license": ""
        }))
        .unwrap();
        glib::BoxedAnyObject::new(Entry {
            wallpaper,
            rank,
            texture: RefCell::new(None),
        })
    }
    #[test]
    fn source_order_can_be_restored_after_resolution_sort() {
        let model = gio::ListStore::new::<glib::BoxedAnyObject>();
        model.append(&entry(0, 100));
        model.append(&entry(1, 1000));
        model.sort(|a, b| compare(a, b, "resolution", 1));
        assert_eq!(
            model
                .item(0)
                .and_downcast::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Entry>()
                .rank,
            1
        );
        model.sort(|a, b| compare(a, b, "source", 1));
        assert_eq!(
            model
                .item(0)
                .and_downcast::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Entry>()
                .rank,
            0
        );
    }
}
