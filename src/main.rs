mod backend;
mod backends;
mod model;
mod window;

use std::sync::Arc;

use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;
use gtk::gio;

const APP_ID: &str = "io.github.chloevpin.Removestaller";
const APP_NAME: &str = "Removestaller";
const APP_VERSION: &str = "1.0.1";

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();

    let about_action = gio::SimpleAction::new("about", None);
    let about_app = app.downgrade();
    about_action.connect_activate(move |_, _| {
        if let Some(app) = about_app.upgrade() {
            show_about(&app);
        }
    });
    app.add_action(&about_action);

    let quit_action = gio::SimpleAction::new("quit", None);
    let quit_app = app.downgrade();
    quit_action.connect_activate(move |_, _| {
        if let Some(app) = quit_app.upgrade() {
            app.quit();
        }
    });
    app.add_action(&quit_action);
    app.set_accels_for_action("app.quit", &["<primary>q"]);

    let registry = Arc::new(backends::build_registry());

    app.connect_activate(move |app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        let window = window::RemovestallerWindow::new(app, registry.clone());
        window.present();
    });

    app.run()
}

fn show_about(app: &adw::Application) {
    let dialog = adw::AboutDialog::builder()
        .application_name(APP_NAME)
        .application_icon(APP_ID)
        .version(APP_VERSION)
        .developer_name("chloevpin")
        .license("GPL-3.0-or-later")
        .comments("Remove installed applications.")
        .build();

    if let Some(window) = app.active_window() {
        dialog.present(Some(&window));
    }
}
