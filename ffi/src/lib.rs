use glib::object::ObjectType;
use pop_keyboard_backlight::{keyboards, KeyboardColorButton};
use std::boxed::Box;
use std::ptr;

#[no_mangle]
pub struct PopKeyboardColorButton;

#[no_mangle]
pub extern "C" fn pop_keyboard_color_button_new() -> *mut PopKeyboardColorButton {
    unsafe {
        gtk::set_initialized();
    }

    // TODO: UI For multiple
    let keyboard = keyboards().next().unwrap();

    Box::into_raw(Box::new(KeyboardColorButton::new(keyboard))) as *mut PopKeyboardColorButton
}

#[no_mangle]
pub extern "C" fn pop_keyboard_color_button_widget(
    ptr: *const PopKeyboardColorButton,
) -> *mut gtk_sys::GtkWidget {
    let value = unsafe { (ptr as *const KeyboardColorButton).as_ref() };
    value.map_or(ptr::null_mut(), |v| v.widget().as_ptr())
}

#[no_mangle]
pub extern "C" fn pop_keyboard_color_button_free(widget: *mut PopKeyboardColorButton) {
    unsafe { Box::from_raw(widget as *mut KeyboardColorButton) };
}
