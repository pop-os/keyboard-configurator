mod about_dialog;
mod backlight;
mod configurator_app;
mod error_dialog;
mod keyboard;
mod keyboard_layer;
mod main_window;
mod page;
mod picker;
mod shortcuts_window;

pub use self::configurator_app::run;
use self::{
    backlight::*, configurator_app::*, error_dialog::*, keyboard::*, keyboard_layer::*,
    main_window::*, page::*, picker::*, shortcuts_window::*,
};
