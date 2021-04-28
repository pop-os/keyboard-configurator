use backend::DerefCell;
use cascade::cascade;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

#[derive(Default)]
pub struct TestingInner {
    num_runs_entry: DerefCell<gtk::Entry>,
    serial_entry: DerefCell<gtk::Entry>,
    test_button: DerefCell<gtk::Button>,
}

#[glib::object_subclass]
impl ObjectSubclass for TestingInner {
    const NAME: &'static str = "S76Testing";
    type ParentType = gtk::ListBox;
    type Type = Testing;
}

impl ObjectImpl for TestingInner {
    fn constructed(&self, obj: &Self::Type) {
        fn row(widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            cascade! {
                gtk::ListBoxRow::new();
                ..set_selectable(false);
                ..set_activatable(false);
                ..set_property_margin(8);
                ..add(widget);
            }
        }

        fn label_row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            row(&cascade! {
                gtk::Box::new(gtk::Orientation::Horizontal, 8);
                ..add(&cascade! {
                    gtk::Label::new(Some(label));
                    ..set_halign(gtk::Align::Start);
                });
                ..pack_end(widget, false, false, 0);
            })
        }

        let num_runs_entry = gtk::Entry::new();
        let serial_entry = gtk::Entry::new();
        let test_button = gtk::Button::with_label("Test");

        cascade! {
            obj;
            ..set_valign(gtk::Align::Start);
            ..get_style_context().add_class("frame");
            ..add(&row(&cascade! {
                gtk::Label::new(Some("Testing"));
            }));
            ..add(&label_row("Number of runs", &num_runs_entry));
            ..add(&label_row("Serial", &serial_entry));
            ..add(&row(&test_button));
            ..set_header_func(Some(Box::new(|row, before| {
                if before.is_none() {
                    row.set_header::<gtk::Widget>(None)
                } else if row.get_header().is_none() {
                    row.set_header(Some(&cascade! {
                        gtk::Separator::new(gtk::Orientation::Horizontal);
                        ..show();
                    }));
                }
            })));
            ..show_all();
        };

        self.num_runs_entry.set(num_runs_entry);
        self.serial_entry.set(serial_entry);
        self.test_button.set(test_button);
    }
}

impl WidgetImpl for TestingInner {}
impl ContainerImpl for TestingInner {}
impl ListBoxImpl for TestingInner {}

glib::wrapper! {
    pub struct Testing(ObjectSubclass<TestingInner>)
        @extends gtk::ListBox, gtk::Container, gtk::Widget;
}

impl Testing {
    pub fn new() -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();
        obj
    }

    fn inner(&self) -> &TestingInner {
        TestingInner::from_instance(self)
    }
}
