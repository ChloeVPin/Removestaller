use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk4 as gtk;
use libadwaita as adw;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::backend::Registry;
use crate::model::App;
use crate::APP_NAME;

const BACKEND_ORDER: [&str; 10] = [
    "apt", "dnf", "zypper", "pacman", "flatpak", "snap", "appimage", "pip", "npm", "cargo",
];

/// Logical pixel size for app icons and the fallback initials avatar.
const ICON_SIZE: i32 = 40;

#[derive(Default)]
struct State {
    all_apps: Vec<App>,
    query: String,
    loaded: bool,
    scanning: bool,
    busy: bool,
    /// Key of the app currently being removed, if any.
    removing: Option<String>,
    /// App currently shown in the detail view, if any.
    detail: Option<App>,
    /// Backends that were present but could not be queried on the last scan.
    failed_backends: Vec<&'static str>,
}

#[derive(Clone)]
pub struct RemovestallerWindow {
    registry: Arc<Registry>,
    state: Rc<RefCell<State>>,
    window: adw::ApplicationWindow,
    groups_box: gtk::Box,
    search: gtk::SearchEntry,
    stack: gtk::Stack,
    spinner: gtk::Spinner,
    progress: gtk::ProgressBar,
    count_label: gtk::Label,
    toast_overlay: adw::ToastOverlay,
    icon_theme: Option<gtk::IconTheme>,
    detail_box: gtk::Box,
    back_button: gtk::Button,
}

impl RemovestallerWindow {
    pub fn new(app: &adw::Application, registry: Arc<Registry>) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(APP_NAME)
            .default_width(860)
            .default_height(700)
            .build();

        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&adw::WindowTitle::new(APP_NAME, "")));

        let back_button = gtk::Button::from_icon_name("go-previous-symbolic");
        back_button.set_tooltip_text(Some("Back to list"));
        back_button.set_visible(false);
        header.pack_start(&back_button);

        let progress = gtk::ProgressBar::new();
        progress.set_pulse_step(0.1);
        progress.set_size_request(140, -1);
        progress.set_valign(gtk::Align::Center);
        progress.set_visible(false);
        header.pack_end(&progress);

        let refresh_button = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some("Refresh"));
        header.pack_end(&refresh_button);

        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some("Main Menu"));
        let menu = gio::Menu::new();
        menu.append(Some("About Removestaller"), Some("app.about"));
        menu.append(Some("Quit"), Some("app.quit"));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search applications"));
        search.set_margin_start(18);
        search.set_margin_end(18);
        search.set_margin_top(14);

        let count_label = gtk::Label::new(None);
        count_label.add_css_class("dim-label");
        count_label.set_xalign(0.0);
        count_label.set_margin_start(18);
        count_label.set_margin_end(18);
        count_label.set_margin_top(8);

        let groups_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        scrolled.set_child(Some(&groups_box));

        let spinner = gtk::Spinner::new();
        spinner.set_size_request(48, 48);

        let loading_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        loading_page.set_vexpand(true);

        let spinner_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let left_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        left_spacer.set_hexpand(true);
        let right_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        right_spacer.set_hexpand(true);
        spinner_row.append(&left_spacer);
        spinner_row.append(&spinner);
        spinner_row.append(&right_spacer);

        let top_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        top_spacer.set_vexpand(true);
        let bottom_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bottom_spacer.set_vexpand(true);

        loading_page.append(&top_spacer);
        loading_page.append(&spinner_row);
        loading_page.append(&bottom_spacer);

        let empty_page = adw::StatusPage::new();
        empty_page.set_icon_name(Some("edit-find-symbolic"));
        empty_page.set_title("No applications found");
        empty_page.set_description(Some("Nothing matched your search."));

        let detail_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let detail_scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        detail_scrolled.set_child(Some(&detail_box));

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.add_named(&loading_page, Some("loading"));
        stack.add_named(&empty_page, Some("empty"));
        stack.add_named(&scrolled, Some("list"));
        stack.add_named(&detail_scrolled, Some("detail"));
        stack.set_visible_child_name("loading");

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&search);
        content.append(&count_label);
        content.append(&stack);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&content));

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar));

        window.set_content(Some(&toast_overlay));

        let icon_theme = gdk::Display::default().map(|d| gtk::IconTheme::for_display(&d));

        let this = Self {
            registry,
            state: Rc::new(RefCell::new(State::default())),
            window,
            groups_box,
            search,
            stack,
            spinner,
            progress,
            count_label,
            toast_overlay,
            icon_theme,
            detail_box,
            back_button: back_button.clone(),
        };

        let refresh_ctx = this.clone();
        refresh_button.connect_clicked(move |_| refresh_ctx.reload());

        let back_ctx = this.clone();
        back_button.connect_clicked(move |_| {
            back_ctx.show_list();
        });

        let search_this = this.clone();
        this.search.connect_search_changed(move |entry| {
            let loaded = {
                let mut state = search_this.state.borrow_mut();
                state.query = entry.text().to_string();
                state.loaded
            };
            if loaded {
                search_this.render();
            }
        });

        let search_key_ctx = this.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, state| {
            if state.contains(gdk::ModifierType::CONTROL_MASK) && key == gdk::Key::f {
                search_key_ctx.search.grab_focus();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        this.window.add_controller(key_controller);

        let pulse_ctx = this.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            let state = pulse_ctx.state.borrow();
            if state.busy || state.scanning {
                pulse_ctx.progress.pulse();
            }
            glib::ControlFlow::Continue
        });

        this.reload();
        this
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn show_detail(&self, app: App) {
        self.state.borrow_mut().detail = Some(app);
        self.back_button.set_visible(true);
        self.search.set_visible(false);
        self.count_label.set_visible(false);
        self.rebuild_detail(false);
        self.stack.set_visible_child_name("detail");
    }

    fn show_list(&self) {
        self.leave_detail();
        self.render();
    }

    fn leave_detail(&self) {
        self.state.borrow_mut().detail = None;
        self.back_button.set_visible(false);
        self.search.set_visible(true);
        self.count_label.set_visible(true);
    }

    fn rebuild_detail(&self, busy: bool) {
        let Some(app) = self.state.borrow().detail.clone() else {
            return;
        };

        while let Some(child) = self.detail_box.first_child() {
            self.detail_box.remove(&child);
        }

        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(480);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_margin_top(32);
        content.set_margin_bottom(32);
        content.set_margin_start(24);
        content.set_margin_end(24);

        let icon = self.icon_widget(&app, 96);
        let icon_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        icon_row.set_halign(gtk::Align::Center);
        icon_row.append(&icon);

        let name_label = gtk::Label::new(Some(&app.name));
        name_label.add_css_class("title-1");
        name_label.set_halign(gtk::Align::Center);
        name_label.set_justify(gtk::Justification::Center);
        name_label.set_wrap(true);
        name_label.set_margin_top(16);

        let source_label = gtk::Label::new(Some(backend_label(app.backend)));
        source_label.add_css_class("dim-label");
        source_label.set_halign(gtk::Align::Center);
        source_label.set_margin_top(4);

        let info_list = gtk::ListBox::new();
        info_list.add_css_class("boxed-list");
        info_list.set_selection_mode(gtk::SelectionMode::None);
        info_list.set_margin_top(24);

        let version = app
            .version
            .clone()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "Not available".to_string());
        info_list.append(&info_row("Version", &version));
        info_list.append(&info_row("Package", &app.id));
        info_list.append(&info_row("Source", backend_label(app.backend)));

        let button_text = if busy {
            "Removing...".to_string()
        } else {
            format!("Remove {}", app.name)
        };
        let spinner = gtk::Spinner::new();
        let button_label = gtk::Label::new(Some(&button_text));
        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        button_box.append(&spinner);
        button_box.append(&button_label);

        let remove_button = gtk::Button::new();
        remove_button.add_css_class("destructive-action");
        remove_button.set_halign(gtk::Align::Center);
        remove_button.set_margin_top(24);
        remove_button.set_child(Some(&button_box));
        remove_button.set_sensitive(!busy);

        if busy {
            spinner.set_spinning(true);
            spinner.set_visible(true);
        } else {
            spinner.set_visible(false);
        }

        let ctx = self.clone();
        let remove_app = app.clone();
        remove_button.connect_clicked(move |_| {
            ctx.request_remove(remove_app.clone());
        });

        content.append(&icon_row);
        content.append(&name_label);
        content.append(&source_label);
        content.append(&info_list);
        content.append(&remove_button);

        clamp.set_child(Some(&content));
        self.detail_box.append(&clamp);
    }

    fn reload(&self) {
        let initial_load = {
            let mut state = self.state.borrow_mut();
            if state.scanning || state.busy {
                return;
            }
            state.scanning = true;
            !state.loaded
        };
        if !initial_load {
            self.progress.set_visible(true);
        }
        self.spinner.set_spinning(true);
        let registry = self.registry.clone();
        let ctx = glib::thread_guard::ThreadGuard::new(self.clone());
        let main_context = glib::MainContext::default();

        std::thread::spawn(move || {
            let report = registry.list_all();
            main_context.invoke(move || {
                let ctx = ctx.into_inner();
                ctx.spinner.set_spinning(false);
                ctx.progress.set_visible(false);
                let failed_backends: Vec<&'static str> = report
                    .failures
                    .iter()
                    .map(|failure| failure.backend)
                    .collect();
                let failed_labels = failed_backends
                    .iter()
                    .map(|id| backend_label(id))
                    .collect::<Vec<_>>()
                    .join(", ");
                {
                    let mut state = ctx.state.borrow_mut();
                    state.all_apps = report.apps;
                    state.failed_backends = failed_backends;
                    state.loaded = true;
                    state.scanning = false;
                }
                ctx.render();
                if !failed_labels.is_empty() {
                    ctx.toast_overlay.add_toast(adw::Toast::new(&format!(
                        "Could not check: {failed_labels}"
                    )));
                }
            });
        });
    }

    fn render(&self) {
        let filtered: Vec<App> = {
            let state = self.state.borrow();
            let query = state.query.trim().to_lowercase();
            state
                .all_apps
                .iter()
                .filter(|a| {
                    query.is_empty()
                        || a.name.to_lowercase().contains(&query)
                        || a.id.to_lowercase().contains(&query)
                })
                .cloned()
                .collect()
        };

        while let Some(child) = self.groups_box.first_child() {
            self.groups_box.remove(&child);
        }

        let (n, failed_count) = (filtered.len(), self.state.borrow().failed_backends.len());
        let application_label = if n == 1 {
            "application"
        } else {
            "applications"
        };
        let mut count_text = format!("{} {application_label}", comma(n));
        if failed_count > 0 {
            let source_label = if failed_count == 1 {
                "source unavailable"
            } else {
                "sources unavailable"
            };
            count_text.push_str(&format!(", {failed_count} {source_label}"));
        }
        self.count_label.set_label(&count_text);

        if filtered.is_empty() {
            self.stack.set_visible_child_name("empty");
            return;
        }

        self.stack.set_visible_child_name("list");

        let mut groups: Vec<(&'static str, Vec<App>)> = Vec::new();
        for app in filtered {
            if let Some((_, apps)) = groups.iter_mut().find(|(id, _)| *id == app.backend) {
                apps.push(app);
            } else {
                groups.push((app.backend, vec![app]));
            }
        }
        groups.sort_by_key(|(id, _)| order_index(id));

        for (id, apps) in groups {
            self.groups_box.append(&make_header(id, apps.len()));
            self.groups_box.append(&self.make_list(&apps));
        }
    }

    fn make_list(&self, apps: &[App]) -> gtk::ListBox {
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();
        list.add_css_class("boxed-list");
        list.set_margin_start(18);
        list.set_margin_end(18);
        list.set_margin_bottom(12);

        for app in apps {
            list.append(&self.make_row(app));
        }
        list
    }

    fn make_row(&self, app: &App) -> adw::ActionRow {
        let leading = self.icon_widget(app, ICON_SIZE);

        let subtitle = match &app.version {
            Some(v) if !v.is_empty() => v.clone(),
            _ => String::new(),
        };

        let key = app_key(app);
        let busy = {
            let state = self.state.borrow();
            state.busy
        };
        let is_removing = self.state.borrow().removing.as_deref() == Some(key.as_str());
        let remove_button =
            gtk::Button::with_label(if is_removing { "Removing..." } else { "Remove" });
        remove_button.set_valign(gtk::Align::Center);
        remove_button.set_sensitive(!busy);

        let row = adw::ActionRow::builder()
            .title(&app.name)
            .subtitle(&subtitle)
            .activatable(true)
            .build();
        row.set_title_lines(1);
        row.set_subtitle_lines(1);
        row.add_prefix(&leading);
        row.add_suffix(&remove_button);

        let remove_app = app.clone();
        let remove_ctx = self.clone();
        remove_button.connect_clicked(move |_| {
            remove_ctx.request_remove(remove_app.clone());
        });

        let detail_app = app.clone();
        let detail_ctx = self.clone();
        row.connect_activated(move |_| detail_ctx.show_detail(detail_app.clone()));

        row
    }

    fn icon_widget(&self, app: &App, size: i32) -> gtk::Widget {
        // Snap and other backends may store an absolute path to the icon file.
        if let Some(icon) = &app.icon {
            if icon.starts_with('/') {
                if let Ok(texture) = gdk::Texture::from_file(&gio::File::for_path(icon)) {
                    let picture = gtk::Picture::for_paintable(&texture);
                    picture.set_content_fit(gtk::ContentFit::Contain);
                    picture.set_can_shrink(true);
                    picture.set_size_request(size, size);
                    return picture.upcast();
                }
            }
        }

        // Prefer the backend's explicit icon name, then fall back to the app
        // id and display name.
        let mut candidates: Vec<String> = Vec::new();
        if let Some(icon) = &app.icon {
            if !icon.starts_with('/') {
                candidates.push(icon.clone());
            }
        }
        candidates.push(app.id.clone());
        candidates.push(app.name.clone());

        if let Some(theme) = &self.icon_theme {
            for name in candidates {
                if name.is_empty() {
                    continue;
                }
                if let Some(image) = themed_image(theme, &name, size) {
                    return image.upcast();
                }
                let lower = name.to_lowercase();
                if lower != name {
                    if let Some(image) = themed_image(theme, &lower, size) {
                        return image.upcast();
                    }
                }
            }
        }

        // Fall back to an initials avatar.
        adw::Avatar::new(size, Some(app.name.as_str()), true).upcast()
    }

    fn request_remove(&self, app: App) {
        if self.state.borrow().busy {
            return;
        }
        let body = format!(
            "This will remove {} from {}. This action cannot be undone.",
            app.name,
            backend_label(app.backend)
        );

        let dialog = adw::AlertDialog::builder()
            .heading(format!("Remove {}?", app.name))
            .body(body)
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("remove", "Remove");
        dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let ctx = self.clone();
        dialog.choose(
            Some(&self.window),
            gio::Cancellable::NONE,
            move |response| {
                if response.as_str() == "remove" {
                    ctx.do_remove(app);
                }
            },
        );
    }

    fn set_busy(&self, busy: bool) {
        self.state.borrow_mut().busy = busy;
        self.progress.set_visible(busy);
    }

    fn do_remove(&self, app: App) {
        self.set_busy(true);
        self.state.borrow_mut().removing = Some(app_key(&app));

        let in_detail = self.state.borrow().detail.is_some();
        if in_detail {
            self.rebuild_detail(true);
        } else {
            self.render();
        }

        self.toast_overlay
            .add_toast(adw::Toast::new(&format!("Removing {}...", app.name)));

        let registry = self.registry.clone();
        let ctx = glib::thread_guard::ThreadGuard::new(self.clone());
        let main_context = glib::MainContext::default();

        std::thread::spawn(move || {
            let result = registry.remove(&app).map_err(|e| e.0);

            main_context.invoke(move || {
                let ctx = ctx.into_inner();
                ctx.set_busy(false);
                ctx.state.borrow_mut().removing = None;
                let in_detail = ctx.state.borrow().detail.is_some();
                match result {
                    Ok(()) => {
                        ctx.toast_overlay
                            .add_toast(adw::Toast::new(&format!("Removed {}", app.name)));
                        // Drop the app immediately, then re-poll every backend
                        // so the list reflects what is installed.
                        let key = app_key(&app);
                        ctx.state
                            .borrow_mut()
                            .all_apps
                            .retain(|a| app_key(a) != key);
                        ctx.leave_detail();
                        ctx.render();
                        ctx.reload();
                    }
                    Err(e) => {
                        ctx.toast_overlay.add_toast(adw::Toast::new(&format!(
                            "Failed to remove {}: {e}",
                            app.name
                        )));
                        if in_detail {
                            ctx.rebuild_detail(false);
                        } else {
                            ctx.render();
                        }
                    }
                }
            });
        });
    }
}

/// A themed icon rendered at ICON_SIZE, using pixel-size so it respects the
/// display scale factor (unlike a manually built IconPaintable at scale 1).
fn themed_image(theme: &gtk::IconTheme, name: &str, size: i32) -> Option<gtk::Image> {
    if theme.has_icon(name) {
        let image = gtk::Image::from_icon_name(name);
        image.set_pixel_size(size);
        Some(image)
    } else {
        None
    }
}

fn make_header(id: &str, count: usize) -> gtk::Box {
    let title = gtk::Label::new(None);
    title.set_markup(&format!(
        "<b>{}</b>",
        glib::markup_escape_text(backend_label(id))
    ));
    title.set_xalign(0.0);
    title.set_hexpand(true);

    let count_label = gtk::Label::new(Some(&count.to_string()));
    count_label.add_css_class("dim-label");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.set_margin_start(18);
    header.set_margin_end(18);
    header.set_margin_top(16);
    header.set_margin_bottom(8);
    header.append(&title);
    header.append(&count_label);
    header
}

fn backend_label(id: &str) -> &'static str {
    match id {
        "apt" => "APT",
        "dnf" => "DNF",
        "zypper" => "Zypper",
        "pacman" => "Pacman",
        "flatpak" => "Flatpak",
        "snap" => "Snap",
        "appimage" => "AppImage",
        "pip" => "Python",
        "npm" => "Node.js",
        "cargo" => "Rust",
        _ => "Other",
    }
}

fn order_index(id: &str) -> usize {
    BACKEND_ORDER
        .iter()
        .position(|&b| b == id)
        .unwrap_or(usize::MAX)
}

fn app_key(app: &App) -> String {
    format!("{}::{}", app.backend, app.id)
}

fn info_row(title: &str, value: &str) -> gtk::ListBoxRow {
    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("dim-label");
    title_label.set_xalign(0.0);
    title_label.set_hexpand(true);

    let value_label = gtk::Label::new(Some(value));
    value_label.set_xalign(1.0);
    value_label.set_selectable(true);
    value_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);

    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row_box.set_margin_top(10);
    row_box.set_margin_bottom(10);
    row_box.set_margin_start(14);
    row_box.set_margin_end(14);
    row_box.append(&title_label);
    row_box.append(&value_label);

    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&row_box));
    row
}

fn comma(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
