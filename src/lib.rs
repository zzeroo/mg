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

#![allow(let_and_return)]

/*
 * TODO: Show the current shortcut in the status bar.
 * TODO: Set a fixed height to the status bar.
 * TODO: Try to return an Application directly instead of an Rc<Application>.
 * TODO: support shortcuts with number like "50G".
 * TODO: Associate a color with modes.
 */

//! Minimal UI library based on GTK+.

#![warn(missing_docs)]

extern crate gdk;
extern crate gdk_sys;
extern crate glib;
extern crate gobject_sys;
extern crate gtk;
extern crate gtk_sys;
extern crate libc;
extern crate mg_settings;

mod key_converter;
mod gobject;
mod style_context;
#[macro_use]
mod widget;
mod status_bar;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::rc::Rc;

use gdk::{EventKey, RGBA, CONTROL_MASK};
use gdk::enums::key::{Escape, colon};
use gdk_sys::GdkRGBA;
use gtk::{ContainerExt, Grid, Inhibit, IsA, Settings, Widget, WidgetExt, Window, WindowExt, WindowType, STATE_FLAG_NORMAL};
use gtk::prelude::WidgetExtManual;
use mg_settings::{Config, EnumFromStr, Parser};
use mg_settings::Command::{Custom, Include, Map, Set, Unmap};
use mg_settings::error::{Error, Result};
use mg_settings::error::ErrorType::{MissingArgument, NoCommand, Parse, UnknownCommand};
use mg_settings::key::Key;

use key_converter::gdk_key_to_key;
use gobject::ObjectExtManual;
use self::ShortcutCommand::{Complete, Incomplete};
use status_bar::{StatusBar, StatusBarItem};
use style_context::StyleContextExtManual;

#[macro_export]
macro_rules! hash {
    ($($key:expr => $value:expr),* $(,)*) => {{
        let mut hashmap = std::collections::HashMap::new();
        $(hashmap.insert($key.into(), $value.into());)*
        hashmap
    }};
}

type Modes = HashMap<String, String>;

const RED: &'static GdkRGBA = &GdkRGBA { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
const TRANSPARENT: &'static GdkRGBA = &GdkRGBA { red: 0.0, green: 0.0, blue: 0.0, alpha: 0.0 };
const WHITE: &'static GdkRGBA = &GdkRGBA { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 };

/// A command from a map command.
#[derive(Debug)]
enum ShortcutCommand {
    /// A complete command that is to be executed.
    Complete(String),
    /// An incomplete command where the user needs to complete it and press Enter.
    Incomplete(String),
}

/// Create a new MG application window.
/// This window contains a status bar where the user can type a command and a central widget.
pub struct Application<T> {
    command_callback: RefCell<Option<Box<Fn(T)>>>,
    current_mode: RefCell<String>,
    current_shortcut: RefCell<Vec<Key>>,
    foreground_color: RefCell<RGBA>,
    mappings: RefCell<HashMap<String, HashMap<Vec<Key>, String>>>,
    modes: Modes,
    message: StatusBarItem,
    settings_parser: RefCell<Parser<T>>,
    status_bar: StatusBar,
    vbox: Grid,
    variables: RefCell<HashMap<String, Box<Fn() -> String>>>,
    window: Window,
}

impl<T: EnumFromStr + 'static> Application<T> {
    /// Create a new application.
    #[allow(new_without_default)]
    pub fn new() -> Rc<Self> {
        Application::new_with_config(hash!{})
    }

    /// Create a new application with configuration.
    pub fn new_with_config(modes: Modes) -> Rc<Self> {
        let config = Config {
            mapping_modes: modes.keys().cloned().collect()
        };
        let window = Window::new(WindowType::Toplevel);
        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        let grid = Grid::new();
        window.add(&grid);

        let status_bar = StatusBar::new();
        grid.attach(&status_bar, 0, 1, 1, 1);
        window.show_all();
        status_bar.hide();

        let foreground_color = Application::<T>::get_foreground_color(&window);

        let message = StatusBarItem::new().left();

        let app = Rc::new(Application {
            command_callback: RefCell::new(None),
            current_mode: RefCell::new("normal".to_string()),
            current_shortcut: RefCell::new(vec![]),
            foreground_color: RefCell::new(foreground_color),
            mappings: RefCell::new(HashMap::new()),
            modes: modes,
            message: message,
            settings_parser: RefCell::new(Parser::new_with_config(config)),
            status_bar: status_bar,
            vbox: grid,
            variables: RefCell::new(HashMap::new()),
            window: window,
        });

        app.status_bar.add_item(&app.message);

        {
            let instance = app.clone();
            app.status_bar.connect_activate(move |command| instance.handle_command(command));
        }

        {
            let instance = app.clone();
            app.window.connect_key_press_event(move |_, key| instance.key_press(key));
        }

        app
    }

    /// Convert an action String to a command String.
    fn action_to_command(&self, action: &str) -> ShortcutCommand {
        if let Some(':') = action.chars().next() {
            if let Some(index) = action.find("<Enter>") {
                Complete(action[1..index].to_string())
            }
            else {
                Incomplete(action[1..].to_string())
            }
        }
        else {
            Complete(action.to_string())
        }
    }

    /// Create a new status bar item.
    pub fn add_statusbar_item(&self) -> StatusBarItem {
        let item = StatusBarItem::new();
        self.status_bar.add_item(&item);
        item
    }

    /// Add the key to the current shortcut.
    fn add_to_shortcut(&self, key: Key) {
        let mut shortcut = self.current_shortcut.borrow_mut();
        shortcut.push(key);
    }

    /// Add a variable that can be used in mappings.
    /// The placeholder will be replaced by the value return by the function.
    pub fn add_variable<F: Fn() -> String + 'static>(&self, variable_name: &str, function: F) {
        let mut variables = self.variables.borrow_mut();
        variables.insert(variable_name.to_string(), Box::new(function));
    }

    /// Add a callback to the command event.
    pub fn connect_command<F: Fn(T) + 'static>(&self, callback: F) {
        *self.command_callback.borrow_mut() = Some(Box::new(callback));
    }

    /// Show an error to the user.
    pub fn error(&self, error: &str) {
        self.message.set_text(error);
        self.status_bar.override_background_color(STATE_FLAG_NORMAL, RED);
        self.status_bar.override_color(STATE_FLAG_NORMAL, WHITE);
    }

    /// Get the color of the text.
    fn get_foreground_color(window: &Window) -> RGBA {
        let style_context = window.get_style_context().unwrap();
        style_context.get_color(STATE_FLAG_NORMAL)
    }

    /// Handle the command activate event.
    fn handle_command(&self, command: Option<String>) {
        if let Some(command) = command {
            if let Some(ref callback) = *self.command_callback.borrow() {
                let result = self.settings_parser.borrow_mut().parse_line(&command);
                match result {
                    Ok(command) => {
                        match command {
                            Custom(command) => callback(command),
                            _ => unimplemented!(),
                        }
                    },
                    Err(error) => {
                        if let Some(error) = error.downcast_ref::<Error>() {
                            let message =
                                match error.typ {
                                    MissingArgument => "Argument required".to_string(),
                                    NoCommand => return,
                                    Parse => format!("Parse error: unexpected {}, expecting: {}", error.unexpected, error.expected),
                                    UnknownCommand => format!("Not a command: {}", error.unexpected),
                                };
                            self.set_mode("normal");
                            self.error(&message);
                        }
                    },
                }
            }
            self.status_bar.hide_entry();
        }
    }

    /// Handle a possible input of a shortcut.
    fn handle_shortcut(&self, key: &EventKey) -> Inhibit {
        if !self.status_bar.entry_shown() {
            let control_pressed = key.get_state() & CONTROL_MASK == CONTROL_MASK;
            if let Some(key) = gdk_key_to_key(key.get_keyval(), control_pressed) {
                self.add_to_shortcut(key);
                let action = {
                    let shortcut = self.current_shortcut.borrow();
                    let mappings = self.mappings.borrow();
                    mappings.get(&*self.current_mode.borrow())
                        .and_then(|mappings| mappings.get(&*shortcut).cloned())
                };
                if let Some(action) = action {
                    self.reset();
                    match self.action_to_command(&action) {
                        Complete(command) => self.handle_command(Some(command)),
                        Incomplete(command) => {
                            self.input_command(&command);
                            return Inhibit(true);
                        },
                    }
                }
                else if self.no_possible_shortcut() {
                    self.reset();
                }
            }
        }
        Inhibit(false)
    }

    /// Input the specified command.
    fn input_command(&self, command: &str) {
        self.status_bar.show_entry();
        let variables = self.variables.borrow();
        let mut command = command.to_string();
        for (variable, function) in variables.iter() {
            command = command.replace(&format!("<{}>", variable), &function());
        }
        let text: Cow<str> =
            if command.contains(' ') {
                command.into()
            }
            else {
                format!("{} ", command).into()
            };
        self.status_bar.set_command(&text);
    }

    /// Handle the key press event.
    #[allow(non_upper_case_globals)]
    fn key_press(&self, key: &EventKey) -> Inhibit {
        let mode = self.current_mode.borrow().clone();
        match mode.as_ref() {
            "normal" => {
                match key.get_keyval() {
                    colon => {
                        self.set_mode("command");
                        self.reset();
                        self.status_bar.show_entry();
                        Inhibit(true)
                    },
                    Escape => {
                        self.reset();
                        Inhibit(true)
                    },
                    _ => self.handle_shortcut(key),
                }
            },
            "command" => {
                match key.get_keyval() {
                    Escape => {
                        self.set_mode("normal");
                        self.reset();
                        Inhibit(true)
                    },
                    _ => self.handle_shortcut(key),
                }
            },
            _ => self.handle_shortcut(key)
        }
    }

    /// Check if there are no possible shortcuts.
    fn no_possible_shortcut(&self) -> bool {
        let current_shortcut = self.current_shortcut.borrow();
        let mappings = self.mappings.borrow();
        if let Some(mappings) = mappings.get(&*self.current_mode.borrow()) {
            for key in mappings.keys() {
                if key.starts_with(&*current_shortcut) {
                    return false;
                }
            }
        }
        true
    }

    /// Parse a configuration file.
    pub fn parse_config<P: AsRef<Path>>(&self, filename: P) -> Result<()> {
        let file = try!(File::open(filename));
        let buf_reader = BufReader::new(file);
        let commands = try!(self.settings_parser.borrow_mut().parse(buf_reader));
        for command in commands {
            match command {
                Custom(_) => (), // TODO: call the callback?
                Include(_) => (), // TODO: parse the included file.
                Map { action, keys, mode } => {
                    let mut mappings = self.mappings.borrow_mut();
                    let mappings = mappings.entry(self.modes[&mode].clone()).or_insert_with(HashMap::new);
                    mappings.insert(keys, action);
                },
                Set(_, _) => (), // TODO: set settings.
                Unmap { .. } => (), // TODO
            }
        }
        Ok(())
    }

    /// Handle the escape event.
    fn reset(&self) {
        self.status_bar.override_background_color(STATE_FLAG_NORMAL, TRANSPARENT);
        self.status_bar.override_color(STATE_FLAG_NORMAL, &self.foreground_color.borrow());
        self.status_bar.hide();
        self.show_mode();
        let mut shortcut = self.current_shortcut.borrow_mut();
        shortcut.clear();
    }

    /// Set the current mode.
    pub fn set_mode(&self, mode: &str) {
        *self.current_mode.borrow_mut() = mode.to_string();
        self.show_mode();
    }

    /// Set the main widget.
    pub fn set_view<W: IsA<Widget> + WidgetExt>(&self, view: &W) {
        view.set_hexpand(true);
        view.set_vexpand(true);
        view.show_all();
        self.vbox.attach(view, 0, 0, 1, 1);
    }

    /// Set the window title.
    pub fn set_window_title(&self, title: &str) {
        self.window.set_title(title);
    }

    /// Show the current mode if it is not the normal mode.
    fn show_mode(&self) {
        let mode = self.current_mode.borrow();
        if *mode != "normal" && *mode != "command" {
            self.message.set_text(&mode);
        }
        else {
            self.message.set_text("");
        }
    }

    /// Use the dark variant of the theme if available.
    pub fn use_dark_theme(&self) {
        let settings = Settings::get_default().unwrap();
        settings.set_data("gtk-application-prefer-dark-theme", 1);
        *self.foreground_color.borrow_mut() = Application::<T>::get_foreground_color(&self.window);
    }

    /// Get the application window.
    pub fn window(&self) -> &Window {
        &self.window
    }
}
