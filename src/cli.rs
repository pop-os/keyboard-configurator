use backend::{Backend, Board, KeyMap};
use futures::channel::oneshot;
use gtk::{glib::clone, prelude::*};
use std::{
    cell::RefCell,
    fs::File,
    io::{self, Write},
    process,
    rc::Rc,
};

pub enum Device {
    Any,
    Internal,
    Usb,
}

impl Device {
    pub fn new(internal: bool, usb: bool) -> Self {
        if internal {
            Self::Internal
        } else if usb {
            Self::Usb
        } else {
            Self::Any
        }
    }
}

async fn backend_boards() -> (Backend, Vec<Board>) {
    let backend = Backend::new().expect("Failed to create server");

    let boards = Rc::new(RefCell::new(Vec::new()));
    let id1 = backend.connect_board_added(clone!(@strong boards => move |board| {
        boards.borrow_mut().push(board.clone());
    }));

    let (sender, receiver) = oneshot::channel::<()>();
    let sender = RefCell::new(Some(sender));
    let id2 = backend.connect_board_loading_done(move || {
        if let Some(sender) = sender.borrow_mut().take() {
            sender.send(()).unwrap();
        }
    });
    backend.refresh();
    receiver.await.unwrap();

    backend.disconnect(id1);
    backend.disconnect(id2);

    (backend, boards.take())
}

pub async fn list_boards() {
    let (_backend, boards) = backend_boards().await;

    for board in boards {
        println!("{}", board.display_name());
    }
}

fn match_board(board: &Board, device: &Device) -> bool {
    let is_usb = board.layout().meta.is_usb;
    match device {
        Device::Any => true,
        Device::Internal => !is_usb,
        Device::Usb => is_usb,
    }
}

pub async fn board(device: Device) -> Board {
    let (backend, mut boards) = backend_boards().await;

    boards = boards
        .into_iter()
        .filter(|board| match_board(board, &device))
        .collect();

    if boards.is_empty() {
        error!("No board detected");
        process::exit(1)
    } else if boards.len() == 1 {
        boards[0].clone()
    } else {
        eprintln!("Multiple boards detected");
        for (i, board) in boards.iter().enumerate() {
            // Human readable name?
            eprintln!("[{}] {}", i, board.display_name());
        }
        print!("> ");
        io::stdout().lock().flush().unwrap();
        let mut selection = String::new();
        io::stdin().read_line(&mut selection).unwrap();
        // XXX panic
        boards
            .get(selection.trim().parse::<usize>().unwrap())
            .unwrap()
            .clone()
    }
}

// usb:  bool
pub async fn save(device: Device, path: String) {
    let board = board(device).await;
    let keymap = board.export_keymap();
    match File::create(&path) {
        Ok(file) => match keymap.to_writer_pretty(file) {
            Ok(()) => (),
            Err(err) => todo!(),
        },
        Err(err) => todo!(),
    }
}

pub async fn load(device: Device, path: String) {
    let board = board(device).await;
    let keymap = match File::open(&path) {
        Ok(file) => match KeyMap::from_reader(file) {
            Ok(keymap) => keymap,
            Err(err) => todo!(),
        },
        Err(err) => todo!(),
    };
    board.import_keymap(keymap).await;
}

pub async fn reset(device: Device) {
    let board = board(device).await;
    board.import_keymap(board.layout().default.clone()).await;
}
