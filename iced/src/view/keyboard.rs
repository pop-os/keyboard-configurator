use cosmic::iced;

use crate::{view, FixedWidget, Msg, Page};

pub(crate) fn keyboard(board: &backend::Board, page: Page) -> cosmic::Element<Msg> {
    let meta = &board.layout().meta;
    let mut key_views = Vec::new();
    for key in board.keys() {
        key_views.push(view::key(key, page, meta.pressed_color, 0));
    }
    iced::widget::column![
        cosmic::widget::text(&meta.display_name),
        FixedWidget::new(key_views),
    ]
    .into()
}
