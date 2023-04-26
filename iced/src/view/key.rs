use cosmic::{
    iced::{self, Application, Color, Rectangle},
    iced_style,
};

use crate::Msg;

const SCALE: f32 = 64.;
const MARGIN: f32 = 4.;
const RADIUS: f32 = 4.;
const SELECTED_BORDER: f32 = 4.0;
const SELECTED_COLOR: Color = Color::from_rgb(0.984, 0.722, 0.424);

fn key_button_appearance(
    _: &cosmic::Theme,
    selected: bool,
) -> cosmic::iced_style::button::Appearance {
    cosmic::iced_style::button::Appearance {
        shadow_offset: iced::Vector::new(0.0, 0.0),
        background: Some(iced_style::Background::Color(Color::BLACK)),
        border_radius: RADIUS.into(),
        border_width: if selected { SELECTED_BORDER } else { 0.0 },
        border_color: SELECTED_COLOR,
        text_color: Color::WHITE,
    }
}

fn key_button_appearance_default(theme: &cosmic::Theme) -> cosmic::iced_style::button::Appearance {
    key_button_appearance(theme, false)
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

    let scancode_name = key.get_scancode(layer).unwrap().1;

    let label = iced::widget::text(&scancode_name).style(cosmic::theme::Text::Color(fg));
    let element = iced::widget::button(label)
        .style(cosmic::theme::Button::Custom {
            active: key_button_appearance_default,
            hover: key_button_appearance_default,
        })
        .into();
    (element, key_position_wide(&key.physical))
}
