// TODO Need multi-window, cross platform

use cosmic::{
    iced::{self, keyboard::KeyCode, widget, Application, Color, Command, Rectangle, Subscription},
    iced_native::window::Id as SurfaceId,
    iced_style,
};
use futures::StreamExt;
use std::{collections::HashMap, mem};
use tokio::sync::oneshot;

use backend::{Backend, Key, Layout, Rgb};

mod fixed_widget;
use fixed_widget::FixedWidget;

const SCALE: f32 = 64.;
const MARGIN: f32 = 4.;
const RADIUS: f32 = 4.;

#[derive(Clone, Debug)]
enum Msg {
    Backend(BackendEvent),
}

struct Keyboard {
    board: backend::Board,
}

struct App {
    backend: Option<Backend>,
    keyboards: Vec<Keyboard>,
}

impl Application for App {
    type Message = Msg;
    type Theme = cosmic::Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Msg>) {
        (
            Self {
                backend: None,
                keyboards: Vec::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("cosmic-workspaces")
    }

    // TODO transparent style?
    // TODO: show panel and dock? Drag?

    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Backend(BackendEvent::Backend(backend)) => {
                self.backend = Some(backend);
            }
            Msg::Backend(BackendEvent::Event(event)) => match event {
                backend::Event::BoardAdded(board) => {
                    // XXX
                    tokio::spawn(reset_layout(board.clone()));
                    self.keyboards.push(Keyboard { board });
                }
                backend::Event::Board(id, _event) => {
                    let _keyboard = self
                        .keyboards
                        .iter_mut()
                        .find(|x| x.board.board() == id)
                        .unwrap();
                    // Events just need to update view, as long as shared memory is used
                }
                backend::Event::BoardRemoved(id) => {
                    let idx = self
                        .keyboards
                        .iter()
                        .position(|x| x.board.board() == id)
                        .unwrap();
                    self.keyboards.remove(idx);
                }
                backend::Event::BoardLoading
                | backend::Event::BoardLoadingDone
                | backend::Event::BoardNotUpdated
                | backend::Event::BootloadedAdded(_)
                | backend::Event::BootloadedRemoved => {}
            },
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Msg> {
        backend_subscription().map(Msg::Backend)
    }

    fn view(&self) -> cosmic::Element<Msg> {
        iced::widget::column(self.keyboards.iter().map(keyboard_view).collect()).into()
    }
}

fn key_button_appearance(_: &cosmic::Theme) -> cosmic::iced_style::button::Appearance {
    cosmic::iced_style::button::Appearance {
        shadow_offset: iced::Vector::new(0.0, 0.0),
        background: Some(iced_style::Background::Color(Color::BLACK)),
        border_radius: RADIUS.into(),
        border_width: 0.0,
        border_color: Color::WHITE,
        text_color: Color::WHITE,
    }
}

// TODO narrow view?
fn key_position_wide(physical: &backend::Rect) -> Rectangle {
    Rectangle {
        x: physical.x as f32 * SCALE + MARGIN,
        y: physical.y as f32 * SCALE + MARGIN,
        width: physical.w as f32 * SCALE - MARGIN * 2.0,
        height: physical.h as f32 * SCALE - MARGIN * 2.0,
    }
}

fn key_view(key: &Key, pressed_color: Rgb, layer: usize) -> (cosmic::Element<Msg>, Rectangle) {
    let bg = if key.pressed() {
        pressed_color
    } else {
        key.background_color
    };
    let bg = iced::Color::from_rgb8(bg.r, bg.g, bg.b);

    let fg = if (bg.r + bg.g + bg.b) / 3. >= 0.5 {
        iced::Color::BLACK
    } else {
        iced::Color::WHITE
    };

    let scancode_name = key.get_scancode(layer).unwrap().1;

    let label = iced::widget::text(&scancode_name).style(cosmic::theme::Text::Color(fg));
    let element = iced::widget::button(label)
        .style(cosmic::theme::Button::Custom {
            active: key_button_appearance,
            hover: key_button_appearance,
        })
        .into();
    (element, key_position_wide(&key.physical))
}

fn keyboard_view(keyboard: &Keyboard) -> cosmic::Element<Msg> {
    let meta = &keyboard.board.layout().meta;
    let mut key_views = Vec::new();
    for key in keyboard.board.keys() {
        key_views.push(key_view(key, meta.pressed_color, 0));
    }
    iced::widget::column![
        cosmic::widget::text(&meta.display_name),
        FixedWidget::new(key_views),
    ]
    .into()
}

#[derive(Clone, Debug)]
enum BackendEvent {
    Backend(Backend),
    Event(backend::Event),
}

fn backend_subscription() -> iced::Subscription<BackendEvent> {
    let (backend, events) = Backend::new_dummy(vec!["system76/launch_1".to_string()]).unwrap();
    backend.refresh();
    let backend_stream = futures::stream::once(async { BackendEvent::Backend(backend) });
    let event_stream = events.map(BackendEvent::Event);
    iced::subscription::run(
        "keyboard-configurator-backend",
        backend_stream.chain(event_stream),
    )
}

async fn reset_layout(board: backend::Board) {
    let key_indices = board
        .keys()
        .iter()
        .enumerate()
        .map(|(i, k)| (&k.logical_name, i))
        .collect::<HashMap<_, _>>();

    let layout = &board.layout().default;
    for (k, v) in layout.map.iter() {
        for (layer, scancode_name) in v.iter().enumerate() {
            let n = key_indices[&k];
            board.keys()[n]
                .set_scancode(layer, scancode_name)
                .await
                .unwrap();
        }
    }
}

pub fn main() -> iced::Result {
    App::run(iced::Settings {
        antialiasing: true,
        ..iced::Settings::default()
    })
}
