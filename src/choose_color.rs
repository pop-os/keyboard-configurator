use cascade::cascade;
use futures::future::abortable;
use glib::clone;
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

use crate::{ColorWheel, KeyboardColorIndex};
use backend::{Board, Hs};

pub async fn choose_color<W: IsA<gtk::Widget>>(
    board: Board,
    w: &W,
    title: &'static str,
    color: Option<Hs>,
    index: KeyboardColorIndex,
) -> Option<Hs> {
    let index = Rc::new(index);
    let original_colors = index.get_colors(&board);
    let abort_handle = Rc::new(RefCell::new(None));
    board.block_led_save();

    let color_wheel = cascade! {
        ColorWheel::new();
        ..set_hs(color.unwrap_or_default());
        ..set_size_request(300, 300);
    };

    let preview = cascade! {
        gtk::DrawingArea::new();
        ..set_halign(gtk::Align::Center);
        ..set_size_request(300, 25);
        ..connect_draw(clone!(@weak color_wheel => @default-panic, move |_w, cr| {
            let (r, g, b) = color_wheel.hs().to_rgb().to_floats();
            cr.set_source_rgb(r, g, b);
            cr.paint();
            Inhibit(false)
        }));
    };

    color_wheel.connect_hs_changed(
        clone!(@strong board, @strong index, @weak preview => @default-panic, move |wheel| {
            glib::MainContext::default().spawn_local(clone!(@strong board, @strong wheel, @strong index, @strong abort_handle => async move {
                let (res, new_abort_handle) = abortable(index.set_color(&board, wheel.hs()));
                if let Some(handle) = abort_handle.replace(Some(new_abort_handle)) {
                    handle.abort();
                }
                if let Ok(Err(err)) = res.await {
                    error!("Failed to set keyboard color: {}", err);
                }
            }));
            preview.queue_draw();
        }),
    );

    let hue_adjustment = gtk::Adjustment::new(0., 0., 360., 1., 1., 0.);
    let saturation_adjustment = gtk::Adjustment::new(0., 0., 100., 1., 1., 0.);
    let flags = glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE;
    color_wheel
        .bind_property("hue", &hue_adjustment, "value")
        .flags(flags)
        .build();
    color_wheel
        .bind_property("saturation", &saturation_adjustment, "value")
        .flags(flags)
        .build();

    let hue_box = cascade! {
        gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ..add(&gtk::Label::new(Some("Hue")));
        ..add(&cascade! {
            gtk::Scale::new(gtk::Orientation::Horizontal, Some(&hue_adjustment));
            ..set_hexpand(true);
            ..set_draw_value(false);
        });
        ..add(&gtk::SpinButton::new(Some(&hue_adjustment), 0., 0));
    };

    let saturation_box = cascade! {
        gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ..add(&gtk::Label::new(Some("Saturation")));
        ..add(&cascade! {
            gtk::Scale::new(gtk::Orientation::Horizontal, Some(&saturation_adjustment));
            ..set_hexpand(true);
            ..set_draw_value(false);
        });
        ..add(&gtk::SpinButton::new(Some(&saturation_adjustment), 0., 0));
    };

    let vbox = cascade! {
        gtk::Box::new(gtk::Orientation::Vertical, 12);
        ..set_property_margin(24);
        ..add(&color_wheel);
        ..add(&preview);
        ..add(&hue_box);
        ..add(&saturation_box);
    };

    let window = w
        .get_toplevel()
        .and_then(|x| x.downcast::<gtk::Window>().ok());

    let dialog = cascade! {
        gtk::DialogBuilder::new()
            .title(title)
            .use_header_bar(1)
            .modal(true)
            .build();
        ..add_button("Cancel", gtk::ResponseType::Cancel);
        ..add_button("Save", gtk::ResponseType::Ok);
        ..get_content_area().add(&vbox);
        ..set_transient_for(window.as_ref());
        ..show_all();
    };

    let signal_id = board.connect_removed(clone!(@strong dialog => move || dialog.close()));

    let response = dialog.run_future().await;

    board.disconnect(signal_id);

    dialog.close();
    board.unblock_led_save();

    if response == gtk::ResponseType::Ok {
        Some(color_wheel.hs())
    } else {
        if let Err(err) = index.set_colors(&board, &original_colors).await {
            error!("Failed to set keyboard color: {}", err);
        }
        None
    }
}
