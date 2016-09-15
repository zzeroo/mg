/*
 * Copyright (c) 2016 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use std::cell::Cell;

use gtk::{ContainerExt, EditableExt, Entry, EntryExt, Label, WidgetExt};
use gtk::Orientation::Horizontal;

pub type StatusBarWidget = HBox;
pub type HBox = ::gtk::Box;

/// The window status bar.
pub struct StatusBar {
    colon_label: Label,
    entry: Entry,
    entry_shown: Cell<bool>,
    hbox: HBox,
    message_label: Label,
}

impl StatusBar {
    /// Create a new status bar.
    pub fn new() -> Self {
        let hbox = HBox::new(Horizontal, 0);

        let message_label = Label::new(None);
        hbox.add(&message_label);

        let colon_label = Label::new(Some(":"));
        hbox.add(&colon_label);

        let entry = Entry::new();
        entry.set_has_frame(false);
        entry.set_hexpand(true);
        hbox.add(&entry);

        StatusBar {
            colon_label: colon_label,
            entry: entry,
            entry_shown: Cell::new(false),
            hbox: hbox,
            message_label: message_label,
        }
    }

    /// Connect the active entry event.
    pub fn connect_activate<F: Fn(Option<String>) + 'static>(&self, callback: F) {
        self.entry.connect_activate(move |entry| callback(entry.get_text()));
    }

    /// Get whether the entry is shown or not.
    pub fn entry_shown(&self) -> bool {
        self.entry_shown.get()
    }

    /// Hide all the widgets.
    pub fn hide(&self) {
        self.hide_entry();
        self.message_label.hide();
    }

    /// Hide the entry.
    pub fn hide_entry(&self) {
        self.entry_shown.set(false);
        self.entry.hide();
        self.colon_label.hide();
    }

    /// Set the text of the command entry.
    pub fn set_command(&self, command: &str) {
        self.entry.set_text(command);
        self.entry.set_position(command.len() as i32);
    }

    /// Show the entry.
    pub fn show_entry(&self) {
        self.entry_shown.set(true);
        self.entry.set_text("");
        self.entry.show();
        self.entry.grab_focus();
        self.colon_label.show();
        self.message_label.hide();
    }

    /// Show an error message.
    pub fn show_error(&self, message: &str) {
        // TODO: show a red background.
        self.message_label.set_text(message);
        self.message_label.show();
    }
}

is_widget!(StatusBar, hbox);