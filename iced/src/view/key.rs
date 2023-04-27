use cosmic::{
    iced::{self, Application, Color, Rectangle},
    iced_native::alignment::{Horizontal, Vertical},
    iced_style,
};

use crate::{Msg, Page};

const SCALE: f32 = 64.;
const MARGIN: f32 = 4.;
const RADIUS: f32 = 4.;
const SELECTED_BORDER: f32 = 4.0;
const SELECTED_COLOR: Color = Color::from_rgb(0.984, 0.722, 0.424);

fn key_button_appearance(
    _: &cosmic::Theme,
    background: Color,
    selected: bool,
) -> cosmic::iced_style::button::Appearance {
    cosmic::iced_style::button::Appearance {
        shadow_offset: iced::Vector::new(0.0, 0.0),
        background: Some(iced_style::Background::Color(background)),
        border_radius: RADIUS.into(),
        border_width: if selected { SELECTED_BORDER } else { 0.0 },
        border_color: SELECTED_COLOR,
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

pub(crate) fn key(
    key: &backend::Key,
    page: Page,
    pressed_color: backend::Rgb,
    layer: usize,
) -> (cosmic::Element<Msg>, Rectangle) {
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

    let label_text = page.get_label(key);
    let labels = label_text
        .split('\n')
        .map(|text| {
            iced::widget::text(&text)
                .style(cosmic::theme::Text::Color(fg))
                .horizontal_alignment(Horizontal::Center)
                .width(iced::Length::Fill)
                .into()
        })
        .collect();

    let element = iced::widget::button(
        iced::widget::column(labels)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
    )
    .height(iced::Length::Fill)
    .style(cosmic::theme::Button::Custom {
        active: Box::new(move |theme| key_button_appearance(theme, bg, false)),
        hover: Box::new(move |theme| key_button_appearance(theme, bg, false)),
    })
    .into();
    (element, key_position_wide(&key.physical))
}
