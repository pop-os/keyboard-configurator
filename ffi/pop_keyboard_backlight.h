#pragma once

#include <gtk/gtk.h>

typedef struct { } PopKeyboardColorButton;

PopKeyboardColorButton *pop_keyboard_color_button_new (void);

GtkWidget *pop_keyboard_color_button_widget (const PopKeyboardColorButton *self);

void pop_keyboard_color_button_free (PopKeyboardColorButton *self);
