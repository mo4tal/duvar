use super::*;
use crate::catalog::{self, Topic};
use std::collections::BTreeSet;

pub(super) fn summary(state: &Rc<State>) {
    while let Some(child) = state.topic_bar.first_child() {
        state.topic_bar.remove(&child);
    }
    let cfg = state.config.borrow();
    for (sensitive, ids) in [
        (false, &cfg.policy.general_topics),
        (true, &cfg.policy.sensitive_topics),
    ] {
        for id in ids.iter().take(10) {
            if let Some(t) = catalog::all(sensitive).iter().find(|t| &t.id == id) {
                let b = gtk::Button::with_label(&format!("{} ×", t.title));
                b.set_tooltip_text(Some("Bu konu filtresini kaldır"));
                b.add_css_class("pill");
                let w = weak(state);
                let id = id.clone();
                b.connect_clicked(move |_| {
                    if let Some(s) = w.upgrade() {
                        if sensitive {
                            s.config
                                .borrow_mut()
                                .policy
                                .sensitive_topics
                                .retain(|t| t != &id);
                        } else {
                            s.config
                                .borrow_mut()
                                .policy
                                .general_topics
                                .retain(|t| t != &id);
                        }
                        s.changed();
                        summary(&s);
                    }
                });
                state.topic_bar.insert(&b, -1);
                if let Some(fc) = b.parent().and_downcast::<gtk::FlowBoxChild>() {
                    fc.set_focusable(false);
                }
            }
        }
    }
    drop(cfg);
    state
        .topic_bar
        .set_visible(state.topic_bar.first_child().is_some());
    if let Some(row) = state.topic_summary.upgrade() {
        let cfg = state.config.borrow();
        row.set_subtitle(&format!(
            "Genel: {}/100 · NSFW: {}/100 · Seçim yoksa tüm konular",
            cfg.policy.general_topics.len(),
            cfg.policy.sensitive_topics.len()
        ));
    }
}
pub(super) fn show(state: &Rc<State>) {
    let Some(window) = state.window.upgrade() else {
        return;
    };
    let draft = Rc::new(RefCell::new((
        state
            .config
            .borrow()
            .policy
            .general_topics
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        state
            .config
            .borrow()
            .policy
            .sensitive_topics
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
    )));
    let initial = draft.borrow().clone();
    let dialog = adw::Dialog::builder()
        .title("Kategoriler")
        .content_width(720)
        .content_height(700)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_start(16);
    content.set_margin_end(16);
    let intro = gtk::Label::new(Some(
        "Başlık ve kaynak etiketleriyle eşleşir. Seçilen konulardan biri yeterlidir; seçim yoksa konu sınırı uygulanmaz. İçerik izinlerin ayrıca geçerlidir.",
    ));
    intro.set_wrap(true);
    intro.set_xalign(0.0);
    intro.add_css_class("dim-label");
    intro.add_css_class("caption");
    content.append(&intro);
    let search = gtk::SearchEntry::builder()
        .placeholder_text("200 kategori içinde ara…")
        .build();
    search.set_search_delay(120);
    search.update_property(&[gtk::accessible::Property::Label("Kategori ara")]);
    content.append(&search);
    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);
    let tabs = adw::ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();
    content.append(&tabs);
    content.append(&stack);
    let query = Rc::new(RefCell::new(String::new()));
    let mut filters = Vec::new();
    let mut stores = Vec::new();
    let counter = gtk::Label::new(None);
    counter.set_hexpand(true);
    counter.set_xalign(0.0);
    counter.add_css_class("caption");
    let original = initial.clone();
    let update_count = {
        let draft = draft.clone();
        let label = counter.clone();
        Rc::new(move || {
            let d = draft.borrow();
            label.set_text(&format!(
                "{} genel · {} NSFW seçili · {} değişiklik",
                d.0.len(),
                d.1.len(),
                d.0.symmetric_difference(&original.0).count()
                    + d.1.symmetric_difference(&original.1).count()
            ));
        })
    };
    update_count();
    for sensitive in [false, true] {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        for topic in catalog::all(sensitive) {
            store.append(&glib::BoxedAnyObject::new(topic.clone()));
        }
        stores.push((store.clone(), sensitive));
        let selected_group = Rc::new(Cell::new(0u32));
        let selected_only = Rc::new(Cell::new(false));
        let only = selected_only.clone();
        let filter_draft = draft.clone();
        let q = query.clone();
        let group = selected_group.clone();
        let filter = gtk::CustomFilter::new(move |obj| {
            let topic = obj
                .downcast_ref::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Topic>();
            let query = q.borrow();
            (!only.get() || {
                let d = filter_draft.borrow();
                if sensitive {
                    d.1.contains(&topic.id)
                } else {
                    d.0.contains(&topic.id)
                }
            }) && (group.get() == 0 || topic.group == group.get() as usize - 1)
                && (query.is_empty()
                    || catalog::search_key(&topic.title).contains(query.as_str())
                    || topic
                        .aliases
                        .iter()
                        .any(|a| catalog::search_key(a).contains(query.as_str())))
        });
        filters.push(filter.clone());
        let filtered = gtk::FilterListModel::new(Some(store), Some(filter.clone()));
        let selection = gtk::NoSelection::new(Some(filtered.clone()));
        let factory = gtk::SignalListItemFactory::new();
        let d = draft.clone();
        let update = update_count.clone();
        let selection_filter = filter.clone();
        factory.connect_setup(move |_, object| {
            let row = object.downcast_ref::<gtk::ListItem>().unwrap();
            let check = gtk::CheckButton::new();
            check.set_height_request(44);
            check.set_margin_start(12);
            check.set_margin_end(12);
            check.set_hexpand(true);
            row.set_child(Some(&check));
            let weak = row.downgrade();
            let draft = d.clone();
            let update = update.clone();
            let selection_filter = selection_filter.clone();
            check.connect_toggled(move |check| {
                let Some(row) = weak.upgrade() else {
                    return;
                };
                let Some(item) = row.item().and_downcast::<glib::BoxedAnyObject>() else {
                    return;
                };
                let topic = item.borrow::<Topic>();
                let mut d = draft.borrow_mut();
                let selected = if topic.sensitive { &mut d.1 } else { &mut d.0 };
                if selected.contains(&topic.id) == check.is_active() {
                    return;
                }
                if check.is_active() {
                    selected.insert(topic.id.clone());
                } else {
                    selected.remove(&topic.id);
                }
                drop(d);
                update();
                selection_filter.changed(gtk::FilterChange::Different);
            });
        });
        let d = draft.clone();
        factory.connect_bind(move |_, object| {
            let row = object.downcast_ref::<gtk::ListItem>().unwrap();
            let item = row.item().and_downcast::<glib::BoxedAnyObject>().unwrap();
            let topic = item.borrow::<Topic>();
            let check = row.child().and_downcast::<gtk::CheckButton>().unwrap();
            check.set_label(Some(&topic.title));
            check.set_tooltip_text(Some(&topic.aliases.join(", ")));
            let active = {
                let d = d.borrow();
                if sensitive {
                    d.1.contains(&topic.id)
                } else {
                    d.0.contains(&topic.id)
                }
            };
            check.set_active(active);
        });
        let list = gtk::ListView::new(Some(selection), Some(factory));
        list.set_show_separators(true);
        list.add_css_class("boxed-list");
        let pane = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let names = std::iter::once("Tüm gruplar")
            .chain(if sensitive {
                catalog::SENSITIVE_GROUPS
            } else {
                catalog::GENERAL_GROUPS
            })
            .collect::<Vec<_>>();
        let groups = gtk::DropDown::from_strings(&names);
        groups.update_property(&[gtk::accessible::Property::Label("Kategori grubu")]);
        let group_choice = selected_group.clone();
        let f = filter.clone();
        groups.connect_selected_notify(move |r| {
            selected_group.set(r.selected());
            f.changed(gtk::FilterChange::Different);
        });
        let top_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        groups.set_hexpand(true);
        top_bar.append(&groups);
        let onlybutton = gtk::ToggleButton::with_label("Yalnızca seçili");
        let f = filter.clone();
        onlybutton.connect_toggled(move |b| {
            selected_only.set(b.is_active());
            f.changed(gtk::FilterChange::Different);
        });
        top_bar.append(&onlybutton);
        let bulk_btn = gtk::MenuButton::builder()
            .label("Toplu seçim")
            .tooltip_text("Grup içi toplu seçim işlemleri")
            .build();
        let bulk_pop = gtk::Popover::new();
        let bulk_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        bulk_box.set_margin_top(6);
        bulk_box.set_margin_bottom(6);
        bulk_box.set_margin_start(6);
        bulk_box.set_margin_end(6);
        for (name, mode) in [
            ("Gruptakileri seç", 0),
            ("Gruptakileri kaldır", 1),
            ("Seçimi tersine çevir", 2),
        ] {
            let b = gtk::Button::with_label(name);
            b.add_css_class("flat");
            let d = draft.clone();
            let st = stores.last().unwrap().0.clone();
            let choice = group_choice.clone();
            let q = query.clone();
            let update = update_count.clone();
            let bp = bulk_pop.downgrade();
            b.connect_clicked(move |_| {
                if let Some(p) = bp.upgrade() {
                    p.popdown();
                }
                let mut d = d.borrow_mut();
                let target = if sensitive { &mut d.1 } else { &mut d.0 };
                for topic in catalog::all(sensitive).iter().filter(|t| {
                    (choice.get() == 0 || t.group == choice.get() as usize - 1)
                        && (q.borrow().is_empty()
                            || catalog::search_key(&t.title).contains(q.borrow().as_str())
                            || t.aliases
                                .iter()
                                .any(|a| catalog::search_key(a).contains(q.borrow().as_str())))
                }) {
                    if mode == 0 || (mode == 2 && !target.contains(&topic.id)) {
                        target.insert(topic.id.clone());
                    } else {
                        target.remove(&topic.id);
                    }
                }
                drop(d);
                let items = catalog::all(sensitive)
                    .iter()
                    .cloned()
                    .map(glib::BoxedAnyObject::new)
                    .collect::<Vec<_>>();
                st.splice(0, st.n_items(), &items);
                update();
            });
            bulk_box.append(&b);
        }
        bulk_pop.set_child(Some(&bulk_box));
        bulk_btn.set_popover(Some(&bulk_pop));
        top_bar.append(&bulk_btn);
        pane.append(&top_bar);
        pane.append(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .min_content_height(300)
                .build(),
        );
        stack.add_titled_with_icon(
            &pane,
            Some(if sensitive { "sensitive" } else { "general" }),
            if sensitive {
                "NSFW · 100"
            } else {
                "Genel · 100"
            },
            if sensitive {
                "view-reveal-symbolic"
            } else {
                "image-x-generic-symbolic"
            },
        );
    }
    let q = query.clone();
    search.connect_search_changed(move |entry| {
        *q.borrow_mut() = catalog::search_key(&entry.text());
        for f in &filters {
            f.changed(gtk::FilterChange::Different);
        }
    });
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 8);
    footer.set_margin_start(16);
    footer.set_margin_end(16);
    footer.set_margin_top(12);
    footer.set_margin_bottom(12);
    footer.append(&counter);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    footer.append(&buttons);
    let clear = gtk::Button::with_label("Temizle");
    let d = draft.clone();
    let update = update_count.clone();
    clear.connect_clicked(move |_| {
        d.borrow_mut().0.clear();
        d.borrow_mut().1.clear();
        for (store, sensitive) in &stores {
            let items = catalog::all(*sensitive)
                .iter()
                .cloned()
                .map(glib::BoxedAnyObject::new)
                .collect::<Vec<_>>();
            store.splice(0, store.n_items(), &items);
        }
        update();
    });
    buttons.append(&clear);
    let cancel = gtk::Button::with_label("Vazgeç");
    let dlg = dialog.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(d) = dlg.upgrade() {
            d.close();
        }
    });
    buttons.append(&cancel);
    let apply = gtk::Button::with_label("Uygula");
    apply.add_css_class("suggested-action");
    let w = weak(state);
    let dlg = dialog.downgrade();
    apply.connect_clicked(move |_| {
        if let Some(s) = w.upgrade() {
            let d = draft.borrow();
            if *d == initial {
                if let Some(dialog) = dlg.upgrade() {
                    dialog.close();
                }
                return;
            }
            {
                let mut c = s.config.borrow_mut();
                c.policy.general_topics = d.0.iter().cloned().collect();
                c.policy.sensitive_topics = d.1.iter().cloned().collect();
            }
            s.changed();
            summary(&s);
        }
        if let Some(dialog) = dlg.upgrade() {
            dialog.close();
        }
    });
    buttons.append(&apply);
    toolbar.add_bottom_bar(&footer);
    toolbar.set_content(Some(&content));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&window));
}
