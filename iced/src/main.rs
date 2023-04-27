// TODO Need multi-window, cross platform

use cosmic::iced::{self, Application, Command, Subscription};
use futures::StreamExt;
use i18n_embed::DesktopLanguageRequester;
use std::collections::HashMap;

use backend::Backend;

mod fixed_widget;
use fixed_widget::FixedWidget;
mod localize;
mod page;
pub use page::Page;
mod picker_json;
mod view;

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
        iced::widget::column(
            self.keyboards
                .iter()
                .map(|keyboard| view::keyboard(&keyboard.board, Page::Layer1))
                .collect(),
        )
        .into()
    }
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
    translate();
    App::run(iced::Settings {
        antialiasing: true,
        ..iced::Settings::default()
    })
}

fn translate() {
    let requested_languages = DesktopLanguageRequester::requested_languages();

    let localizers = vec![
        ("keyboard-configurator", crate::localize::localizer()),
        ("backend", backend::localizer()),
    ];

    for (crate_name, localizer) in localizers {
        if let Err(error) = localizer.select(&requested_languages) {
            eprintln!("Error while loading languages for {} {}", crate_name, error);
        }
    }
}
