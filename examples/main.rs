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

extern crate gtk;
extern crate mg;
#[macro_use]
extern crate mg_settings;

use gtk::Label;
use mg::Application;
use mg_settings::Config;

use AppCommand::*;

commands!(AppCommand {
    Open(String),
    Quit,
});

fn main() {
    gtk::init().unwrap();

    let config = Config {
        mapping_modes: vec!["n".to_string()],
    };

    let app = Application::new_with_config(config);
    if let Err(error) = app.parse_config("main.conf") {
        // TODO: show in the status bar.
        println!("{:?}", error);
    }
    app.set_window_title("First Mg Program");

    let label = Label::new(Some("Mg App"));
    app.set_view(&label);

    app.connect_command(move |command| {
        match command {
            Open(url) => label.set_text(&format!("Opening URL {}", url)),
            Quit => gtk::main_quit(),
        }
    });

    gtk::main();
}