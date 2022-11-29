use crate::{fl, Keyboard};
use backend::{Board, DerefCell, NelsonKind, Rgb};
use cascade::cascade;
use futures::channel::oneshot;
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::OnceCell;
use std::{cell::RefCell, collections::HashMap, sync::RwLock};

struct TestResults {
    bench: RwLock<HashMap<&'static str, Result<f64, String>>>,
}

impl TestResults {
    fn global() -> &'static Self {
        static TEST_RESULTS: OnceCell<TestResults> = OnceCell::new();
        TEST_RESULTS.get_or_init(Self::new)
    }

    fn new() -> Self {
        let test_results = Self {
            bench: RwLock::new(HashMap::new()),
        };
        test_results.reset();
        test_results
    }

    fn reset(&self) {
        let mut bench = self.bench.write().unwrap();
        bench.clear();
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
            bench.insert(*port_desc, Err("no benchmarks performed".to_string()));
        }
    }
}

#[derive(Clone, Default, glib::Boxed)]
#[boxed_type(name = "S76TestingColor")]
pub struct TestingColors(pub HashMap<(usize, usize), Rgb>);

#[derive(Default)]
pub struct TestingInner {
    board: DerefCell<Board>,
    keyboard: DerefCell<glib::WeakRef<Keyboard>>,
    reset_button: DerefCell<gtk::Button>,
    bench_button: DerefCell<gtk::ToggleButton>,
    bench_labels: DerefCell<HashMap<&'static str, gtk::Label>>,
    num_runs_spin_2: DerefCell<gtk::SpinButton>,
    test_buttons: DerefCell<[gtk::Button; 2]>,
    test_labels: DerefCell<[gtk::Label; 3]>,
    selma_start_button: DerefCell<gtk::Button>,
    selma_stop_button: DerefCell<gtk::Button>,
    selma_stop_sender: RefCell<Option<oneshot::Sender<()>>>,
    colors: RefCell<TestingColors>,
}

#[glib::object_subclass]
impl ObjectSubclass for TestingInner {
    const NAME: &'static str = "S76Testing";
    type ParentType = gtk::Box;
    type Type = Testing;
}

impl ObjectImpl for TestingInner {
    fn constructed(&self, obj: &Self::Type) {
        fn row(widget: &impl IsA<gtk::Widget>) -> gtk::ListBoxRow {
            cascade! {
                gtk::ListBoxRow::new();
                ..set_selectable(false);
                ..set_activatable(false);
                ..set_margin(8);
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
                    cr.paint().unwrap();
                    Inhibit(false)
                });
            }
        }

        fn header_func(row: &gtk::ListBoxRow, before: Option<&gtk::ListBoxRow>) {
            if before.is_none() {
                row.set_header(None::<&gtk::Widget>)
            } else if row.header().is_none() {
                row.set_header(Some(&cascade! {
                    gtk::Separator::new(gtk::Orientation::Horizontal);
                    ..show();
                }));
            }
        }

        let reset_button = gtk::Button::with_label("Reset testing");

        obj.add(&cascade! {
            gtk::ListBox::new();
            ..set_valign(gtk::Align::Start);
            ..style_context().add_class("frame");
            ..add(&row(&reset_button));
        });

        let bench_list = gtk::ListBox::new();

        let mut bench_labels = HashMap::new();
        for (port_desc, _port_result) in TestResults::global().bench.read().unwrap().iter() {
            let bench_label = gtk::Label::new(None);
            bench_list.add(&label_row(port_desc, &bench_label));
            bench_labels.insert(*port_desc, bench_label);
        }

        let bench_button = gtk::ToggleButton::with_label("Run USB test");

        let usb_test = &cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&gtk::Label::new(Some("USB Port Test")));
            ..add(&cascade! {
                bench_list;
                ..set_valign(gtk::Align::Start);
                ..style_context().add_class("frame");
                ..add(&row(&bench_button));
                ..set_header_func(Some(Box::new(header_func)));
            });
        };

        let num_runs_spin_2 = gtk::SpinButton::with_range(1.0, 1000.0, 1.0);
        num_runs_spin_2.set_value(100.0);

        let test_buttons = [
            gtk::Button::with_label(&fl!("button-test")),
            gtk::Button::with_label(&fl!("button-test")),
        ];
        let test_labels = [
            gtk::Label::new(None),
            gtk::Label::new(None),
            gtk::Label::new(None),
        ];

        let nelson_test_1 = &cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&gtk::Label::new(Some("Nelson Test 1")));
            ..add(&cascade! {
                gtk::ListBox::new();
                ..set_valign(gtk::Align::Start);
                ..style_context().add_class("frame");
                ..add(&row(&test_buttons[0]));
                ..add(&row(&test_labels[0]));
                ..add(&label_row("Check pins (missing)", &color_box(1., 0., 0.)));
                ..add(&label_row("Check key (sticking)", &color_box(0., 1., 0.)));
                ..set_header_func(Some(Box::new(header_func)));
            });
        };

        let nelson_test_2 = &cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&gtk::Label::new(Some("Nelson Test 2")));
            ..add(&cascade! {
                gtk::ListBox::new();
                ..set_valign(gtk::Align::Start);
                ..style_context().add_class("frame");
                ..add(&label_row(&fl!("test-number-of-runs"), &num_runs_spin_2));
                ..add(&row(&test_buttons[1]));
                ..add(&row(&test_labels[2]));
                ..add(&label_row(&fl!("test-check-pins"), &color_box(1., 0., 0.)));
                ..add(&label_row(&fl!("test-check-key"), &color_box(0., 1., 0.)));
                ..set_header_func(Some(Box::new(header_func)));
            });
        };

        let selma_start_button = gtk::Button::with_label(&fl!("button-start"));
        let selma_stop_button = cascade! {
            gtk::Button::with_label(&fl!("button-stop"));
            ..set_sensitive(false);
        };

        let selma_test = &cascade! {
            gtk::Box::new(gtk::Orientation::Vertical, 12);
            ..add(&gtk::Label::new(Some("Selma Test")));
            ..add(&cascade! {
                gtk::ListBox::new();
                ..set_valign(gtk::Align::Start);
                ..style_context().add_class("frame");
                ..add(&row(&cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    ..set_halign(gtk::Align::Center);
                    ..add(&selma_start_button);
                    ..add(&selma_stop_button);
                }));
                ..add(&label_row(&fl!("test-spurious-keypress"), &color_box(1., 0., 0.)));
                ..set_header_func(Some(Box::new(header_func)));
            });
        };

        obj.add(&cascade! {
            gtk::Box::new(gtk::Orientation::Horizontal, 18);
            ..set_valign(gtk::Align::Start);
            ..add(&cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 18);
                ..set_valign(gtk::Align::Start);
                ..add(&row(usb_test));
                ..add(&row(selma_test));
            });
            ..add(&cascade! {
                gtk::Box::new(gtk::Orientation::Vertical, 18);
                ..set_valign(gtk::Align::Start);
                ..add(&row(nelson_test_1));
                ..add(&row(nelson_test_2));
            });
        });

        self.reset_button.set(reset_button);
        self.bench_button.set(bench_button);
        self.bench_labels.set(bench_labels);
        self.num_runs_spin_2.set(num_runs_spin_2);
        self.test_buttons.set(test_buttons);
        self.test_labels.set(test_labels);
        self.selma_start_button.set(selma_start_button);
        self.selma_stop_button.set(selma_stop_button);

        cascade! {
            obj;
            ..set_orientation(gtk::Orientation::Vertical);
            ..set_spacing(18);
            ..show_all();
        };
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;

        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpecBoxed::new(
                "colors",
                "colors",
                "colors",
                TestingColors::static_type(),
                glib::ParamFlags::READABLE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "colors" => self.colors.borrow().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for TestingInner {}
impl ContainerImpl for TestingInner {}
impl BoxImpl for TestingInner {}

glib::wrapper! {
    pub struct Testing(ObjectSubclass<TestingInner>)
        @extends gtk::Box, gtk::Container, gtk::Widget, @implements gtk::Orientable;
}

impl Testing {
    fn update_benchmarks(&self) {
        for (port_desc, port_result) in TestResults::global().bench.read().unwrap().iter() {
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

    async fn bench(&self) {
        let testing = self.inner();

        testing.bench_button.set_label("Running USB test");

        while testing.bench_button.is_active() {
            match testing.board.benchmark().await {
                Ok(benchmark) => {
                    for (port_desc, port_result) in benchmark.port_results.iter() {
                        let text = format!("{:.2?}", port_result);
                        info!("{}: {}", port_desc, text);
                        if let Some(bench_result) = TestResults::global()
                            .bench
                            .write()
                            .unwrap()
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
                                Err(_err) => {
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

            self.update_benchmarks();

            glib::timeout_future(std::time::Duration::new(1, 0)).await;
        }

        testing.bench_button.set_label("Run USB test");
    }

    fn connect_bench_button(&self) {
        self.inner()
            .bench_button
            .connect_clicked(clone!(@strong self as self_ => move |_| {
                glib::MainContext::default().spawn_local(clone!(@strong self_ => async move {
                    self_.bench().await;
                }));
            }));
    }

    fn test_buttons_sensitive(&self, sensitive: bool) {
        for i in 0..2 {
            self.inner().test_buttons[i].set_sensitive(sensitive);
        }
        self.inner().selma_start_button.set_sensitive(sensitive);
    }

    async fn nelson(&self, test_runs: i32, test_index: usize, nelson_kind: NelsonKind) {
        let testing = self.inner();

        info!("Disabling test buttons");
        self.test_buttons_sensitive(false);

        let test_label = &testing.test_labels[test_index];

        info!("Disable keyboard input events");
        self.set_no_input(true).await;

        for test_run in 1..=test_runs {
            let message = format!("Test {}/{} running", test_run, test_runs);
            info!("{}", message);
            test_label.set_text(&message);

            let nelson = match testing.board.nelson(nelson_kind).await {
                Ok(ok) => ok,
                Err(err) => {
                    let message = format!("Test {}/{} failed to run: {}", test_run, test_runs, err);
                    error!("{}", message);
                    test_label.set_text(&message);
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

            self.notify("colors");

            if nelson.success(testing.board.layout().layout()) {
                let message = format!("Test {}/{} successful", test_run, test_runs);
                info!("{}", message);
                test_label.set_text(&message);
            } else {
                let message = format!("Test {}/{} failed", test_run, test_runs);
                error!("{}", message);
                test_label.set_text(&message);
                break;
            }
        }

        info!("Re-enable keyboard input events");
        self.set_no_input(false).await;

        info!("Enabling test buttons");
        self.test_buttons_sensitive(true);
    }

    fn connect_test_button_1(&self) {
        self.inner().test_buttons[0].connect_clicked(clone!(@strong self as self_ => move |_| {
            glib::MainContext::default().spawn_local(clone!(@strong self_ => async move {
                self_.nelson(1, 0, NelsonKind::Normal).await;
            }));
        }));
    }

    fn connect_test_button_2(&self) {
        self.inner().test_buttons[1].connect_clicked(clone!(@strong self as self_ => move |_| {
            glib::MainContext::default().spawn_local(clone!(@strong self_ => async move {
                self_.nelson(
                    self_.inner().num_runs_spin_2.value_as_int(),
                    2,
                    NelsonKind::Normal,
                ).await;
            }));
        }));
    }

    fn selma_update_colors(&self) {
        let mut colors = self.inner().colors.borrow_mut();
        for k in self.inner().board.keys() {
            let (row, col) = k.electrical;
            if k.pressed() {
                colors
                    .0
                    .insert((row as usize, col as usize), Rgb::new(255, 0, 0));
            }
        }
        drop(colors);
        self.notify("colors");
    }

    async fn selma(&self) {
        let testing = self.inner();

        info!("Disabling test buttons");
        self.test_buttons_sensitive(false);
        testing.selma_stop_button.set_sensitive(true);

        info!("Disable keyboard input events");
        self.set_no_input(true).await;

        testing.colors.borrow_mut().0.clear();
        let matrix_changed_handle =
            testing
                .board
                .connect_matrix_changed(clone!(@strong self as self_ => move || {
                    self_.selma_update_colors();
                }));
        self.selma_update_colors();

        // Wait for stop button to be pressed
        let (sender, reciever) = oneshot::channel();
        *testing.selma_stop_sender.borrow_mut() = Some(sender);
        let _ = reciever.await;

        testing.board.disconnect(matrix_changed_handle);

        info!("Re-enable keyboard input events");
        self.set_no_input(false).await;

        info!("Enabling test buttons");
        self.test_buttons_sensitive(true);
        testing.selma_stop_button.set_sensitive(false);
    }

    fn connect_selma_buttons(&self) {
        self.inner()
            .selma_start_button
            .connect_clicked(clone!(@strong self as self_ => move |_| {
                glib::MainContext::default().spawn_local(clone!(@strong self_ => async move {
                    self_.selma().await;
                }));
            }));

        self.inner()
            .selma_stop_button
            .connect_clicked(clone!(@strong self as self_ => move |_| {
                glib::MainContext::default().spawn_local(clone!(@strong self_ => async move {
                    if let Some(sender) = self_.inner().selma_stop_sender.borrow_mut().take() {
                        let _ = sender.send(());
                    }
                }));
            }));
    }

    fn connect_reset_button(&self) {
        let obj_btn = self.clone();
        self.inner().reset_button.connect_clicked(move |_button| {
            TestResults::global().reset();
            obj_btn.update_benchmarks();
        });
    }

    pub fn new(board: &Board, keyboard: &Keyboard) -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();
        obj.inner().board.set(board.clone());
        obj.inner().keyboard.set(keyboard.downgrade());
        obj.connect_bench_button();
        obj.connect_test_button_1();
        obj.connect_test_button_2();
        obj.connect_selma_buttons();
        obj.connect_reset_button();
        obj.update_benchmarks();
        obj
    }

    fn inner(&self) -> &TestingInner {
        TestingInner::from_instance(self)
    }

    #[allow(dead_code)]
    fn keyboard(&self) -> Keyboard {
        self.inner().keyboard.upgrade().unwrap()
    }

    async fn set_no_input(&self, no_input: bool) {
        if let Err(err) = self.inner().board.set_no_input(no_input).await {
            error!("Error setting no input mode: {}", err);
        }
    }
}
