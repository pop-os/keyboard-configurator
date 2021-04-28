use backend::{Board, DerefCell, Rgb};
use cascade::cascade;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{cell::RefCell, collections::HashMap};

#[derive(Clone, Default, glib::GBoxed)]
#[gboxed(type_name = "S76TestingColor")]
pub struct TestingColors(pub HashMap<usize, Rgb>);

#[derive(Default)]
pub struct TestingInner {
    board: DerefCell<Board>,
    num_runs_entry: DerefCell<gtk::Entry>,
    serial_entry: DerefCell<gtk::Entry>,
    test_button: DerefCell<gtk::Button>,
    colors: RefCell<TestingColors>,
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

        fn color_box(r: f64, g: f64, b: f64) -> gtk::DrawingArea {
            cascade! {
                gtk::DrawingArea::new();
                ..set_size_request(18, 18);
                ..connect_draw(move |_w, cr| {
                    cr.set_source_rgb(r, g, b);
                    cr.paint();
                    Inhibit(false)
                });
            }
        }

        let num_runs_entry = gtk::Entry::new();
        let serial_entry = gtk::Entry::new();
        let test_button = gtk::Button::with_label("Test");

        cascade! {
            obj;
            ..set_valign(gtk::Align::Start);
            ..get_style_context().add_class("frame");
            ..add(&label_row("Check pins", &color_box(1., 0., 0.)));
            ..add(&label_row("Replace switch", &color_box(0., 0., 1.)));
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

        self.colors.borrow_mut().0.insert(0, Rgb::new(255, 0, 0));
        self.colors.borrow_mut().0.insert(1, Rgb::new(0, 0, 255));

        self.num_runs_entry.set(num_runs_entry);
        self.serial_entry.set(serial_entry);
        self.test_button.set(test_button);
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;

        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::boxed(
                "colors",
                "colors",
                "colors",
                TestingColors::get_type(),
                glib::ParamFlags::READABLE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn get_property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "colors" => self.colors.borrow().to_value(),
            _ => unimplemented!(),
        }
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
    pub fn new(board: Board) -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().board.set(board);
        obj
    }

    fn inner(&self) -> &TestingInner {
        TestingInner::from_instance(self)
    }
}
