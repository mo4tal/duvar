use super::*;

pub(super) fn filters(state: &Rc<State>, page: &adw::PreferencesPage) {
    let notes = crate::registry::compatibility_notes(&state.config.borrow());
    if !notes.is_empty() {
        let compat = adw::PreferencesGroup::builder()
            .title("Etkin kaynak uyumu")
            .description("Seçili kaynakların bu filtrelerle nasıl etkileştiği:")
            .build();
        for note in notes {
            let row = adw::ActionRow::builder()
                .title(&note)
                .use_markup(false)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
            compat.add(&row);
        }
        page.add(&compat);
    }

    let purity = adw::PreferencesGroup::builder()
        .title("İçerik izinleri")
        .description(
            "Kaynağın güvenlik etiketine göre uygulanır. Kategori seçimi bu izinleri değiştirmez.",
        )
        .build();
    switch(
        state,
        &purity,
        "SFW",
        "Kaynağın güvenli olarak etiketlediği görseller.",
        |c| c.policy.safe,
        |c, v| c.policy.safe = v,
    );
    switch(
        state,
        &purity,
        "Şüpheli içerik",
        "Hassas veya sınırda etiketlenmiş görseller.",
        |c| c.policy.sketchy,
        |c, v| c.policy.sketchy = v,
    );
    switch(
        state,
        &purity,
        "Yetişkin içerik",
        "Başlangıçta kapalıdır. Wallhaven bu sınıf için API anahtarı ister.",
        |c| c.policy.adult,
        |c, v| c.policy.adult = v,
    );
    switch(
        state,
        &purity,
        "Etiketsiz içerik",
        "Yerel dosyalar, Commons ve güvenlik derecesi vermeyen sanat arşivleri.",
        |c| c.policy.unknown,
        |c, v| c.policy.unknown = v,
    );
    switch(
        state,
        &purity,
        "Hassas içerik izinlerini hatırla",
        "Kapalıyken her yeni pencere yalnızca SFW ile başlar.",
        |c| c.remember_sensitive,
        |c, v| c.remember_sensitive = v,
    );
    page.add(&purity);

    let themes = adw::PreferencesGroup::builder().title("Konular").build();
    let row = adw::ActionRow::builder()
        .title("Kategoriler")
        .subtitle("100 genel · 100 hassas içerik konusu")
        .build();
    state.topic_summary.set(Some(&row));
    let button = gtk::Button::with_label("Seç");
    button.set_valign(gtk::Align::Center);
    button.add_css_class("pill");
    let w = weak(state);
    button.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            topics::show(&s);
        }
    });
    row.add_suffix(&button);
    themes.add(&row);
    page.add(&themes);
    topics::summary(state);

    let quality = adw::PreferencesGroup::builder()
        .title("Çözünürlük ve biçim")
        .description("Minimum piksel boyutu, yön ve dosya biçimi tercihleri.")
        .build();
    extra_filters(state, &quality, page);
    page.add(&quality);

    let advanced = adw::PreferencesGroup::builder()
        .title("Etiketler ve kaynak sınıfları")
        .build();
    let tags = adw::ExpanderRow::builder()
        .title("Etiket kuralları")
        .subtitle(
            "İstenen ve engellenen etiketler; virgülle ayırıp Enter veya onay düğmesiyle uygula.",
        )
        .build();
    for (title, get, set) in [
        (
            "Engelleme listesi etkin",
            (|c: &Config| c.policy.exclude_enabled) as fn(&Config) -> bool,
            (|c: &mut Config, v| c.policy.exclude_enabled = v) as fn(&mut Config, bool),
        ),
        (
            "Dahil etme listesi etkin",
            |c: &Config| c.policy.include_enabled,
            |c: &mut Config, v| c.policy.include_enabled = v,
        ),
        (
            "Bütün istenen etiketler eşleşsin",
            |c: &Config| c.policy.all_included,
            |c: &mut Config, v| c.policy.all_included = v,
        ),
    ] {
        let r = adw::SwitchRow::builder()
            .title(title)
            .active(get(&state.config.borrow()))
            .build();
        let w = weak(state);
        r.connect_active_notify(move |r| {
            if let Some(s) = w.upgrade() {
                set(&mut s.config.borrow_mut(), r.is_active());
                s.changed();
            }
        });
        tags.add_row(&r);
    }
    for (title, get, set) in [
        (
            "Engellenecek etiketler",
            (|c: &Config| c.policy.exclude_tags.clone()) as fn(&Config) -> String,
            (|c: &mut Config, v: String| c.policy.exclude_tags = v) as fn(&mut Config, String),
        ),
        (
            "İstenen etiketler",
            |c: &Config| c.policy.include_tags.clone(),
            |c: &mut Config, v: String| c.policy.include_tags = v,
        ),
    ] {
        let r = adw::EntryRow::builder()
            .title(title)
            .text(get(&state.config.borrow()))
            .show_apply_button(true)
            .build();
        let w = weak(state);
        r.connect_apply(move |r| {
            if let Some(s) = w.upgrade() {
                set(
                    &mut s.config.borrow_mut(),
                    r.text().chars().take(512).collect(),
                );
                s.changed();
            }
        });
        tags.add_row(&r);
    }
    advanced.add(&tags);

    let masks = adw::ExpanderRow::builder()
        .title("Wallhaven kaynak sınıfları")
        .subtitle("Yalnızca Wallhaven sonuçlarını sınırlar.")
        .build();
    for (i, title) in ["Genel", "Anime", "İnsan"].into_iter().enumerate() {
        let r = adw::SwitchRow::builder()
            .title(title)
            .active(state.config.borrow().categories[i])
            .build();
        let w = weak(state);
        r.connect_active_notify(move |r| {
            if let Some(s) = w.upgrade() {
                s.config.borrow_mut().categories[i] = r.is_active();
                s.changed();
            }
        });
        masks.add_row(&r);
    }
    advanced.add(&masks);
    page.add(&advanced);

    let reset_group = adw::PreferencesGroup::new();
    let filter_page = page.downgrade();
    let reset = adw::ButtonRow::builder()
        .title("Filtreleri güvenli varsayılana döndür")
        .start_icon_name("edit-undo-symbolic")
        .build();
    reset.add_css_class("destructive-action");
    let w = weak(state);
    reset.connect_activated(move |_| {
        if let Some(s) = w.upgrade()
            && let Some(win) = s.window.upgrade()
        {
            let dlg = adw::AlertDialog::builder()
                .heading("Filtreler sıfırlansın mı?")
                .body(format!(
                    "{} etkin filtre temizlenecek. Kaynak ve klasör tercihlerin korunur.",
                    s.config.borrow().policy.active_filters()
                ))
                .build();
            dlg.add_responses(&[("cancel", "Vazgeç"), ("reset", "Sıfırla")]);
            dlg.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
            dlg.set_close_response("cancel");
            let w = weak(&s);
            let page = filter_page.clone();
            dlg.connect_response(None, move |_, id| {
                if id == "reset"
                    && let Some(s) = w.upgrade()
                {
                    s.config.borrow_mut().policy = crate::model::Policy::default();
                    s.changed();
                    topics::summary(&s);
                    if let Some(page) = page.upgrade() {
                        rebuild_filters(&s, &page);
                    }
                    s.toast("Filtreler sıfırlandı.");
                }
            });
            dlg.present(Some(&win));
        }
    });
    reset_group.add(&reset);
    page.add(&reset_group);
}

pub(super) fn rebuild_filters(state: &Rc<State>, page: &adw::PreferencesPage) {
    fn groups(widget: &gtk::Widget, out: &mut Vec<adw::PreferencesGroup>) {
        if let Some(group) = widget.downcast_ref::<adw::PreferencesGroup>() {
            out.push(group.clone());
            return;
        }
        let mut c = widget.first_child();
        while let Some(ch) = c {
            c = ch.next_sibling();
            groups(&ch, out);
        }
    }
    let mut old = Vec::new();
    groups(page.upcast_ref(), &mut old);
    for group in old {
        page.remove(&group);
    }
    filters(state, page);
}

pub(super) fn settings(state: &Rc<State>, page: &adw::PreferencesPage) {
    let providers = adw::PreferencesGroup::builder()
        .title("Koleksiyonlar")
        .build();
    let active_names = crate::registry::enabled(&state.config.borrow())
        .iter()
        .map(|s| s.label)
        .collect::<Vec<_>>();
    let sub = if active_names.is_empty() {
        "Etkin kaynak yok".into()
    } else {
        format!(
            "{} etkin kaynak · {}",
            active_names.len(),
            active_names.join(", ")
        )
    };
    let row = adw::ActionRow::builder()
        .title("Görsel kaynakları")
        .subtitle(&sub)
        .use_markup(false)
        .build();
    state.sources_summary.set(Some(&row));
    let button = gtk::Button::with_label("Kaynakları yönet");
    button.set_valign(gtk::Align::Center);
    button.add_css_class("pill");
    let w = weak(state);
    button.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            sources(&s);
        }
    });
    row.add_suffix(&button);
    providers.add(&row);
    page.add(&providers);

    let files = adw::PreferencesGroup::builder()
        .title("Dosyalar ve duvar kâğıdı")
        .build();
    folder(state, &files, false);
    folder(state, &files, true);
    let undo_row = adw::ActionRow::builder()
        .title("Son uygulamayı geri al")
        .subtitle("Önceki açık ve koyu duvar kâğıdını geri yükler.")
        .build();
    let undo = gtk::Button::with_label("Geri al");
    undo.set_valign(gtk::Align::Center);
    let w = weak(state);
    undo.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            match undo_wallpaper() {
                Ok(()) => s.toast("Önceki duvar kâğıdı geri yüklendi."),
                Err(e) => s.toast(&e.to_string()),
            }
        }
    });
    undo_row.add_suffix(&undo);
    files.add(&undo_row);
    page.add(&files);

    let behavior = adw::PreferencesGroup::builder()
        .title("Görünüm ve gizlilik")
        .build();
    let checker = adw::SwitchRow::builder()
        .title("Saydam görsellerde dama zemini")
        .subtitle("Yeni açılan önizlemede uygulanır; tema rengine uyar.")
        .active(state.config.borrow().behavior.checkerboard)
        .build();
    let w = weak(state);
    checker.connect_active_notify(move |r| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().behavior.checkerboard = r.is_active();
            s.save();
        }
    });
    behavior.add(&checker);
    let history = adw::SwitchRow::builder()
        .title("Arama geçmişini hatırla")
        .subtitle("Başlangıçta kapalıdır. Kapattığında kayıtlı geçmiş temizlenir.")
        .active(state.config.borrow().behavior.history_enabled)
        .build();
    let w = weak(state);
    history.connect_active_notify(move |r| {
        if let Some(s) = w.upgrade() {
            let mut c = s.config.borrow_mut();
            c.behavior.history_enabled = r.is_active();
            if !r.is_active() {
                c.behavior.search_history.clear();
            }
            drop(c);
            s.save();
        }
    });
    behavior.add(&history);
    page.add(&behavior);

    maintenance(state, page);

    let info = adw::PreferencesGroup::new();
    let about = adw::ActionRow::builder()
        .title("Duvar hakkında")
        .subtitle(concat!(
            "Sürüm ",
            env!("CARGO_PKG_VERSION"),
            " · Sürüm notları ve lisans"
        ))
        .activatable(true)
        .build();
    let app_logo = gtk::Image::from_icon_name("io.github.mo4tal.Duvar");
    app_logo.set_pixel_size(32);
    about.add_prefix(&app_logo);
    about.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    let w = weak(state);
    about.connect_activated(move |_| {
        if let Some(s) = w.upgrade()
            && let Some(win) = s.window.upgrade()
        {
            let debug_info = format!(
                "Duvar {}\nGTK {}.{}.{}\nLibadwaita {}.{}.{}\nMimari: {}",
                env!("CARGO_PKG_VERSION"),
                gtk::major_version(),
                gtk::minor_version(),
                gtk::micro_version(),
                unsafe { adw::ffi::adw_get_major_version() },
                unsafe { adw::ffi::adw_get_minor_version() },
                unsafe { adw::ffi::adw_get_micro_version() },
                std::env::consts::ARCH
            );
            let about = adw::AboutDialog::builder()
                .application_name("Duvar")
                .application_icon("io.github.mo4tal.Duvar")
                .version(env!("CARGO_PKG_VERSION"))
                .developer_name("mo4tal")
                .developers(vec!["mo4tal https://github.com/mo4tal".to_string()])
                .website("https://github.com/mo4tal/duvar")
                .issue_url("https://github.com/mo4tal/duvar/issues")
                .copyright("© 2026 mo4tal")
                .license_type(gtk::License::Gpl30)
                .comments("GNOME için çok kaynaklı duvar kâğıdı galerisi ve duvar kâğıdına uyarlanan dinamik renk motoru.")
                .release_notes_version(env!("CARGO_PKG_VERSION"))
                .release_notes(
                    "<p>0.6.0 sürümüyle galeri arayüzü, önizleme deneyimi ve üst panel tema motoru baştan sona yenilendi.</p>\
                     <ul>\
                       <li>Açık ve koyu temada duvar kâğıdına uyarlanan ortam tonlu cam üst panel kapsülleri ve canlı stil/yoğunluk seçimi</li>\
                       <li>OKLab baskın renk analizi, 16 dilimli üst şerit kontrast ölçümü ve User Themes olmadan doğrudan Shell desteği</li>\
                       <li>6 hatlı paralel küçük resim motoru, 180 öğelik LRU bellek önbelleği ve sıçramasız sonsuz kaydırma</li>\
                       <li>Fare tekerleğiyle imlece yakınlaştırma, sürükle-kaydır, sağ tık menüsü ve bilgi çekmeceli yeni önizleme penceresi</li>\
                       <li>Yeni Adwaita uygulama simgesi ve tüm sekmelerde sadeleştirilmiş tasarım dili</li>\
                     </ul>",
                )
                .debug_info(&debug_info)
                .build();
            about.add_credit_section(
                Some("Esinlenme ve ekosistem"),
                &["WSelector (işlevsel esinlenme)", "GNOME Shell, GTK 4 ve Libadwaita"],
            );
            about.add_acknowledgement_section(
                Some("Üretim notu"),
                &["Yapay zekâ destekli geliştirme (vibe coding) akışıyla tasarlandı ve doğrulandı"],
            );
            about.present(Some(&win));
        }
    });
    info.add(&about);
    page.add(&info);
}

fn folder(state: &Rc<State>, group: &adw::PreferencesGroup, local: bool) {
    let path = if local {
        state.config.borrow().local_directory.clone()
    } else if state.config.borrow().download_directory.is_empty() {
        crate::store::downloads().to_string_lossy().into()
    } else {
        state.config.borrow().download_directory.clone()
    };
    let row = adw::ActionRow::builder()
        .title(if local {
            "Resim klasörü"
        } else {
            "İndirme klasörü"
        })
        .subtitle(if path.is_empty() {
            "Henüz seçilmedi"
        } else {
            &path
        })
        .use_markup(false)
        .build();
    let pick = gtk::Button::from_icon_name("folder-open-symbolic");
    pick.set_valign(gtk::Align::Center);
    pick.set_tooltip_text(Some("Klasör seç"));
    let w = weak(state);
    let r = row.clone();
    pick.connect_clicked(move |_| {
        let Some(s) = w.upgrade() else {
            return;
        };
        let Some(win) = s.window.upgrade() else {
            return;
        };
        let w = weak(&s);
        let r = r.clone();
        gtk::FileDialog::new().select_folder(
            Some(&win),
            None::<&gio::Cancellable>,
            move |result| {
                if let (Some(s), Ok(file)) = (w.upgrade(), result)
                    && let Some(path) = file.path()
                {
                    if local {
                        s.config.borrow_mut().local_directory = path.to_string_lossy().into();
                    } else {
                        s.config.borrow_mut().download_directory = path.to_string_lossy().into();
                    }
                    r.set_subtitle(&path.to_string_lossy());
                    s.changed();
                }
            },
        );
    });
    row.add_suffix(&pick);
    group.add(&row);
}

fn source_subtitle(id: &str, base: &str) -> String {
    let Some(cap) = crate::registry::SOURCES.iter().find(|s| s.id == id) else {
        return base.into();
    };
    let mut tags = Vec::new();
    if cap.search {
        tags.push("Arama");
    }
    if cap.dates {
        tags.push("Tarih");
    }
    if cap.dimensions {
        tags.push("Boyut");
    }
    if cap.tags {
        tags.push("Etiket");
    }
    if cap.unknown_permission {
        tags.push("Etiketsiz izni gerekir");
    }
    format!("{base} · {}", tags.join(" · "))
}

pub(super) fn sources(state: &Rc<State>) {
    let Some(window) = state.window.upgrade() else {
        return;
    };
    let dialog = adw::Dialog::builder()
        .title("Görsel kaynakları")
        .content_width(680)
        .content_height(720)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 22);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.set_margin_top(12);
    content.set_margin_bottom(24);

    let photo = adw::PreferencesGroup::builder()
        .title("Fotoğraf ve duvar kâğıdı")
        .build();
    switch(
        state,
        &photo,
        "Wallhaven",
        &source_subtitle("Wallhaven", "Yüksek kaliteli duvar kâğıtları"),
        |c| c.wallhaven,
        |c, v| c.wallhaven = v,
    );
    switch(
        state,
        &photo,
        "Openverse",
        &source_subtitle("Openverse", "Açık lisanslı fotoğraf ve görseller"),
        |c| c.openverse,
        |c, v| c.openverse = v,
    );
    switch(
        state,
        &photo,
        "NASA",
        &source_subtitle("NASA", "Özgün uzay ve astronomi fotoğrafları"),
        |c| c.nasa,
        |c, v| c.nasa = v,
    );
    switch(
        state,
        &photo,
        "Bing",
        &source_subtitle("Bing", "Son 8 günün 4K UHD günlük görselleri"),
        |c| c.bing,
        |c, v| c.bing = v,
    );
    switch(
        state,
        &photo,
        "Wikimedia Commons",
        &source_subtitle("Commons", "Özgür medya arşivi"),
        |c| c.commons,
        |c, v| c.commons = v,
    );
    content.append(&photo);

    let art = adw::PreferencesGroup::builder()
        .title("Sanat koleksiyonları")
        .description(
            "Müze arşivleri güvenlik derecesi sunmadığı için Etiketsiz içerik izni gerektirir.",
        )
        .build();
    switch(
        state,
        &art,
        "Cleveland Museum of Art",
        &source_subtitle("Cleveland", "CC0 müze koleksiyonu"),
        |c| c.cleveland,
        |c, v| c.cleveland = v,
    );
    switch(
        state,
        &art,
        "The Met",
        &source_subtitle("The Met", "Açık erişimli sanat eserleri"),
        |c| c.met,
        |c, v| c.met = v,
    );
    switch(
        state,
        &art,
        "Victoria and Albert Museum",
        &source_subtitle("V&A", "Yüksek çözünürlüklü sanat ve tasarım"),
        |c| c.vam,
        |c, v| c.vam = v,
    );
    content.append(&art);

    let sensitive = adw::PreferencesGroup::builder()
        .title("Çizim ve anime arşivleri")
        .description(
            "Kaynağı açmak yetişkin içerik izni vermez; Filtreler'deki tercihler ayrıca uygulanır.",
        )
        .build();
    switch(
        state,
        &sensitive,
        "Yande.re",
        &source_subtitle("Yande.re", "Yüksek çözünürlüklü çizim arşivi"),
        |c| c.yandere,
        |c, v| c.yandere = v,
    );
    switch(
        state,
        &sensitive,
        "AIBooru",
        &source_subtitle("AIBooru", "Yapay zekâ çizimleri"),
        |c| c.aibooru,
        |c, v| c.aibooru = v,
    );
    switch(
        state,
        &sensitive,
        "NekosAPI",
        &source_subtitle("NekosAPI", "Anime illüstrasyon arşivi"),
        |c| c.nekos,
        |c, v| c.nekos = v,
    );
    content.append(&sensitive);

    let personal = adw::PreferencesGroup::builder()
        .title("Kişisel koleksiyon")
        .build();
    switch(
        state,
        &personal,
        "Yerel klasör",
        &source_subtitle("Yerel", "Ayarlar'daki resim klasörünü tarar"),
        |c| c.local,
        |c, v| c.local = v,
    );
    content.append(&personal);

    let access = adw::PreferencesGroup::builder()
        .title("Wallhaven erişimi")
        .build();
    let expand = adw::ExpanderRow::builder()
        .title("API anahtarı")
        .subtitle("Wallhaven yetişkin içerik sorguları için gereklidir.")
        .build();
    let key = adw::PasswordEntryRow::builder()
        .title("Wallhaven API anahtarı")
        .text(state.api_key.borrow().as_str())
        .build();
    state.key_entry.set(Some(&key));
    let w = weak(state);
    key.connect_changed(move |r| {
        if let Some(s) = w.upgrade() {
            s.key_edited.set(true);
            s.api_key
                .replace(r.text().trim().chars().take(256).collect());
        }
    });
    expand.add_row(&key);
    let row = adw::ActionRow::builder()
        .title("Sistem anahtarlığı")
        .subtitle("Yalnızca mevcut anahtarlığa kayıt eklenir; parolası değiştirilmez.")
        .build();
    let save = gtk::Button::with_label("Kaydet");
    save.set_valign(gtk::Align::Center);
    save.add_css_class("suggested-action");
    state.key_save.set(Some(&save));
    let w = weak(state);
    save.connect_clicked(move |button| {
        if let Some(s) = w.upgrade() {
            let key = s.api_key.borrow().clone();
            if key.is_empty() {
                s.toast("Önce API anahtarını gir.");
                return;
            }
            if s.pool.push(Job::SaveKey("wallhaven".into(), key)) {
                button.set_sensitive(false);
                s.ensure_poll();
            }
        }
    });
    row.add_suffix(&save);
    let remove = gtk::Button::from_icon_name("edit-delete-symbolic");
    remove.set_valign(gtk::Align::Center);
    remove.add_css_class("flat");
    remove.set_tooltip_text(Some("Duvar’ın Wallhaven anahtarını anahtarlıktan kaldır"));
    let w = weak(state);
    remove.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            s.pool.push(Job::RemoveKey("wallhaven".into()));
            s.ensure_poll();
        }
    });
    row.add_suffix(&remove);
    let known = adw::ActionRow::builder()
        .title("Erişim durumu")
        .subtitle(if state.key_registered.get() {
            "Anahtarlıkta Duvar anahtarı kayıtlı; değer gizli"
        } else {
            "Anahtarlıkta kayıtlı anahtar yüklenmedi"
        })
        .build();
    state.key_status.set(Some(&known));
    expand.add_row(&known);
    expand.add_row(&row);
    access.add(&expand);
    content.append(&access);

    let clamp = adw::Clamp::builder()
        .maximum_size(640)
        .child(&content)
        .build();
    toolbar.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .child(&clamp)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    ));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&window));
}

pub(super) fn library_controls(state: &Rc<State>, group: &adw::PreferencesGroup) {
    // Sleek 3-item Adwaita bar: [Search] [Sort DropDown] [Filtrele ▾ Popover] — zero FlowBox ghost hitboxes!
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.add_css_class("duvar-toolbar-row");
    let search = gtk::SearchEntry::builder()
        .placeholder_text("İndirilenlerde ara…")
        .text(&state.config.borrow().behavior.library_query)
        .hexpand(true)
        .build();
    search.set_search_delay(180);
    let w = weak(state);
    search.connect_search_changed(move |e| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().behavior.library_query = e.text().chars().take(256).collect();
            s.save();
            s.render_library();
        }
    });
    bar.append(&search);

    let sort_values = ["downloaded", "resolution", "size", "title"];
    let sort_dd = gtk::DropDown::from_strings(&["Son indirilen", "Çözünürlük", "Boyut", "Başlık"]);
    sort_dd.set_selected(
        sort_values
            .iter()
            .position(|v| *v == state.config.borrow().behavior.library_sort)
            .unwrap_or(0) as u32,
    );
    sort_dd.update_property(&[gtk::accessible::Property::Label("Kütüphane sırası")]);
    let w = weak(state);
    sort_dd.connect_selected_notify(move |d| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().behavior.library_sort =
                sort_values[d.selected().min(3) as usize].into();
            s.save();
            s.render_library();
        }
    });
    bar.append(&sort_dd);

    // Compact filter popover holding Format, Orientation, Source dropdowns & Missing toggle
    let filter_btn = gtk::MenuButton::builder()
        .icon_name("duvar-filter-symbolic")
        .label("Süz")
        .tooltip_text("Biçim, yön, kaynak ve eksik dosya süzgeci")
        .valign(gtk::Align::Center)
        .build();
    let pop_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    pop_box.set_margin_top(10);
    pop_box.set_margin_bottom(10);
    pop_box.set_margin_start(12);
    pop_box.set_margin_end(12);
    pop_box.set_width_request(260);

    let add_labeled = |title: &str, widget: &gtk::Widget| {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let lbl = gtk::Label::new(Some(title));
        lbl.set_xalign(0.0);
        lbl.set_hexpand(true);
        lbl.add_css_class("caption");
        lbl.add_css_class("dim-label");
        row.append(&lbl);
        row.append(widget);
        pop_box.append(&row);
    };

    for (title, label, values, names, get, set) in [
        (
            "Biçim",
            "Kütüphane biçimi",
            vec!["", "jpg", "png", "webp"],
            vec!["Tümü", "JPEG", "PNG", "WebP"],
            (|c: &Config| c.behavior.library_format.clone()) as fn(&Config) -> String,
            (|c: &mut Config, v: String| c.behavior.library_format = v) as fn(&mut Config, String),
        ),
        (
            "Yön",
            "Kütüphane yönü",
            vec!["any", "landscape", "portrait", "square"],
            vec!["Tümü", "Yatay", "Dikey", "Kare"],
            |c| c.behavior.library_orientation.clone(),
            |c, v| c.behavior.library_orientation = v,
        ),
    ] {
        let d = gtk::DropDown::from_strings(&names);
        d.set_selected(
            values
                .iter()
                .position(|v| *v == get(&state.config.borrow()))
                .unwrap_or(0) as u32,
        );
        d.update_property(&[gtk::accessible::Property::Label(label)]);
        let w = weak(state);
        d.connect_selected_notify(move |d| {
            if let Some(s) = w.upgrade() {
                set(
                    &mut s.config.borrow_mut(),
                    values[d.selected().min(values.len() as u32 - 1) as usize].into(),
                );
                s.save();
                s.render_library();
            }
        });
        add_labeled(title, d.upcast_ref());
    }

    let names = std::iter::once("Tümü")
        .chain(crate::registry::SOURCES.iter().map(|s| s.id))
        .collect::<Vec<_>>();
    let source = gtk::DropDown::from_strings(&names);
    source.update_property(&[gtk::accessible::Property::Label("Kütüphane kaynağı")]);
    source.set_selected(
        names
            .iter()
            .position(|v| *v == state.config.borrow().behavior.library_source)
            .unwrap_or(0) as u32,
    );
    let w = weak(state);
    source.connect_selected_notify(move |d| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().behavior.library_source = if d.selected() == 0 {
                String::new()
            } else {
                crate::registry::SOURCES
                    [(d.selected() - 1).min(crate::registry::SOURCES.len() as u32 - 1) as usize]
                    .id
                    .into()
            };
            s.save();
            s.render_library();
        }
    });
    add_labeled("Kaynak", source.upcast_ref());

    pop_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let missing_sw = gtk::Switch::builder()
        .active(state.config.borrow().behavior.library_missing)
        .valign(gtk::Align::Center)
        .build();
    let w = weak(state);
    missing_sw.connect_active_notify(move |sw| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().behavior.library_missing = sw.is_active();
            s.save();
            s.render_library();
        }
    });
    add_labeled("Eksik dosyaları göster", missing_sw.upcast_ref());

    let pop = gtk::Popover::new();
    pop.set_child(Some(&pop_box));
    filter_btn.set_popover(Some(&pop));
    bar.append(&filter_btn);
    group.add(&bar);
}

pub(super) fn queue_controls(state: &Rc<State>, row: &adw::ExpanderRow) {
    let controls = transfer_ui::action_box();
    controls.set_max_children_per_line(4);
    controls.set_margin_top(6);
    controls.set_margin_bottom(6);
    controls.set_margin_start(8);
    controls.set_margin_end(8);
    for (title, mode) in [
        ("Duraklat / Devam et", 0),
        ("Tümünü iptal et", 1),
        ("Başarısızları tekrar dene", 2),
        ("Tamamlananları temizle", 3),
    ] {
        let b = gtk::Button::with_label(title);
        b.add_css_class("flat");
        let w = weak(state);
        b.connect_clicked(move |_| {
            if let Some(s) = w.upgrade() {
                match mode {
                    0 => {
                        let pause = !s.queue_paused.get();
                        s.queue_paused.set(pause);
                        let ids = s
                            .queue
                            .borrow()
                            .iter()
                            .filter(|r| {
                                if pause {
                                    crate::runtime::Phase::from_status(&r.status).pending()
                                } else {
                                    r.status == "paused"
                                }
                            })
                            .map(|r| r.id)
                            .collect::<Vec<_>>();
                        for id in ids {
                            if pause {
                                s.cancel_download(id, true);
                            } else {
                                s.start_download(id);
                            }
                        }
                    }
                    1 => {
                        let ids = s
                            .queue
                            .borrow()
                            .iter()
                            .filter(|r| {
                                crate::runtime::Phase::from_status(&r.status).pending()
                                    || r.status == "paused"
                            })
                            .map(|r| r.id)
                            .collect::<Vec<_>>();
                        for id in ids {
                            s.cancel_download(id, false);
                        }
                    }
                    2 => {
                        let ids = s
                            .queue
                            .borrow()
                            .iter()
                            .filter(|r| r.status == "failed")
                            .map(|r| r.id)
                            .collect::<Vec<_>>();
                        for id in ids {
                            s.start_download(id);
                        }
                    }
                    _ => s
                        .queue
                        .borrow_mut()
                        .retain(|r| !matches!(r.status.as_str(), "done" | "cancelled")),
                }
                s.queue_changed();
            }
        });
        controls.insert(&b, -1);
        if let Some(fc) = b.parent().and_downcast::<gtk::FlowBoxChild>() {
            fc.set_focusable(false);
        }
    }
    row.add_row(&controls);
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Kuyrukta ara…")
        .text(&state.config.borrow().behavior.queue_query)
        .build();
    search.set_search_delay(180);
    let w = weak(state);
    search.connect_search_changed(move |e| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().behavior.queue_query = e.text().chars().take(256).collect();
            s.save();
            s.queue_changed();
        }
    });
    row.add_row(&search);
    let status = adw::ComboRow::builder()
        .title("Kuyruk durumu")
        .model(&gtk::StringList::new(&[
            "Tümü",
            "Etkin",
            "Sırada",
            "Duraklatılan",
            "Başarısız",
            "Tamamlanan",
            "İptal edilen",
        ]))
        .build();
    let keys = [
        "all",
        "active",
        "queued",
        "paused",
        "failed",
        "done",
        "cancelled",
    ];
    status.set_selected(
        keys.iter()
            .position(|k| *k == state.config.borrow().behavior.queue_filter)
            .unwrap_or(0) as u32,
    );
    let w = weak(state);
    status.connect_selected_notify(move |r| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().behavior.queue_filter = keys[r.selected().min(6) as usize].into();
            s.save();
            s.queue_changed();
        }
    });
    row.add_row(&status);
}

pub(super) fn tag_details(state: &Rc<State>, content: &gtk::Box, item: &Wallpaper) {
    let heading = gtk::Label::new(Some("Etiketler"));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    content.append(&heading);
    let Some(tags) = &item.tags else {
        let empty = gtk::Label::new(Some("Kaynak etiket bilgisi sağlamıyor."));
        empty.set_xalign(0.0);
        empty.add_css_class("caption");
        empty.add_css_class("dim-label");
        content.append(&empty);
        return;
    };
    if tags.is_empty() {
        let empty = gtk::Label::new(Some("Etiket listesi boş."));
        empty.set_xalign(0.0);
        empty.add_css_class("caption");
        empty.add_css_class("dim-label");
        content.append(&empty);
        return;
    }
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Etiketlerde ara…")
        .build();
    content.append(&search);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    for tag in tags.iter().take(128) {
        let r = adw::ActionRow::builder()
            .title(tag)
            .use_markup(false)
            .build();
        for (label, icon, block) in [
            ("Bu etiketi ara", "system-search-symbolic", false),
            ("Bu etiketi engelle", "list-remove-symbolic", true),
        ] {
            let b = gtk::Button::from_icon_name(icon);
            b.set_tooltip_text(Some(label));
            b.set_valign(gtk::Align::Center);
            b.add_css_class("flat");
            b.update_property(&[gtk::accessible::Property::Label(label)]);
            let w = weak(state);
            let tag = tag.clone();
            b.connect_clicked(move |_| {
                if let Some(s) = w.upgrade() {
                    if block {
                        let mut c = s.config.borrow_mut();
                        let mut parts = c
                            .policy
                            .exclude_tags
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>();
                        if !parts.iter().any(|p| p.eq_ignore_ascii_case(&tag)) {
                            parts.push(tag.clone());
                        }
                        c.policy.exclude_tags = parts.join(", ");
                        c.policy.exclude_enabled = true;
                        drop(c);
                        s.changed();
                        s.toast("Etiket engelleme listesine eklendi.");
                    } else {
                        s.search.set_text(&tag);
                    }
                }
            });
            r.add_suffix(&b);
        }
        list.append(&r);
    }
    let listweak = list.downgrade();
    search.connect_search_changed(move |e| {
        if let Some(list) = listweak.upgrade() {
            let q = crate::catalog::search_key(&e.text());
            let mut ch = list.first_child();
            while let Some(r) = ch {
                ch = r.next_sibling();
                if let Some(row) = r.downcast_ref::<adw::ActionRow>() {
                    row.set_visible(crate::catalog::search_key(&row.title()).contains(&q));
                }
            }
        }
    });
    content.append(&list);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview_zoom(
    state: &Rc<State>,
    dialog: &adw::Dialog,
    header: &adw::HeaderBar,
    picture: &gtk::Picture,
    scroll: &gtk::ScrolledWindow,
    toggle: &gtk::ToggleButton,
    dimensions: Rc<Cell<(i32, i32)>>,
    quality: &gtk::Label,
    split: &adw::OverlaySplitView,
    item: Wallpaper,
    on_navigate: Option<Rc<dyn Fn(i32)>>,
) {
    let current = Rc::new(Cell::new(1.0f64));
    let pointer = Rc::new(Cell::new((-1.0f64, -1.0f64)));

    let p_weak = picture.downgrade();
    let sc_weak = scroll.downgrade();
    let q_weak = quality.downgrade();
    let t_weak = toggle.downgrade();
    let cur_apply = current.clone();
    let dims_apply = dimensions.clone();

    let fit_scale = {
        let sc = scroll.downgrade();
        let dims = dimensions.clone();
        Rc::new(move || -> f64 {
            let (w, h) = dims.get();
            if w <= 0 || h <= 0 {
                return 1.0;
            }
            let Some(sc) = sc.upgrade() else {
                return 1.0;
            };
            let vw = sc.width().max(1) as f64;
            let vh = sc.height().max(1) as f64;
            (vw / w as f64).min(vh / h as f64).min(1.0)
        })
    };

    // Anchor zoom around viewport coordinates (vx, vy) — defaults to viewport center.
    type ZoomFn = Rc<dyn Fn(f64, Option<(f64, f64)>)>;
    let set_zoom_at: ZoomFn = Rc::new(move |f, anchor| {
        let (w, h) = dims_apply.get();
        if w <= 0 || h <= 0 {
            return;
        }
        let f = f.clamp(0.1, 6.0);
        if let (Some(p), Some(sc), Some(q), Some(t)) = (
            p_weak.upgrade(),
            sc_weak.upgrade(),
            q_weak.upgrade(),
            t_weak.upgrade(),
        ) {
            let hadj = sc.hadjustment();
            let vadj = sc.vadjustment();
            let vw = sc.width().max(1) as f64;
            let vh = sc.height().max(1) as f64;
            let (ax, ay) = anchor
                .filter(|(x, y)| *x >= 0.0 && *y >= 0.0)
                .unwrap_or((vw / 2.0, vh / 2.0));
            let was_zoomed = t.is_active();
            let (norm_x, norm_y) = if was_zoomed && hadj.upper() > 1.0 && vadj.upper() > 1.0 {
                (
                    (hadj.value() + ax) / hadj.upper(),
                    (vadj.value() + ay) / vadj.upper(),
                )
            } else {
                ((ax / vw).clamp(0.0, 1.0), (ay / vh).clamp(0.0, 1.0))
            };
            let new_w = (w as f64 * f).round().max(1.0) as i32;
            let new_h = (h as f64 * f).round().max(1.0) as i32;
            cur_apply.set(f);
            if !t.is_active() {
                t.set_active(true);
            }
            p.set_size_request(new_w, new_h);
            p.set_can_shrink(false);
            sc.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
            sc.set_cursor_from_name(Some("grab"));
            q.set_text(&format!(
                "Özgün dosya · {w} × {h} piksel · %{:.0}",
                f * 100.0
            ));
            let target_x = (norm_x * new_w as f64 - ax).max(0.0);
            let target_y = (norm_y * new_h as f64 - ay).max(0.0);
            glib::idle_add_local_once(move || {
                let max_x = (hadj.upper() - hadj.page_size()).max(0.0);
                let max_y = (vadj.upper() - vadj.page_size()).max(0.0);
                hadj.set_value(target_x.clamp(0.0, max_x));
                vadj.set_value(target_y.clamp(0.0, max_y));
            });
        }
    });

    let reset_fit: Rc<dyn Fn()> = {
        let p = picture.downgrade();
        let sc = scroll.downgrade();
        let q = quality.downgrade();
        let t = toggle.downgrade();
        let c = current.clone();
        let dims = dimensions.clone();
        Rc::new(move || {
            c.set(1.0);
            if let Some(t) = t.upgrade()
                && t.is_active()
            {
                t.set_active(false);
            }
            if let (Some(p), Some(sc), Some(q)) = (p.upgrade(), sc.upgrade(), q.upgrade()) {
                p.set_size_request(-1, -1);
                p.set_can_shrink(true);
                sc.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Never);
                sc.set_cursor_from_name(None);
                let (w, h) = dims.get();
                if w > 0 && h > 0 {
                    q.set_text(&format!(
                        "Özgün dosya · {w} × {h} piksel · Ekrana sığdırılıyor"
                    ));
                }
            }
        })
    };

    // Linked zoom pill controls in HeaderBar: [-] [%100] [+] [Fit]
    let zoom_group = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    zoom_group.add_css_class("linked");

    let out_btn = gtk::Button::from_icon_name("zoom-out-symbolic");
    out_btn.set_tooltip_text(Some("Uzaklaştır"));
    {
        let set_z = set_zoom_at.clone();
        let c = current.clone();
        let t = toggle.downgrade();
        let fs = fit_scale.clone();
        out_btn.connect_clicked(move |_| {
            let base = if t.upgrade().is_some_and(|b| b.is_active()) {
                c.get()
            } else {
                fs()
            };
            let next = (base * 0.8).clamp(0.1, 6.0);
            c.set(next);
            set_z(next, None);
        });
    }
    zoom_group.append(&out_btn);
    zoom_group.append(toggle);

    let in_btn = gtk::Button::from_icon_name("zoom-in-symbolic");
    in_btn.set_tooltip_text(Some("Yakınlaştır"));
    {
        let set_z = set_zoom_at.clone();
        let c = current.clone();
        in_btn.connect_clicked(move |_| {
            let next = (c.get() * 1.25).clamp(0.1, 6.0);
            c.set(next);
            set_z(next, None);
        });
    }
    zoom_group.append(&in_btn);

    let fit = gtk::Button::from_icon_name("zoom-fit-best-symbolic");
    fit.set_tooltip_text(Some("Ekrana sığdır (0)"));
    {
        let rf = reset_fit.clone();
        fit.connect_clicked(move |_| rf());
    }
    zoom_group.append(&fit);
    header.pack_start(&zoom_group);

    // Track pointer position inside the viewport for cursor-anchored wheel zoom
    let motion = gtk::EventControllerMotion::new();
    let ptr_move = pointer.clone();
    motion.connect_motion(move |_, x, y| {
        ptr_move.set((x, y));
    });
    let ptr_leave = pointer.clone();
    motion.connect_leave(move |_| {
        ptr_leave.set((-1.0, -1.0));
    });
    scroll.add_controller(motion);

    // Plain mouse wheel (and Ctrl+wheel) zooms smoothly right under the cursor!
    let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    {
        let c = current.clone();
        let set_z = set_zoom_at.clone();
        let rf = reset_fit.clone();
        let ptr = pointer.clone();
        let t = toggle.downgrade();
        let fs = fit_scale.clone();
        wheel.connect_scroll(move |_, _, dy| {
            if dy.abs() < 0.001 {
                return glib::Propagation::Proceed;
            }
            let fit_f = fs();
            let active = t.upgrade().is_some_and(|b| b.is_active());
            let base = if active { c.get() } else { fit_f };
            let step = if dy < 0.0 { 1.18 } else { 1.0 / 1.18 };
            let next = (base * step).clamp(0.1, 6.0);
            if dy > 0.0 && next <= fit_f * 1.02 {
                rf();
            } else {
                let anchor = ptr.get();
                set_z(next, Some(anchor));
            }
            glib::Propagation::Stop
        });
    }
    scroll.add_controller(wheel);

    // Left-click drag to pan smoothly across the zoomed image
    let drag = gtk::GestureDrag::new();
    drag.set_button(1);
    let drag_start = Rc::new(Cell::new((0.0f64, 0.0f64)));
    let dragging = Rc::new(Cell::new(false));
    {
        let sc = scroll.downgrade();
        let t = toggle.downgrade();
        let ds = drag_start.clone();
        let dg = dragging.clone();
        drag.connect_drag_begin(move |_, _, _| {
            if t.upgrade().is_some_and(|b| b.is_active())
                && let Some(sc) = sc.upgrade()
            {
                ds.set((sc.hadjustment().value(), sc.vadjustment().value()));
                dg.set(true);
                sc.set_cursor_from_name(Some("grabbing"));
            }
        });
    }
    {
        let sc = scroll.downgrade();
        let ds = drag_start.clone();
        let dg = dragging.clone();
        drag.connect_drag_update(move |_, ox, oy| {
            if dg.get()
                && let Some(sc) = sc.upgrade()
            {
                let (sx, sy) = ds.get();
                let hadj = sc.hadjustment();
                let vadj = sc.vadjustment();
                let max_x = (hadj.upper() - hadj.page_size()).max(0.0);
                let max_y = (vadj.upper() - vadj.page_size()).max(0.0);
                hadj.set_value((sx - ox).clamp(0.0, max_x));
                vadj.set_value((sy - oy).clamp(0.0, max_y));
            }
        });
    }
    {
        let sc = scroll.downgrade();
        let t = toggle.downgrade();
        let dg = dragging;
        drag.connect_drag_end(move |_, _, _| {
            if dg.replace(false)
                && let Some(sc) = sc.upgrade()
            {
                sc.set_cursor_from_name(if t.upgrade().is_some_and(|b| b.is_active()) {
                    Some("grab")
                } else {
                    None
                });
            }
        });
    }
    scroll.add_controller(drag);

    // Trackpad pinch-to-zoom
    let pinch = gtk::GestureZoom::new();
    let pinch_base = Rc::new(Cell::new(1.0f64));
    {
        let pb = pinch_base.clone();
        let c = current.clone();
        let t = toggle.downgrade();
        let fs = fit_scale.clone();
        pinch.connect_begin(move |_, _| {
            pb.set(if t.upgrade().is_some_and(|b| b.is_active()) {
                c.get()
            } else {
                fs()
            });
        });
    }
    {
        let pb = pinch_base;
        let set_z = set_zoom_at.clone();
        let rf = reset_fit.clone();
        let fs = fit_scale;
        pinch.connect_scale_changed(move |_, scale| {
            let next = (pb.get() * scale).clamp(0.1, 6.0);
            if next <= fs() * 1.01 {
                rf();
            } else {
                set_z(next, None);
            }
        });
    }
    scroll.add_controller(pinch);

    // Left double-click toggles Fit <-> 100% anchored at click position
    let dbl = gtk::GestureClick::new();
    dbl.set_button(1);
    {
        let t = toggle.downgrade();
        let set_z = set_zoom_at.clone();
        let rf = reset_fit.clone();
        let ptr = pointer;
        dbl.connect_pressed(move |_, n, _, _| {
            if n == 2 {
                if t.upgrade().is_some_and(|b| b.is_active()) {
                    rf();
                } else {
                    set_z(1.0, Some(ptr.get()));
                }
            }
        });
    }
    scroll.add_controller(dbl);

    // Right-click native context popover with instant wallpaper & zoom actions
    let rclick = gtk::GestureClick::new();
    rclick.set_button(3);
    {
        let w = weak(state);
        let sc = scroll.downgrade();
        let sp = split.downgrade();
        let set_z = set_zoom_at.clone();
        let rf = reset_fit.clone();
        let it = item.clone();
        rclick.connect_pressed(move |_, _, x, y| {
            let (Some(s), Some(sc)) = (w.upgrade(), sc.upgrade()) else {
                return;
            };
            let pop = gtk::Popover::new();
            pop.set_parent(&sc);
            pop.set_has_arrow(false);
            pop.set_pointing_to(Some(&gdk::Rectangle::new(
                x.round() as i32,
                y.round() as i32,
                1,
                1,
            )));
            let menu = gtk::Box::new(gtk::Orientation::Vertical, 2);
            menu.set_margin_top(4);
            menu.set_margin_bottom(4);
            menu.set_margin_start(4);
            menu.set_margin_end(4);
            let add_item = |label: &str, icon: &str, cb: Box<dyn Fn()>| {
                let b = gtk::Button::new();
                b.add_css_class("flat");
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                row.append(&gtk::Image::from_icon_name(icon));
                let l = gtk::Label::new(Some(label));
                l.set_xalign(0.0);
                l.set_hexpand(true);
                row.append(&l);
                b.set_child(Some(&row));
                let p = pop.downgrade();
                b.connect_clicked(move |_| {
                    if let Some(p) = p.upgrade() {
                        p.popdown();
                    }
                    cb();
                });
                menu.append(&b);
            };
            let phase = s.phase(&it);
            let acts = s.library.borrow().actions(&it, phase);
            if acts.apply || acts.download {
                let s1 = s.clone();
                let it1 = it.clone();
                let loc = acts.apply;
                add_item(
                    "Açık tema için uygula",
                    "weather-clear-symbolic",
                    Box::new(move || {
                        transfer_ui::apply_for_scope(&s1, &it1, "light", loc);
                    }),
                );
                let s2 = s.clone();
                let it2 = it.clone();
                add_item(
                    "Koyu tema için uygula",
                    "weather-clear-night-symbolic",
                    Box::new(move || {
                        transfer_ui::apply_for_scope(&s2, &it2, "dark", loc);
                    }),
                );
            }
            if acts.download {
                let s3 = s.clone();
                let it3 = it.clone();
                add_item(
                    "İndir",
                    "folder-download-symbolic",
                    Box::new(move || {
                        s3.download(it3.clone(), None);
                    }),
                );
            }
            if acts.trash {
                let s4 = s.clone();
                let it4 = it.clone();
                add_item(
                    "Çöp kutusuna taşı",
                    "user-trash-symbolic",
                    Box::new(move || {
                        s4.trash(&it4);
                    }),
                );
            }
            let is_fav = s.config.borrow().favorites.iter().any(|f| f.id == it.id);
            {
                let s5 = s.clone();
                let it5 = it.clone();
                add_item(
                    if is_fav {
                        "Favorilerden çıkar"
                    } else {
                        "Favorilere ekle"
                    },
                    if is_fav {
                        "starred-symbolic"
                    } else {
                        "non-starred-symbolic"
                    },
                    Box::new(move || {
                        s5.favorite(&it5);
                        s5.refresh_cards(Some(&it5.id), true);
                    }),
                );
            }
            menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            {
                let rf1 = rf.clone();
                add_item(
                    "Ekrana sığdır (0)",
                    "zoom-fit-best-symbolic",
                    Box::new(move || rf1()),
                );
            }
            {
                let sz1 = set_z.clone();
                add_item(
                    "%100 gerçek piksel (1)",
                    "zoom-original-symbolic",
                    Box::new(move || sz1(1.0, None)),
                );
            }
            {
                let sp1 = sp.clone();
                add_item(
                    "Görsel bilgileri ve etiketler (I)",
                    "dialog-information-symbolic",
                    Box::new(move || {
                        if let Some(sp) = sp1.upgrade() {
                            sp.set_show_sidebar(!sp.shows_sidebar());
                        }
                    }),
                );
            }
            if acts.open {
                menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
                let s6 = s.clone();
                let it6 = it.clone();
                add_item(
                    "Görüntüleyicide aç",
                    "image-x-generic-symbolic",
                    Box::new(move || {
                        let local = s6.library.borrow().localize(&it6);
                        let _ = gio::AppInfo::launch_default_for_uri(
                            &local.image,
                            None::<&gio::AppLaunchContext>,
                        );
                    }),
                );
                let s7 = s.clone();
                let it7 = it.clone();
                add_item(
                    "Klasörü aç",
                    "folder-open-symbolic",
                    Box::new(move || {
                        let local = s7.library.borrow().localize(&it7);
                        if let Some(parent) = gio::File::for_uri(&local.image).parent() {
                            let _ = gio::AppInfo::launch_default_for_uri(
                                &parent.uri(),
                                None::<&gio::AppLaunchContext>,
                            );
                        }
                    }),
                );
            }
            pop.set_child(Some(&menu));
            pop.connect_closed(|p| {
                p.unparent();
            });
            pop.popup();
        });
    }
    scroll.add_controller(rclick);

    // Keyboard shortcuts inside preview: +, -, 0, 1, I, F, Left/Right navigation
    let keys = gtk::EventControllerKey::new();
    keys.set_name(Some("duvar-preview-zoom"));
    let setkey = set_zoom_at;
    let rfkey = reset_fit;
    let c = current;
    let sp_key = split.downgrade();
    let w_key = weak(state);
    keys.connect_key_pressed(move |_, key, _, _| {
        match key {
            gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => {
                c.set((c.get() * 1.25).min(6.0));
                setkey(c.get(), None);
            }
            gdk::Key::minus | gdk::Key::KP_Subtract => {
                c.set((c.get() * 0.8).max(0.1));
                setkey(c.get(), None);
            }
            gdk::Key::_0 => rfkey(),
            gdk::Key::_1 => {
                c.set(1.0);
                setkey(1.0, None);
            }
            gdk::Key::i | gdk::Key::I => {
                if let Some(sp) = sp_key.upgrade() {
                    sp.set_show_sidebar(!sp.shows_sidebar());
                }
            }
            gdk::Key::f | gdk::Key::F => {
                if let Some(s) = w_key.upgrade() {
                    s.favorite(&item);
                    s.refresh_cards(Some(&item.id), true);
                }
            }
            gdk::Key::Left => {
                if let Some(nav) = &on_navigate {
                    nav(-1);
                } else {
                    return glib::Propagation::Proceed;
                }
            }
            gdk::Key::Right => {
                if let Some(nav) = &on_navigate {
                    nav(1);
                } else {
                    return glib::Propagation::Proceed;
                }
            }
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    dialog.add_controller(keys);
}

#[allow(deprecated, reason = "GNOME supplies PreferencesWindow")]
pub(super) fn keyboard(
    state: &Rc<State>,
    window: &adw::PreferencesWindow,
    pages: [adw::PreferencesPage; 4],
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_name(Some("duvar-navigation"));
    let w = weak(state);
    let win = window.downgrade();
    controller.connect_key_pressed(move |_, key, _, mods| {
        if let Some(s) = w.upgrade() {
            if mods.contains(gdk::ModifierType::CONTROL_MASK) {
                if key == gdk::Key::f {
                    if let Some(win) = win.upgrade() {
                        win.set_visible_page(&pages[0]);
                    }
                    s.search.grab_focus();
                    return glib::Propagation::Stop;
                }
                let idx = match key {
                    gdk::Key::_1 => Some(0),
                    gdk::Key::_2 => Some(1),
                    gdk::Key::_3 => Some(2),
                    gdk::Key::_4 => Some(3),
                    _ => None,
                };
                if let (Some(i), Some(win)) = (idx, win.upgrade()) {
                    win.set_visible_page(&pages[i]);
                    return glib::Propagation::Stop;
                }
            }
            if key == gdk::Key::Escape && s.search.has_focus() && !s.search.text().is_empty() {
                s.search.set_text("");
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    window.add_controller(controller);
}

pub(super) fn maintenance(state: &Rc<State>, page: &adw::PreferencesPage) {
    let storage = adw::PreferencesGroup::builder()
        .title("Önbellek ve bakım")
        .description("256 MiB sınırında küçük resim ve özgün önbelleği. Uygulama kapalıyken arka plan işlemi çalışmaz.")
        .build();
    let cache_exp = adw::ExpanderRow::builder()
        .title("Önbellek yönetimi")
        .subtitle("İstatistikleri gör veya önbellek bölümlerini temizle")
        .build();
    for (title, part) in [
        ("Önbellek istatistikleri", "stats"),
        ("Küçük resim önbelleğini temizle", "thumbnails"),
        ("Özgün önbelleği temizle", "originals"),
        ("Hassas önbelleği temizle", "sensitive"),
    ] {
        let row = adw::ActionRow::builder()
            .title(title)
            .activatable(true)
            .build();
        let w = weak(state);
        row.connect_activated(move |_| {
            if let Some(s) = w.upgrade() {
                s.pool.push(Job::Cache(part.into()));
                s.ensure_poll();
            }
        });
        cache_exp.add_row(&row);
    }
    storage.add(&cache_exp);

    let export = adw::ActionRow::builder()
        .title("Tanı olaylarını JSON olarak kaydet")
        .subtitle("Son 256 olay; görseller ve API anahtarları içermez.")
        .activatable(true)
        .build();
    export.add_suffix(&gtk::Image::from_icon_name("document-save-symbolic"));
    let w = weak(state);
    export.connect_activated(move |_| {
        if let Some(s) = w.upgrade() {
            let path = crate::store::state_dir()
                .join(format!("diagnostics-{}.json", crate::runtime::now()));
            s.pool.push(Job::Export(path));
            s.ensure_poll();
        }
    });
    storage.add(&export);

    let shortcuts = adw::ActionRow::builder()
        .title("Klavye ve fare kısayolları")
        .subtitle("Ctrl+F: ara · Ctrl+1–4: sekmeler · Önizleme: tekerlek yakınlaştır · sol sürükle kaydır · sağ tık menü · ←/→ gez · I: bilgiler · 0/1: sığdır/%100")
        .build();
    storage.add(&shortcuts);
    page.add(&storage);
}

fn extra_filters(state: &Rc<State>, group: &adw::PreferencesGroup, _page: &adw::PreferencesPage) {
    let syncing = Rc::new(Cell::new(false));
    let debounce: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let schedule_apply = {
        let deb = debounce.clone();
        let w = weak(state);
        Rc::new(move || {
            if let Some(id) = deb.borrow_mut().take() {
                id.remove();
            }
            let w2 = w.clone();
            let deb2 = deb.clone();
            let id = glib::timeout_add_local_once(Duration::from_millis(450), move || {
                deb2.borrow_mut().take();
                if let Some(s) = w2.upgrade() {
                    s.changed();
                }
            });
            deb.borrow_mut().replace(id);
        })
    };

    let cur_w = state.config.borrow().policy.minimum_width;
    let cur_h = state.config.borrow().policy.minimum_height;
    let minimum = adw::ComboRow::builder()
        .title("Minimum çözünürlük")
        .model(&gtk::StringList::new(&[
            "Sınır yok",
            "1920 × 1080 (FHD)",
            "2560 × 1440 (QHD)",
            "3840 × 2160 (4K UHD)",
            "Özel",
        ]))
        .selected(match (cur_w, cur_h) {
            (0, 0) => 0,
            (1920, 1080) => 1,
            (2560, 1440) => 2,
            (3840, 2160) => 3,
            _ => 4,
        })
        .build();
    let custom_w = adw::SpinRow::with_range(0.0, 16384.0, 10.0);
    custom_w.set_title("Özel minimum genişlik");
    custom_w.set_value(cur_w as f64);
    let custom_h = adw::SpinRow::with_range(0.0, 16384.0, 10.0);
    custom_h.set_title("Özel minimum yükseklik");
    custom_h.set_value(cur_h as f64);

    {
        let w = weak(state);
        let cw = custom_w.downgrade();
        let ch = custom_h.downgrade();
        let sync = syncing.clone();
        minimum.connect_selected_notify(move |r| {
            if sync.get() || r.selected() == 4 {
                return;
            }
            if let Some(s) = w.upgrade() {
                let (a, b) = [(0, 0), (1920, 1080), (2560, 1440), (3840, 2160)]
                    [r.selected().min(3) as usize];
                sync.set(true);
                if let Some(cw) = cw.upgrade() {
                    cw.set_value(a as f64);
                }
                if let Some(ch) = ch.upgrade() {
                    ch.set_value(b as f64);
                }
                sync.set(false);
                {
                    let mut c = s.config.borrow_mut();
                    c.policy.minimum_width = a;
                    c.policy.minimum_height = b;
                }
                s.changed();
            }
        });
    }

    for (row, is_width) in [(&custom_w, true), (&custom_h, false)] {
        let w = weak(state);
        let min_row = minimum.downgrade();
        let sync = syncing.clone();
        let sched = schedule_apply.clone();
        row.connect_value_notify(move |r| {
            if sync.get() {
                return;
            }
            if let Some(s) = w.upgrade() {
                let (mw, mh) = {
                    let mut c = s.config.borrow_mut();
                    if is_width {
                        c.policy.minimum_width = r.value() as u32;
                    } else {
                        c.policy.minimum_height = r.value() as u32;
                    }
                    (c.policy.minimum_width, c.policy.minimum_height)
                };
                if let Some(m) = min_row.upgrade() {
                    let preset = match (mw, mh) {
                        (0, 0) => 0,
                        (1920, 1080) => 1,
                        (2560, 1440) => 2,
                        (3840, 2160) => 3,
                        _ => 4,
                    };
                    if m.selected() != preset {
                        sync.set(true);
                        m.set_selected(preset);
                        sync.set(false);
                    }
                }
                sched();
            }
        });
    }

    group.add(&minimum);
    let custom_exp = adw::ExpanderRow::builder()
        .title("Özel piksel sınırı")
        .subtitle("İsteğe bağlı minimum genişlik ve yükseklik; değiştirince otomatik uygulanır.")
        .build();
    custom_exp.add_row(&custom_w);
    custom_exp.add_row(&custom_h);
    group.add(&custom_exp);

    let orientation = adw::ComboRow::builder()
        .title("Görsel yönü")
        .model(&gtk::StringList::new(&[
            "Tüm yönler",
            "Yatay",
            "Dikey",
            "Kare",
        ]))
        .selected(
            ["any", "landscape", "portrait", "square"]
                .iter()
                .position(|v| *v == state.config.borrow().policy.orientation)
                .unwrap_or(0) as u32,
        )
        .build();
    let w = weak(state);
    orientation.connect_selected_notify(move |r| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().policy.orientation =
                ["any", "landscape", "portrait", "square"][r.selected().min(3) as usize].into();
            s.changed();
        }
    });
    group.add(&orientation);

    let formats = adw::ExpanderRow::builder()
        .title("Dosya biçimleri")
        .subtitle("Seçim yoksa desteklenen tüm biçimler gösterilir.")
        .build();
    for name in ["jpg", "png", "webp"] {
        let r = adw::SwitchRow::builder()
            .title(name.to_uppercase())
            .active(
                state
                    .config
                    .borrow()
                    .policy
                    .formats
                    .iter()
                    .any(|f| f == name),
            )
            .build();
        let w = weak(state);
        r.connect_active_notify(move |r| {
            if let Some(s) = w.upgrade() {
                let mut c = s.config.borrow_mut();
                c.policy.formats.retain(|f| f != name);
                if r.is_active() {
                    c.policy.formats.push(name.into());
                }
                drop(c);
                s.changed();
            }
        });
        formats.add_row(&r);
    }
    group.add(&formats);

    let missing = adw::SwitchRow::builder()
        .title("Boyutu bilinmeyenleri göster")
        .subtitle("Ön boyut verisi sunmayan kaynaklardaki görselleri boyut filtresinde korur.")
        .active(state.config.borrow().policy.allow_missing_dimensions)
        .build();
    let w = weak(state);
    missing.connect_active_notify(move |r| {
        if let Some(s) = w.upgrade() {
            s.config.borrow_mut().policy.allow_missing_dimensions = r.is_active();
            s.changed();
        }
    });
    group.add(&missing);
}

#[allow(deprecated, reason = "GNOME 51 PreferencesWindow host")]
pub(super) fn explore_menu(
    state: &Rc<State>,
    filters_page: &adw::PreferencesPage,
) -> gtk::MenuButton {
    let menu_btn = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Keşfet araçları ve geçmiş")
        .valign(gtk::Align::Center)
        .build();
    menu_btn.add_css_class("flat");
    let popover = gtk::Popover::new();
    menu_btn.set_popover(Some(&popover));
    let w = weak(state);
    let fp = filters_page.downgrade();
    menu_btn.connect_active_notify(move |button| {
        if !button.is_active() {
            return;
        }
        let Some(s) = w.upgrade() else {
            return;
        };
        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
        box_.set_margin_top(6);
        box_.set_margin_bottom(6);
        box_.set_margin_start(6);
        box_.set_margin_end(6);
        box_.set_width_request(240);

        let pop_weak = button.popover().map(|p| p.downgrade());
        let add_btn = |label: &str, icon: &str, cb: Box<dyn Fn()>| {
            let b = gtk::Button::new();
            b.add_css_class("flat");
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.append(&gtk::Image::from_icon_name(icon));
            let l = gtk::Label::new(Some(label));
            l.set_xalign(0.0);
            l.set_hexpand(true);
            row.append(&l);
            b.set_child(Some(&row));
            let pw = pop_weak.clone();
            b.connect_clicked(move |_| {
                if let Some(p) = pw.as_ref().and_then(glib::WeakRef::upgrade) {
                    p.popdown();
                }
                cb();
            });
            box_.append(&b);
        };

        {
            let s1 = s.clone();
            add_btn(
                "Yeni arama başlat",
                "view-refresh-symbolic",
                Box::new(move || s1.load()),
            );
        }
        {
            let s2 = s.clone();
            add_btn(
                "Sonuçları koruyarak yenile",
                "media-playlist-repeat-symbolic",
                Box::new(move || {
                    s2.current().cancel.store(true, Ordering::Relaxed);
                    s2.pool.cancel_queued();
                    let number = s2.current().number + 1;
                    s2.request.replace(Request {
                        number,
                        cancel: Arc::new(AtomicBool::new(false)),
                    });
                    if s2.config.borrow().gallery_order == "random" {
                        s2.sort_gallery();
                    }
                    for c in s2.cursors.borrow_mut().values_mut() {
                        *c = Cursor::default();
                    }
                    s2.errors.borrow_mut().clear();
                    s2.load_more(false);
                }),
            );
        }
        {
            let s3 = s.clone();
            add_btn(
                "Kaynak durumları",
                "network-server-symbolic",
                Box::new(move || {
                    show_source_status(&s3);
                }),
            );
        }
        {
            let s4 = s.clone();
            add_btn(
                "Kaynakları yönet",
                "folder-remote-symbolic",
                Box::new(move || {
                    sources(&s4);
                }),
            );
        }
        {
            let s5 = s.clone();
            let fp1 = fp.clone();
            add_btn(
                "Filtreler",
                "duvar-filter-symbolic",
                Box::new(move || {
                    if let (Some(win), Some(page)) = (s5.window.upgrade(), fp1.upgrade()) {
                        win.set_visible_page(&page);
                    }
                }),
            );
        }

        let c = s.config.borrow();
        if c.behavior.history_enabled && !c.behavior.search_history.is_empty() {
            box_.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            let hdr = gtk::Label::new(Some("Son aramalar"));
            hdr.set_xalign(0.0);
            hdr.set_margin_start(8);
            hdr.set_margin_top(4);
            hdr.add_css_class("caption");
            hdr.add_css_class("dim-label");
            box_.append(&hdr);
            for q in c.behavior.search_history.iter().take(6) {
                let entry = s.search.downgrade();
                let q_text = q.clone();
                add_btn(
                    q,
                    "document-open-recent-symbolic",
                    Box::new(move || {
                        if let Some(e) = entry.upgrade() {
                            e.set_text(&q_text);
                        }
                    }),
                );
            }
        }
        if let Some(p) = button.popover().and_downcast::<gtk::Popover>() {
            p.set_child(Some(&box_));
        }
    });
    menu_btn
}

fn show_source_status(s: &Rc<State>) {
    let Some(window) = s.window.upgrade() else {
        return;
    };
    let dialog = adw::Dialog::builder()
        .title("Kaynak durumları")
        .content_width(600)
        .content_height(440)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 12);
    wrap.set_margin_top(12);
    wrap.set_margin_bottom(16);
    wrap.set_margin_start(16);
    wrap.set_margin_end(16);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    for (name, cursor) in s.cursors.borrow().iter() {
        let row = adw::ActionRow::builder()
            .title(name)
            .subtitle(if cursor.loading {
                "Yükleniyor…"
            } else if cursor.failed {
                "Kaynak yanıt vermedi; diğer sonuçlar korunuyor"
            } else if cursor.exhausted {
                "Tüm sonuçlar yüklendi"
            } else {
                "Hazır"
            })
            .build();
        if cursor.failed {
            let b = gtk::Button::with_label("Tekrar dene");
            b.set_valign(gtk::Align::Center);
            let w = weak(s);
            let name = name.clone();
            b.connect_clicked(move |_| {
                if let Some(s) = w.upgrade() {
                    let page = s
                        .cursors
                        .borrow_mut()
                        .get_mut(&name)
                        .and_then(|c| c.begin(true));
                    if let Some(page) = page {
                        let key = if name == "Wallhaven" {
                            s.api_key.borrow().clone()
                        } else {
                            String::new()
                        };
                        s.pool.push(Job::Search(
                            s.current(),
                            name.clone(),
                            s.config.borrow().clone(),
                            page,
                            key,
                        ));
                        s.ensure_poll();
                    }
                }
            });
            row.add_suffix(&b);
        }
        list.append(&row);
    }
    wrap.append(&list);
    if !s.errors.borrow().is_empty() {
        let text = gtk::Label::new(Some(&s.errors.borrow().join("\n")));
        text.set_wrap(true);
        text.set_xalign(0.0);
        text.set_selectable(true);
        text.add_css_class("caption");
        text.add_css_class("dim-label");
        wrap.append(&text);
    }
    toolbar.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .child(&wrap)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    ));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&window));
}
