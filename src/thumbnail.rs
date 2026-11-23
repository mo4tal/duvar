use gtk::prelude::*;
use gtk::subclass::prelude::*;
mod imp {
    use super::*;
    #[derive(Default)]
    pub struct Thumbnail;
    #[glib::object_subclass]
    impl ObjectSubclass for Thumbnail {
        const NAME: &'static str = "DuvarThumbnail";
        type Type = super::Thumbnail;
        type ParentType = gtk::Widget;
    }
    impl ObjectImpl for Thumbnail {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent()
            }
        }
    }
    impl WidgetImpl for Thumbnail {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(child) = self.obj().first_child() {
                child.allocate(width, height, baseline, None);
            }
        }
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(child) = self.obj().first_child() {
                self.obj().snapshot_child(&child, snapshot);
            }
        }
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                (180, 240, -1, -1)
            } else {
                let height = if for_size > 0 {
                    ((for_size as f64) * 9.0 / 16.0).round() as i32
                } else {
                    136
                };
                (height, height, -1, -1)
            }
        }
    }
}
glib::wrapper! {pub struct Thumbnail(ObjectSubclass<imp::Thumbnail>) @extends gtk::Widget, @implements gtk::Accessible,gtk::Buildable,gtk::ConstraintTarget;}
impl Thumbnail {
    pub fn new() -> Self {
        let widget: Self = glib::Object::new();
        let picture = gtk::Picture::new();
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_parent(&widget);
        widget.add_css_class("duvar-thumb-frame");
        widget.set_overflow(gtk::Overflow::Hidden);
        widget
    }
    pub fn picture(&self) -> gtk::Picture {
        self.first_child()
            .and_downcast::<gtk::Picture>()
            .expect("Owned thumbnail picture")
    }
}
