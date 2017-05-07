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

use gdk::RGBA;
use gtk;
use gtk::{
    BoxExt,
    CssProvider,
    EditableExt,
    EditableSignals,
    EntryExt,
    OrientableExt,
    PackType,
    WidgetExt,
    STATE_FLAG_NORMAL,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk::Orientation::Horizontal;
use pango::EllipsizeMode;
use relm::{Relm, Widget};
use relm_attributes::widget;

use self::Msg::*;

#[derive(Msg)]
pub enum Msg {
    EntryActivate(Option<String>),
    EntryChanged(Option<String>),
}

pub struct Model {
    identifier_label: &'static str,
    relm: Relm<StatusBar>,
}

#[widget]
impl Widget for StatusBar {
    fn init_view(&self) {
        // Adjust the look of the entry.
        let style_context = self.command_entry.get_style_context().unwrap();
        // TODO: remove the next line when relm supports css.
        let style = include_str!("../../style/command-input.css");
        let provider = CssProvider::new();
        provider.load_from_data(style).unwrap();
        style_context.add_provider(&provider, STYLE_PROVIDER_PRIORITY_APPLICATION);
    }

    fn model(relm: &Relm<Self>, _: ()) -> Model {
        Model {
            identifier_label: ":",
            relm: relm.clone(),
        }
    }

    fn update(&mut self, _msg: Msg) {
    }

    view! {
        #[container]
        gtk::Box {
            property_height_request: 20, // TODO: is this still useful?
            orientation: Horizontal,
            #[name="identifier_label"]
            gtk::Label {
                text: self.model.identifier_label,
            },
            #[name="command_entry"]
            gtk::Entry {
                activate(entry) => EntryActivate(entry.get_text()),
                changed(entry) => EntryChanged(entry.get_text()),
                has_frame: false,
                hexpand: true,
                name: "mg-input-command",
            },
        }
    }
}

impl StatusBar {
    /// Color the status bar in blue.
    pub fn color_blue(&self) {
        self.root().override_background_color(STATE_FLAG_NORMAL, &RGBA::blue());
        self.white_foreground();
    }

    /// Color the status bar in orange.
    pub fn color_orange(&self) {
        self.root().override_background_color(STATE_FLAG_NORMAL, &RGBA {
            red: 0.9,
            green: 0.55,
            blue: 0.0,
            alpha: 1.0 ,
        });
        self.white_foreground();
    }

    /// Color the status bar in red.
    pub fn color_red(&self) {
        self.root().override_background_color(STATE_FLAG_NORMAL, &RGBA::red());
        self.white_foreground();
    }

    /// Connect the active entry event.
    pub fn connect_activate<F: Fn(Option<String>) + 'static>(&self, callback: F) {
        self.command_entry.connect_activate(move |entry| callback(entry.get_text()));
    }

    /// Delete the word before the cursor.
    pub fn delete_previous_word(&self) {
        if self.command_entry.get_selection_bounds().is_some() {
            self.command_entry.delete_selection();
        }
        else if let Some(text) = self.command_entry.get_text() {
            if !text.is_empty() {
                let pos = self.command_entry.get_position();
                let upos = pos as usize;
                let mut end = upos;
                if end > 0 {
                    end -= 1;
                    let text: Vec<char> = text.chars().collect();
                    while text[end].is_whitespace() && end > 0 {
                        end -= 1;
                    }
                    let mut start = end;
                    while !text[start].is_whitespace() && start > 0 {
                        start -= 1;
                    }
                    if text[start].is_whitespace() && start >= upos {
                        start += 1;
                    }
                    let _lock = self.model.relm.stream().lock();
                    self.command_entry.delete_text(start as i32, pos);
                }
            }
        }
    }

    /// Get the text of the command entry.
    pub fn get_command(&self) -> Option<String> {
        self.command_entry.get_text()
    }

    /// Hide the entry.
    pub fn hide_entry(&self) {
        self.set_entry_shown(false);
    }

    // TODO: merge with show_entry()?
    /// Set whether the entry is visible or not.
    pub fn set_entry_shown(&self, visible: bool) {
        let _lock = self.model.relm.stream().lock();
        self.command_entry.set_text("");
        self.identifier_label.set_visible(visible);
        self.command_entry.set_visible(visible);
    }

    /// Seth the prefix identifier shown at the left of the command entry.
    pub fn set_identifier(&self, identifier: &str) {
        self.identifier_label.set_text(identifier);
    }

    /// Set the text of the input entry and move the cursor at the end.
    pub fn set_input(&self, command: &str) {
        // Prevent updating the completions when the user selects a completion entry.
        let _lock = self.model.relm.stream().lock();
        self.command_entry.set_text(command);
        self.command_entry.set_position(command.len() as i32);
    }

    /// Show the entry.
    pub fn show_entry(&self) {
        self.command_entry.grab_focus();
    }

    /// Set the foreground (text) color to white.
    pub fn white_foreground(&self) {
        self.root().override_color(STATE_FLAG_NORMAL, &RGBA::white());
    }
}

/// A status bar text item.
#[widget]
impl Widget for StatusBarItem {
    fn model() -> () {
        ()
    }

    fn update(&mut self, _msg: ()) {
    }

    view! {
        #[parent="status-bar-item"]
        #[name="label"]
        gtk::Label {
            ellipsize: EllipsizeMode::End,
            packing: {
                pack_type: PackType::End,
                padding: 3,
            },
        }
    }
}

impl StatusBarItem {
    /// Set the text of the status bar item.
    pub fn set_text(&self, text: &str) {
        self.label.set_text(text);
    }
}

/*
impl StatusBar {
    /// Add an item.
    pub fn add_item(&self, item: &StatusBarItem) {
        item.label.show();
        if item.left {
            self.hbox.pack_start(&**item, false, false, 3);
        }
        else {
            self.hbox.pack_end(&**item, false, false, 3);
        }
    }
}
*/
