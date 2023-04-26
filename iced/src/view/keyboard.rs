use cosmic::iced;

use crate::{view, FixedWidget, Msg};

pub(crate) fn keyboard(board: &backend::Board) -> cosmic::Element<Msg> {
    let meta = &board.layout().meta;
    let mut key_views = Vec::new();
    for key in board.keys() {
        key_views.push(view::key(key, meta.pressed_color, 0));
    }
    iced::widget::column![
        cosmic::widget::text(&meta.display_name),
        FixedWidget::new(key_views),
    ]
    .into()
}
