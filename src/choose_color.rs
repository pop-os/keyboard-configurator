use cascade::cascade;
use glib::clone;
use gtk::prelude::*;
use std::rc::Rc;

use crate::ColorWheel;
use backend::{Board, Hs, Mode};

pub fn choose_color<W: IsA<gtk::Widget>, F: Fn(Option<Hs>) + 'static>(
    board: Board,
    w: &W,
    title: &'static str,
    color: Option<Hs>,
    cb: F,
) {
    let cb = Rc::new(cb);

    let original_color = board.layers()[0].color();
    let original_mode = board.layers()[0].mode();

    if original_mode.is_some() {
        glib::MainContext::default().spawn_local(clone!(@strong board => async move {
            if let Err(err) = board.layers()[0].set_mode(&Mode::all()[0], 0).await {
                error!("Failed to set keyboard mode: {}", err);
            }
        }));
    }

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
        clone!(@strong board, @weak preview => @default-panic, move |wheel| {
            glib::MainContext::default().spawn_local(clone!(@strong board, @strong wheel => async move {
                if let Err(err) = board.layers()[0].set_color(wheel.hs()).await {
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
        ..set_margin_start(24);
        ..set_margin_end(24);
        ..set_margin_top(24);
        ..set_margin_bottom(24);
        ..add(&color_wheel);
        ..add(&preview);
        ..add(&hue_box);
        ..add(&saturation_box);
    };

    let window = w
        .get_toplevel()
        .and_then(|x| x.downcast::<gtk::Window>().ok());

    cascade! {
        gtk::DialogBuilder::new()
            .title(title)
            .use_header_bar(1)
            .modal(true)
            .build();
        ..add_button("Cancel", gtk::ResponseType::Cancel);
        ..add_button("Save", gtk::ResponseType::Ok);
        ..get_content_area().add(&vbox);
        ..set_transient_for(window.as_ref());
        ..connect_response(move |dialog, response| {
            if response == gtk::ResponseType::DeleteEvent {
                return;
            }

            let hs = color_wheel.hs();
            dialog.close();

            glib::MainContext::default().spawn_local(clone!(@strong board, @strong cb => async move {
                let layer = &board.layers()[0];

                if let Err(err) = layer.set_color(original_color).await {
                    error!("Failed to set keyboard color: {}", err);
                }
                if let Some(mode) = original_mode {
                    if let Err(err) = layer.set_mode(mode.0, mode.1).await {
                        error!("Failed to set keyboard mode: {}", err);
                    }
                }

                let cb = cb.clone();
                cb(if response == gtk::ResponseType::Ok {
                    Some(hs)
                } else {
                    None
                })
            }));
        });
        ..show_all();
    };
}
