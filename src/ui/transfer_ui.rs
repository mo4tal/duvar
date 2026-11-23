use super::*;
use crate::runtime::{Facts, Phase, Progress};
use std::f64::consts::{FRAC_PI_2, TAU};

#[derive(Clone)]
pub(super) struct Ring {
    pub widget: gtk::DrawingArea,
    value: Rc<RefCell<Option<Progress>>>,
}
impl Ring {
    pub fn new() -> Self {
        let widget = gtk::DrawingArea::builder()
            .content_width(34)
            .content_height(34)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .can_target(false)
            .build();
        widget.set_accessible_role(gtk::AccessibleRole::ProgressBar);
        let value: Rc<RefCell<Option<Progress>>> = Rc::new(RefCell::new(None));
        let v = value.clone();
        widget.set_draw_func(move |area, cr, w, h| {
            let p = v.borrow();
            let Some(p) = p.as_ref() else {
                return;
            };
            let c = area.color();
            let (x, y) = (w as f64 / 2.0, h as f64 / 2.0);
            let r = x.min(y) - 3.0;
            cr.set_line_width(3.0);
            cr.set_source_rgba(c.red() as f64, c.green() as f64, c.blue() as f64, 0.22);
            cr.arc(x, y, r, 0.0, TAU);
            let _ = cr.stroke();
            cr.set_source_rgba(c.red() as f64, c.green() as f64, c.blue() as f64, 1.0);
            if let Some(f) = p.fraction() {
                cr.arc(x, y, r, -FRAC_PI_2, -FRAC_PI_2 + TAU * f);
                let _ = cr.stroke();
            } else {
                cr.set_dash(&[2.0, 3.0], 0.0);
                cr.arc(x, y, r, 0.0, TAU);
                let _ = cr.stroke();
                cr.set_dash(&[], 0.0);
            }
            cr.set_line_width(1.8);
            cr.move_to(x, y - 5.5);
            cr.line_to(x, y + 3.5);
            cr.move_to(x - 3.5, y);
            cr.line_to(x, y + 3.5);
            cr.line_to(x + 3.5, y);
            cr.move_to(x - 4.5, y + 6.5);
            cr.line_to(x + 4.5, y + 6.5);
            let _ = cr.stroke();
        });
        widget.set_visible(false);
        Self { widget, value }
    }
    pub fn set(&self, p: Option<Progress>) {
        self.widget
            .set_visible(p.as_ref().is_some_and(|p| p.phase.pending()));
        if let Some(p) = &p {
            self.widget.set_tooltip_text(Some(&p.text()));
            self.widget.update_property(&[
                gtk::accessible::Property::Label(&p.text()),
                gtk::accessible::Property::ValueMin(0.0),
                gtk::accessible::Property::ValueMax(100.0),
                gtk::accessible::Property::ValueText(&p.text()),
            ]);
            self.widget
                .reset_property(gtk::AccessibleProperty::ValueNow);
            if let Some(v) = p.fraction() {
                self.widget
                    .update_property(&[gtk::accessible::Property::ValueNow(v * 100.0)]);
            }
            self.widget
                .update_state(&[gtk::accessible::State::Busy(p.phase.pending())]);
        }
        self.value.replace(p);
        self.widget.queue_draw();
    }
}

pub(super) struct CardTarget {
    pub item: Wallpaper,
    pub root: glib::WeakRef<gtk::Box>,
    pub picture: glib::WeakRef<gtk::Picture>,
    pub badge: glib::WeakRef<gtk::Label>,
    pub subtitle: glib::WeakRef<gtk::Label>,
    pub quick_box: glib::WeakRef<gtk::Box>,
    pub apply_box: glib::WeakRef<gtk::Box>,
    pub ring: Ring,
    pub signature: RefCell<String>,
}

impl State {
    pub(super) fn phase(&self, item: &Wallpaper) -> Phase {
        if self
            .last_trash
            .borrow()
            .as_ref()
            .is_some_and(|t| t.record.item.id == item.id && !Path::new(&t.record.path).exists())
        {
            return Phase::Trashed;
        }
        let q = self.queue.borrow();
        if let Some(r) = q.iter().rev().find(|r| {
            r.item.id == item.id && crate::runtime::Phase::from_status(&r.status).pending()
        }) {
            return Phase::from_status(&r.status);
        }
        if self.library.borrow().resolve(item).is_some() {
            return Phase::Completed;
        }
        if self
            .library
            .borrow()
            .known(&item.id)
            .is_some_and(|e| !Path::new(&e.path).exists())
        {
            return Phase::Missing;
        }
        if item.local {
            return if url::Url::parse(&item.image)
                .ok()
                .and_then(|u| u.to_file_path().ok())
                .is_some_and(|p| p.is_file())
            {
                Phase::Completed
            } else {
                Phase::Missing
            };
        }
        q.iter()
            .rev()
            .find(|r| r.item.id == item.id)
            .map(|r| Phase::from_status(&r.status))
            .filter(|p| *p != Phase::Completed)
            .unwrap_or(Phase::Remote)
    }
    pub(super) fn progress_for(&self, item: &Wallpaper) -> Option<Progress> {
        self.progress_for_id(&item.id)
    }
    fn progress_for_id(&self, id: &str) -> Option<Progress> {
        self.queue
            .borrow()
            .iter()
            .rev()
            .find(|r| r.item.id == id && Phase::from_status(&r.status).pending())
            .map(|r| {
                r.progress.clone().unwrap_or(Progress {
                    job: r.id,
                    image: id.to_string(),
                    phase: Phase::from_status(&r.status),
                    received: 0,
                    total: None,
                    bytes_per_second: None,
                    eta_seconds: None,
                    at: crate::runtime::now(),
                })
            })
    }
    pub(super) fn describe(&self, item: &Wallpaper) -> String {
        let ownership = if self
            .library
            .borrow()
            .resolve(item)
            .is_some_and(crate::files::managed_name)
        {
            "Dosya yönetimi: Duvar indirmesi; çöp işleminde parmak izi tekrar doğrulanır"
        } else if item.local {
            "Dosya yönetimi: Kişisel dosya; Duvar bu dosyaya çöp işlemi sunmaz"
        } else {
            "Dosya yönetimi: Kaynak görsel; henüz yönetilen yerel kopya yok"
        };
        format!("{}\n{ownership}", information(item, &self.facts_for(item)))
    }
    pub(super) fn facts_for(&self, item: &Wallpaper) -> Facts {
        self.facts
            .borrow()
            .get(&item.id)
            .cloned()
            .unwrap_or_else(|| self.library.borrow().facts(item))
    }
    pub(super) fn refresh_cards(self: &Rc<Self>, id: Option<&str>, actions: bool) {
        let mut cards = self.cards.borrow_mut();
        cards.retain(|t| t.root.upgrade().is_some());
        for t in cards.iter().filter(|t| id.is_none_or(|id| id == t.item.id)) {
            let f = self.facts_for(&t.item);
            let phase = self.phase(&t.item);
            if let Some(badge) = t.badge.upgrade() {
                let text = if f.width > 0 && f.height > 0 {
                    format!("{} × {} · {}", f.width, f.height, t.item.rating.label())
                } else {
                    t.item.rating.label().to_string()
                };
                badge.set_text(&text);
            }
            if let Some(subtitle) = t.subtitle.upgrade() {
                let dim = f.dimensions();
                let phase_tag = match phase {
                    Phase::Completed if !t.item.local => " · İndirildi",
                    p if p.pending() => " · İndiriliyor",
                    Phase::Missing => " · Eksik",
                    Phase::Failed => " · Hata",
                    _ => "",
                };
                subtitle.set_text(&format!("{} · {}{}", t.item.provider, dim, phase_tag));
                subtitle.set_tooltip_text(Some(&format!(
                    "{} · {} · {}",
                    crate::runtime::types(&t.item).join(" · "),
                    f.orientation_label(),
                    phase.label()
                )));
            }
            t.ring.set(self.progress_for(&t.item));
            let signature = action_signature(self, &t.item);
            if actions
                && *t.signature.borrow() != signature
                && let (Some(quick), Some(apply_box)) =
                    (t.quick_box.upgrade(), t.apply_box.upgrade())
            {
                fill_card_actions(self, &quick, &apply_box, &t.item);
                t.signature.replace(signature);
            }
        }
        drop(cards);
        if let Some(t) = self
            .preview_picture
            .borrow()
            .as_ref()
            .filter(|t| id.is_none_or(|id| id == t.id))
        {
            t.ring.set(self.progress_for(&t.item));
            let signature = action_signature(self, &t.item);
            if actions
                && *t.signature.borrow() != signature
                && let Some(a) = t.actions.upgrade()
            {
                fill_actions(self, &a, &t.item, false);
                t.signature.replace(signature);
            }
            if let Some(label) = t.details.upgrade() {
                label.set_text(&self.describe(&t.item));
            }
        }
    }
    pub(super) fn update_progress(self: &Rc<Self>, p: Progress) {
        let id = p.image.clone();
        if let Some(r) = self
            .queue
            .borrow_mut()
            .iter_mut()
            .find(|r| r.id == p.job && Phase::from_status(&r.status).pending())
        {
            if matches!(
                r.status.as_str(),
                "cancel_requested" | "pause_requested" | "resume_requested"
            ) {
                return;
            }
            if !crate::runtime::Phase::from_status(&r.status).transition(
                if p.phase == Phase::Completed {
                    Phase::Saving
                } else {
                    p.phase
                },
            ) {
                crate::diagnostics::log(
                    Some(r.id),
                    "queue",
                    "invalid_transition",
                    &format!("{} -> {:?}", r.status, p.phase),
                    None,
                );
                return;
            }
            r.status = match p.phase {
                Phase::Completed => "saving".into(),
                _ => serde_json::to_value(p.phase)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into(),
            };
            r.progress = Some(p.clone());
        }
        if let Some(row) = self
            .queue_rows
            .borrow()
            .get(&p.job)
            .and_then(glib::WeakRef::upgrade)
        {
            row.set_subtitle(&p.text());
        }
        let progress = self.progress_for_id(&id);
        for target in self
            .cards
            .borrow()
            .iter()
            .filter(|t| t.item.id == id && t.root.upgrade().is_some())
        {
            target.ring.set(progress.clone());
        }
        if let Some(t) = self
            .preview_picture
            .borrow()
            .as_ref()
            .filter(|t| t.id == id)
        {
            t.ring.set(progress);
        }
    }
    pub(super) fn cancel_download(self: &Rc<Self>, id: u64, pause: bool) {
        if let Some(c) = self.queue_cancels.borrow().get(&id) {
            c.store(true, Ordering::Relaxed);
        }
        if let Some(r) = self.queue.borrow_mut().iter_mut().find(|r| r.id == id) {
            r.status = if pause { "paused" } else { "cancelled" }.into();
            r.progress = None;
            r.updated_at = crate::runtime::now();
        }
        self.pool.cancel_queued();
        self.queue_changed();
    }
    pub(super) fn trash(self: &Rc<Self>, item: &Wallpaper) {
        let Some(record) = self.library.borrow().resolve(item).cloned() else {
            self.toast("Bu dosya Duvar tarafından yönetilmiyor.");
            return;
        };
        let settings = gio::Settings::new("org.gnome.desktop.background");
        let current = [
            settings.string("picture-uri"),
            settings.string("picture-uri-dark"),
        ]
        .iter()
        .any(|u| {
            url::Url::parse(u)
                .ok()
                .and_then(|u| u.to_file_path().ok())
                .is_some_and(|p| p == Path::new(&record.path))
        });
        if current {
            if let Some(window) = self.window.upgrade() {
                let dialog=adw::AlertDialog::builder().heading("Etkin duvar kâğıdı çöpe taşınsın mı?").body("Bu dosya şu an masaüstünde kullanılıyor. Çöpe taşırsan başka bir duvar kâğıdı seçmen gerekir. Dosya kalıcı silinmez.").build();
                dialog.add_responses(&[("cancel", "Vazgeç"), ("trash", "Çöpe taşı")]);
                dialog.set_response_appearance("trash", adw::ResponseAppearance::Destructive);
                dialog.set_close_response("cancel");
                let w = weak(self);
                dialog.connect_response(None, move |_, answer| {
                    if answer == "trash"
                        && let Some(s) = w.upgrade()
                    {
                        s.pool.push(Job::Trash(record.clone()));
                        s.ensure_poll();
                    }
                });
                dialog.present(Some(&window));
            }
        } else {
            self.pool.push(Job::Trash(record));
            self.ensure_poll();
        }
    }
}

pub(super) fn information(item: &Wallpaper, f: &Facts) -> String {
    let provenance = match f.dimension_origin.as_str() {
        "file" => {
            if f.verified {
                "Dosyadan doğrulandı"
            } else {
                "Dosya başlığından ölçüldü"
            }
        }
        "original_header" => "Özgün görsel başlığı",
        "source" => "Kaynağın bildirdiği",
        "unknown" => "Boyut bilgisi bekleniyor",
        _ => "Boyut bilgisi",
    };
    let mut fields = vec![
        f.dimensions(),
        f.orientation_label().into(),
        format!("Oran {}", f.ratio()),
        provenance.into(),
    ];
    if let Some(mp) = f.megapixels() {
        fields.push(format!("{mp:.2} MP"));
    }
    if !f.format.is_empty() {
        fields.push(f.format.to_uppercase());
    }
    if f.bytes > 0 {
        fields.push(crate::runtime::size(f.bytes));
    }
    if !item.author.is_empty() {
        fields.push(format!("Sanatçı: {}", item.author));
    }
    fields.push(if item.license.is_empty() {
        "Lisans belirtilmemiş; kaynak koşullarını kontrol et".into()
    } else {
        format!("Lisans: {}", item.license)
    });
    fields.push(format!(
        "Türler: {}",
        crate::runtime::types(item).join(" · ")
    ));
    fields.join("\n")
}

pub(super) fn action_box() -> gtk::FlowBox {
    let fb = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .activate_on_single_click(false)
        .homogeneous(false)
        .column_spacing(6)
        .row_spacing(6)
        .max_children_per_line(8)
        .min_children_per_line(1)
        .halign(gtk::Align::Center)
        .build();
    fb.add_css_class("duvar-clean-flow");
    fb
}

pub(super) fn action_row() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.set_halign(gtk::Align::Center);
    row.set_valign(gtk::Align::Center);
    row
}

fn button(label: &str, icon: &str, compact: bool) -> gtk::Button {
    let b = if compact {
        gtk::Button::from_icon_name(icon)
    } else {
        gtk::Button::with_label(label)
    };
    b.set_valign(gtk::Align::Center);
    b.set_tooltip_text(Some(label));
    b.update_property(&[gtk::accessible::Property::Label(label)]);
    if compact {
        b.add_css_class("flat");
    }
    b
}

pub(super) fn apply_for_scope(state: &Rc<State>, item: &Wallpaper, scope: &str, local: bool) {
    if !local {
        state.download(item.clone(), Some(scope));
        return;
    }
    let image = state.library.borrow().localize(item).image;
    let result = url::Url::parse(&image)
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .ok_or_else(|| anyhow::anyhow!("Dosya artık mevcut değil"))
        .and_then(|p| state.apply_wallpaper(&p, scope));
    if let Err(e) = result {
        state.toast(&e.to_string());
        state.refresh_downloads();
    }
}

/// Fixed-height card action layout:
/// - `quick_box`: floating circular OSD pills in top-right thumbnail corner (Favorite + Download/Cancel/Trash/Undo)
/// - `apply_box`: uniform 1-row linked pill pair below the card metadata (`Açığa uygula` | `Koyuya uygula`)
///
/// Card height NEVER changes across state transitions (eliminating GtkGridView scroll jumps).
pub(super) fn fill_card_actions(
    state: &Rc<State>,
    quick_box: &gtk::Box,
    apply_box: &gtk::Box,
    item: &Wallpaper,
) {
    while let Some(child) = quick_box.first_child() {
        quick_box.remove(&child);
    }
    while let Some(child) = apply_box.first_child() {
        apply_box.remove(&child);
    }
    let phase = state.phase(item);
    let actions = state.library.borrow().actions(item, phase);

    // Top-right floating OSD pills on thumbnail
    let is_fav = state
        .config
        .borrow()
        .favorites
        .iter()
        .any(|w| w.id == item.id);
    let star = gtk::ToggleButton::builder()
        .icon_name(if is_fav {
            "starred-symbolic"
        } else {
            "non-starred-symbolic"
        })
        .active(is_fav)
        .tooltip_text("Favoriye ekle / çıkar")
        .build();
    star.update_property(&[gtk::accessible::Property::Label("Favori")]);
    star.add_css_class("circular");
    star.add_css_class("osd");
    star.add_css_class("duvar-card-osd-btn");
    if is_fav {
        star.add_css_class("duvar-fav-active");
    }
    let w = weak(state);
    let it = item.clone();
    star.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            s.favorite(&it);
            s.refresh_cards(Some(&it.id), true);
        }
    });
    quick_box.append(&star);

    if actions.restore {
        let b = gtk::Button::from_icon_name("edit-undo-symbolic");
        b.set_tooltip_text(Some("Geri al"));
        b.update_property(&[gtk::accessible::Property::Label("Geri al")]);
        b.add_css_class("circular");
        b.add_css_class("osd");
        b.add_css_class("duvar-card-osd-btn");
        let w = weak(state);
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade()
                && let Some(t) = s.last_trash.borrow().clone()
            {
                s.pool.push(Job::Restore(t));
                s.ensure_poll();
            }
        });
        quick_box.append(&b);
    } else if actions.cancel {
        let b = gtk::Button::from_icon_name("process-stop-symbolic");
        b.set_tooltip_text(Some("İptal et"));
        b.update_property(&[gtk::accessible::Property::Label("İptal et")]);
        b.add_css_class("circular");
        b.add_css_class("osd");
        b.add_css_class("duvar-card-osd-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                let id = s.progress_for(&it).map(|p| p.job);
                if let Some(id) = id {
                    s.cancel_download(id, false);
                }
            }
        });
        quick_box.append(&b);
    } else if actions.download {
        let (label, icon) = if actions.retry {
            ("Tekrar dene", "view-refresh-symbolic")
        } else {
            ("İndir", "folder-download-symbolic")
        };
        let b = gtk::Button::from_icon_name(icon);
        b.set_tooltip_text(Some(label));
        b.update_property(&[gtk::accessible::Property::Label(label)]);
        b.add_css_class("circular");
        b.add_css_class("osd");
        b.add_css_class("duvar-card-osd-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                s.download(it.clone(), None);
            }
        });
        quick_box.append(&b);
    } else if actions.trash {
        let b = gtk::Button::from_icon_name("user-trash-symbolic");
        b.set_tooltip_text(Some("Çöp kutusuna taşı"));
        b.update_property(&[gtk::accessible::Property::Label("Çöp kutusuna taşı")]);
        b.add_css_class("circular");
        b.add_css_class("osd");
        b.add_css_class("duvar-card-osd-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                s.trash(&it);
            }
        });
        quick_box.append(&b);
    }

    // Bottom linked 1-row apply pair (exact uniform height)
    if actions.apply || actions.download {
        for (scope, label, short, icon) in [
            (
                "light",
                "Açık tema için uygula",
                "Açığa uygula",
                "weather-clear-symbolic",
            ),
            (
                "dark",
                "Koyu tema için uygula",
                "Koyuya uygula",
                "weather-clear-night-symbolic",
            ),
        ] {
            let b = gtk::Button::new();
            b.set_hexpand(true);
            b.add_css_class("duvar-card-apply-btn");
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 3);
            content.set_halign(gtk::Align::Center);
            let img = gtk::Image::from_icon_name(icon);
            img.set_pixel_size(14);
            content.append(&img);
            let lbl = gtk::Label::new(Some(short));
            lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            content.append(&lbl);
            b.set_child(Some(&content));
            b.set_tooltip_text(Some(&if actions.download {
                format!("{label} · Gerekirse indirilir")
            } else {
                label.into()
            }));
            b.update_property(&[gtk::accessible::Property::Label(label)]);
            let w = weak(state);
            let it = item.clone();
            let local = actions.apply;
            b.connect_clicked(move |_| {
                if let Some(s) = w.upgrade() {
                    apply_for_scope(&s, &it, scope, local);
                }
            });
            apply_box.append(&b);
        }
    } else if actions.restore {
        let b = gtk::Button::with_label("Geri al");
        b.set_hexpand(true);
        b.add_css_class("duvar-card-apply-btn");
        let w = weak(state);
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade()
                && let Some(t) = s.last_trash.borrow().clone()
            {
                s.pool.push(Job::Restore(t));
                s.ensure_poll();
            }
        });
        apply_box.append(&b);
    } else {
        let hint = gtk::Label::new(Some(phase.label()));
        hint.set_hexpand(true);
        hint.add_css_class("caption");
        hint.add_css_class("dim-label");
        apply_box.append(&hint);
    }
}

pub(super) fn fill_actions(state: &Rc<State>, a: &gtk::Box, item: &Wallpaper, compact: bool) {
    while let Some(child) = a.first_child() {
        a.remove(&child);
    }
    let phase = state.phase(item);
    let actions = state.library.borrow().actions(item, phase);
    let mut has_primary = false;
    if actions.apply || actions.download {
        has_primary = true;
        for (scope, label, short, icon, primary) in [
            (
                "light",
                "Açık tema için uygula",
                "Açığa uygula",
                "weather-clear-symbolic",
                false,
            ),
            (
                "dark",
                "Koyu tema için uygula",
                "Koyuya uygula",
                "weather-clear-night-symbolic",
                true,
            ),
        ] {
            let b = button(label, icon, false);
            if primary && !compact {
                b.add_css_class("suggested-action");
            }
            b.add_css_class("duvar-action-btn");
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            content.set_halign(gtk::Align::Center);
            content.append(&gtk::Image::from_icon_name(icon));
            content.append(&gtk::Label::new(Some(if compact { short } else { label })));
            b.set_child(Some(&content));
            if actions.download {
                b.set_tooltip_text(Some(&format!("{label} · Gerekirse indirilir")));
            }
            let w = weak(state);
            let it = item.clone();
            let local = actions.apply;
            b.connect_clicked(move |_| {
                if let Some(s) = w.upgrade() {
                    apply_for_scope(&s, &it, scope, local);
                }
            });
            a.append(&b);
        }
    }
    if actions.restore {
        has_primary = true;
        let b = button("Geri al", "edit-undo-symbolic", compact);
        b.add_css_class("duvar-action-btn");
        let w = weak(state);
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade()
                && let Some(t) = s.last_trash.borrow().clone()
            {
                s.pool.push(Job::Restore(t));
                s.ensure_poll();
            }
        });
        a.append(&b);
    }
    if actions.download {
        has_primary = true;
        let (label, icon) = if actions.retry {
            ("Tekrar dene", "view-refresh-symbolic")
        } else {
            ("İndir", "folder-download-symbolic")
        };
        let b = button(label, icon, false);
        b.add_css_class("duvar-action-btn");
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        content.set_halign(gtk::Align::Center);
        content.append(&gtk::Image::from_icon_name(icon));
        content.append(&gtk::Label::new(Some(label)));
        b.set_child(Some(&content));
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                s.download(it.clone(), None);
            }
        });
        a.append(&b);
    }
    if actions.cancel {
        has_primary = true;
        let b = button("İptal et", "process-stop-symbolic", false);
        b.add_css_class("duvar-action-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                let id = s.progress_for(&it).map(|p| p.job);
                if let Some(id) = id {
                    s.cancel_download(id, false);
                }
            }
        });
        a.append(&b);
    }

    if has_primary {
        let sep = gtk::Separator::new(gtk::Orientation::Vertical);
        sep.set_margin_top(4);
        sep.set_margin_bottom(4);
        a.append(&sep);
    }

    // Crisp uniform 34x34 circular utility buttons (never stretched, zero ghost hitbox!)
    let utils = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    utils.set_valign(gtk::Align::Center);
    let is_fav = state
        .config
        .borrow()
        .favorites
        .iter()
        .any(|w| w.id == item.id);
    let star = gtk::ToggleButton::builder()
        .icon_name(if is_fav {
            "starred-symbolic"
        } else {
            "non-starred-symbolic"
        })
        .active(is_fav)
        .valign(gtk::Align::Center)
        .tooltip_text("Favoriye ekle / çıkar")
        .build();
    star.update_property(&[gtk::accessible::Property::Label("Favori")]);
    star.add_css_class("circular");
    star.add_css_class("flat");
    star.add_css_class("duvar-icon-btn");
    if is_fav {
        star.add_css_class("duvar-fav-active");
    }
    let w = weak(state);
    let it = item.clone();
    star.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            s.favorite(&it);
            s.refresh_cards(Some(&it.id), true);
        }
    });
    utils.append(&star);

    if actions.trash {
        let b = button("Çöp kutusuna taşı", "user-trash-symbolic", true);
        b.add_css_class("circular");
        b.add_css_class("duvar-icon-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                s.trash(&it);
            }
        });
        utils.append(&b);
    }
    if actions.open && !compact {
        let b = button("Görüntüleyicide aç", "image-x-generic-symbolic", true);
        b.add_css_class("circular");
        b.add_css_class("duvar-icon-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                let local = s.library.borrow().localize(&it);
                if let Err(e) = gio::AppInfo::launch_default_for_uri(
                    &local.image,
                    None::<&gio::AppLaunchContext>,
                ) {
                    s.toast(&e.to_string());
                }
            }
        });
        utils.append(&b);
        let b = button("Klasörü aç", "folder-open-symbolic", true);
        b.add_css_class("circular");
        b.add_css_class("duvar-icon-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                let local = s.library.borrow().localize(&it);
                if let Some(p) = gio::File::for_uri(&local.image).parent()
                    && let Err(e) = gio::AppInfo::launch_default_for_uri(
                        &p.uri(),
                        None::<&gio::AppLaunchContext>,
                    )
                {
                    s.toast(&e.to_string());
                }
            }
        });
        utils.append(&b);
    }
    if !compact && state.facts_for(item).width == 0 {
        let b = button("Boyutu öğren", "dialog-information-symbolic", true);
        b.add_css_class("circular");
        b.add_css_class("duvar-icon-btn");
        let w = weak(state);
        let it = item.clone();
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                s.pool.push(Job::Metadata(s.current(), it.clone()));
                s.ensure_poll();
            }
        });
        utils.append(&b);
    }
    if !compact && item.landing.starts_with("https://") {
        let b = button("Kaynağı tarayıcıda aç", "web-browser-symbolic", true);
        b.add_css_class("circular");
        b.add_css_class("duvar-icon-btn");
        let url = item.landing.clone();
        let w = weak(state);
        b.connect_clicked(move |_| {
            if let Err(e) =
                gio::AppInfo::launch_default_for_uri(&url, None::<&gio::AppLaunchContext>)
                && let Some(s) = w.upgrade()
            {
                s.toast(&e.to_string());
            }
        });
        utils.append(&b);
    }
    a.append(&utils);
}

fn action_signature(s: &State, item: &Wallpaper) -> String {
    let phase = s.phase(item);
    format!(
        "{:?}:{}:{}",
        s.library.borrow().actions(item, phase),
        s.config.borrow().favorites.iter().any(|f| f.id == item.id),
        phase == Phase::Trashed
    )
}
