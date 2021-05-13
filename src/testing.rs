use backend::{Board, DerefCell, Rgb};
use cascade::cascade;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::{cell::RefCell, collections::HashMap};

#[derive(Clone, Default, glib::GBoxed)]
#[gboxed(type_name = "S76TestingColor")]
pub struct TestingColors(pub HashMap<(usize, usize), Rgb>);

#[derive(Default)]
pub struct TestingInner {
    board: DerefCell<Board>,
    bench_button: DerefCell<gtk::ToggleButton>,
    bench_labels: DerefCell<HashMap<&'static str, gtk::Label>>,
    bench_results: RefCell<HashMap<&'static str, Result<f64, String>>>,
    num_runs_spin: DerefCell<gtk::SpinButton>,
    serial_entry: DerefCell<gtk::Entry>,
    test_button: DerefCell<gtk::Button>,
    test_label: DerefCell<gtk::Label>,
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

        let bench_button = gtk::ToggleButton::with_label("Run USB test");
        let num_runs_spin = gtk::SpinButton::with_range(1.0, 1000.0, 1.0);
        let serial_entry = gtk::Entry::new();
        let test_button = gtk::Button::with_label("Test");
        let test_label = gtk::Label::new(None);

        let mut bench_labels = HashMap::new();
        let mut bench_results = HashMap::new();
        for port_desc in &[
            "USB 2.0: USB-A Left",
            "USB 2.0: USB-A Right",
            "USB 2.0: USB-C Left",
            "USB 2.0: USB-C Right",
            "USB 3.2 Gen 2: USB-A Left",
            "USB 3.2 Gen 2: USB-A Right",
            "USB 3.2 Gen 2: USB-C Left",
            "USB 3.2 Gen 2: USB-C Right",
        ] {
            let bench_label = gtk::Label::new(None);
            obj.add(&label_row(port_desc, &bench_label));
            bench_labels.insert(*port_desc, bench_label);
            bench_results.insert(*port_desc, Err("no benchmarks performed".to_string()));
        }

        cascade! {
            obj;
            ..set_valign(gtk::Align::Start);
            ..get_style_context().add_class("frame");
            ..add(&row(&bench_button));
            ..add(&label_row("Check pins (missing)", &color_box(1., 0., 0.)));
            ..add(&label_row("Check key (sticking)", &color_box(0., 1., 0.)));
            ..add(&label_row("Replace switch (bouncing)", &color_box(0., 0., 1.)));
            ..add(&label_row("Number of runs", &num_runs_spin));
            ..add(&label_row("Serial", &serial_entry));
            ..add(&row(&test_button));
            ..add(&row(&test_label));
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

        self.bench_button.set(bench_button);
        self.bench_labels.set(bench_labels);
        *self.bench_results.borrow_mut() = bench_results;
        self.num_runs_spin.set(num_runs_spin);
        self.serial_entry.set(serial_entry);
        self.test_button.set(test_button);
        self.test_label.set(test_label);
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

async fn import_keymap_hack(board: &Board, keymap: &backend::KeyMap) -> Result<(), String> {
    for key in board.keys() {
        if let Some(scancodes) = keymap.map.get(&key.logical_name) {
            for layer in 0..scancodes.len() {
                key.set_scancode(layer, &scancodes[layer]).await?;
            }
        }
    }
    Ok(())
}

impl Testing {
    fn update_benchmarks(&self) {
        for (port_desc, port_result) in self.inner().bench_results.borrow().iter() {
            if let Some(bench_label) = self.inner().bench_labels.get(port_desc) {
                match port_result {
                    Ok(ok) => {
                        bench_label.set_text(&format!("{:.2} MB/s ✅", ok));
                    }
                    Err(err) => {
                        bench_label.set_text(&format!("{} ❌", err));
                    }
                }
            } else {
                error!("{} label not found", port_desc);
            }
        }
    }

    fn connect_bench_button(&self) {
        let obj_btn = self.clone();
        self.inner().bench_button.connect_clicked(move |button| {
            button.set_label("Running USB test");

            let obj_spawn = obj_btn.clone();
            glib::MainContext::default().spawn_local(async move {
                let testing = obj_spawn.inner();

                while testing.bench_button.get_active() {
                    match testing.board.benchmark().await {
                        Ok(benchmark) => {
                            for (port_desc, port_result) in benchmark.port_results.iter() {
                                let text = format!("{:.2?}", port_result);
                                info!("{}: {}", port_desc, text);
                                if let Some(bench_result) = testing
                                    .bench_results
                                    .borrow_mut()
                                    .get_mut(port_desc.as_str())
                                {
                                    match bench_result {
                                        Ok(old) => match port_result {
                                            Ok(new) => {
                                                // Replace good results with better results
                                                if new > old {
                                                    *bench_result = Ok(*new);
                                                }
                                            }
                                            Err(_) => (),
                                        },
                                        Err(err) => {
                                            // Replace errors with newest results
                                            *bench_result = port_result.clone();
                                        }
                                    }
                                } else {
                                    error!("{} label result not found", port_desc);
                                }
                            }
                        }
                        Err(err) => {
                            let message = format!("Benchmark failed to run: {}", err);
                            error!("{}", message);
                            //TODO: have a global label?
                            for (_, bench_label) in testing.bench_labels.iter() {
                                bench_label.set_text(&message);
                            }
                        }
                    }

                    obj_spawn.update_benchmarks();

                    glib::timeout_future(std::time::Duration::new(1, 0)).await;
                }

                testing.bench_button.set_label("Run USB test");
            });
        });
    }

    fn connect_test_button(&self) {
        let obj_btn = self.clone();
        self.inner().test_button.connect_clicked(move |button| {
            info!("Disabling test button");
            button.set_sensitive(false);

            let obj_nelson = obj_btn.clone();
            glib::MainContext::default().spawn_local(async move {
                let testing = obj_nelson.inner();

                info!("Save and clear keymap");
                let keymap = testing.board.export_keymap();
                {
                    let mut empty = keymap.clone();
                    for (_name, codes) in empty.map.iter_mut() {
                        for code in codes.iter_mut() {
                            *code = "NONE".to_string();
                        }
                    }
                    if let Err(err) = import_keymap_hack(&testing.board, &empty).await {
                        error!("Failed to clear keymap: {}", err);
                    }
                }

                let test_runs = testing.num_runs_spin.get_value_as_int();
                for test_run in 1..=test_runs {
                    let message = format!("Test {}/{} running", test_run, test_runs);
                    info!("{}", message);
                    testing.test_label.set_text(&message);

                    let nelson = match testing.board.nelson().await {
                        Ok(ok) => ok,
                        Err(err) => {
                            let message =
                                format!("Test {}/{} failed to run: {}", test_run, test_runs, err);
                            error!("{}", message);
                            testing.test_label.set_text(&message);
                            break;
                        }
                    };

                    for row in 0..nelson.max_rows() {
                        for col in 0..nelson.max_cols() {
                            let r = if nelson.missing.get(row, col).unwrap_or(false) {
                                255
                            } else {
                                0
                            };
                            let g = if nelson.sticking.get(row, col).unwrap_or(false) {
                                255
                            } else {
                                0
                            };
                            let b = if nelson.bouncing.get(row, col).unwrap_or(false) {
                                255
                            } else {
                                0
                            };
                            if r != 0 || g != 0 || b != 0 {
                                testing
                                    .colors
                                    .borrow_mut()
                                    .0
                                    .insert((row, col), Rgb::new(r, g, b));
                            } else {
                                testing.colors.borrow_mut().0.remove(&(row, col));
                            }
                        }
                    }

                    obj_nelson.notify("colors");

                    if nelson.success() {
                        let message = format!("Test {}/{} successful", test_run, test_runs);
                        info!("{}", message);
                        testing.test_label.set_text(&message);
                    } else {
                        let message = format!("Test {}/{} failed", test_run, test_runs);
                        error!("{}", message);
                        testing.test_label.set_text(&message);
                        break;
                    }
                }

                info!("Restore keymap");
                if let Err(err) = import_keymap_hack(&testing.board, &keymap).await {
                    error!("Failed to restore keymap: {}", err);
                }

                info!("Enabling test button");
                testing.test_button.set_sensitive(true);
            });
        });
    }

    pub fn new(board: Board) -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().board.set(board);
        obj.connect_bench_button();
        obj.connect_test_button();
        obj.update_benchmarks();
        obj
    }

    fn inner(&self) -> &TestingInner {
        TestingInner::from_instance(self)
    }
}
