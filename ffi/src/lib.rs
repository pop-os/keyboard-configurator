use glib::object::ObjectType;
use pop_keyboard_backlight::KeyboardColorButton;
use std::ptr;
use std::rc::Rc;

#[no_mangle]
pub struct PopKeyboardColorButton;

#[no_mangle]
pub extern "C" fn pop_keyboard_color_button_new() -> *mut PopKeyboardColorButton {
    unsafe {
        gtk::set_initialized();
    }

    Rc::into_raw(KeyboardColorButton::new()) as *mut PopKeyboardColorButton
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
    unsafe { Rc::from_raw(widget as *mut KeyboardColorButton) };
}
