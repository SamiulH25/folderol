//! The GTK4 shell.
//!
//! This is intentionally a thin first slice: it proves the catalog can be read
//! from a native window and that search feels instant. The full interaction
//! design lives in `DESIGN.md`; milestone 2 turns this into the real browser
//! (grid view, preview inspector, command palette, thumbnails).

use std::env;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use folderol_core::Category;
use folderol_index::{Catalog, IndexedFile, SearchWeights};
use folderol_ml::HashingEmbedder;
use gtk::prelude::*;
use gtk::{glib, pango};
use libadwaita as adw;

const APP_ID: &str = "io.github.samiulh25.Folderol";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| install_css());
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let db_path = default_db_path();
    let catalog = match Catalog::open(&db_path) {
        Ok(catalog) => catalog,
        Err(error) => {
            eprintln!(
                "could not open {}: {error}\nstart with: cargo run -p folderol-cli -- scan ~",
                db_path.display()
            );
            Catalog::open_in_memory().expect("in-memory catalog")
        }
    };
    let catalog = Rc::new(catalog);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Folderol")
        .default_width(1180)
        .default_height(760)
        .build();

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search files, or describe what you remember…")
        .width_chars(38)
        .build();

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&search));
    header.pack_end(&gtk::Button::from_icon_name("view-refresh-symbolic"));

    // Sidebar: places + categories from the catalog's own statistics.
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sidebar.add_css_class("sidebar");
    sidebar.set_width_request(248);

    let brand = gtk::Label::new(Some("Folderol"));
    brand.add_css_class("brand");
    brand.set_xalign(0.0);
    sidebar.append(&brand);

    let tagline = gtk::Label::new(Some("files, with memory"));
    tagline.add_css_class("dim-label");
    tagline.set_xalign(0.0);
    sidebar.append(&tagline);

    let places = gtk::ListBox::new();
    places.add_css_class("navigation-sidebar");
    let all_row = gtk::ListBoxRow::new();
    all_row.set_child(Some(&nav_label("All files", None)));
    places.append(&all_row);

    let stats = catalog.stats().unwrap_or_default();
    let categories: Rc<Vec<(Category, usize)>> = Rc::new(
        stats
            .iter()
            .filter(|(category, _)| *category != Category::Other)
            .cloned()
            .collect(),
    );

    for (category, count) in categories.iter() {
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&nav_label(category.label(), Some(*count))));
        places.append(&row);
    }

    sidebar.append(&section_label("Categories"));
    sidebar.append(&places);

    // Main list.
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let list = gtk::ListBox::new();
    list.add_css_class("file-list");
    list.set_selection_mode(gtk::SelectionMode::Single);

    let scroller = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();

    let status = gtk::Label::new(None);
    status.add_css_class("status");
    status.set_xalign(0.0);
    status.set_margin_start(18);
    status.set_margin_end(18);
    status.set_margin_top(8);
    status.set_margin_bottom(8);

    content.append(&scroller);
    content.append(&status);

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_start_child(Some(&sidebar));
    paned.set_end_child(Some(&content));
    paned.set_position(248);
    paned.set_resize_start_child(false);
    paned.set_shrink_start_child(false);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&paned));
    window.set_content(Some(&toolbar));

    let catalog_for_search = Rc::clone(&catalog);
    let list_for_search = list.clone();
    let status_for_search = status.clone();
    let categories_for_search = Rc::clone(&categories);
    search.connect_search_changed(move |entry| {
        let query = entry.text().to_string();
        let files = if query.trim().is_empty() {
            catalog_for_search.all(500).unwrap_or_default()
        } else {
            let embedder = HashingEmbedder::default();
            catalog_for_search
                .search(&query, &embedder, SearchWeights::default(), 80)
                .unwrap_or_default()
                .into_iter()
                .map(|hit| hit.file)
                .collect()
        };
        fill_list(&list_for_search, &files);
        status_for_search.set_text(&format!(
            "{} file{} · {}",
            files.len(),
            if files.len() == 1 { "" } else { "s" },
            if query.trim().is_empty() {
                "recent"
            } else {
                "ranked by meaning + words"
            }
        ));
        let _ = &categories_for_search;
    });

    let catalog_for_selection = Rc::clone(&catalog);
    let list_for_selection = list.clone();
    let status_for_selection = status.clone();
    let categories_for_selection = Rc::clone(&categories);
    places.connect_row_selected(move |_, row| {
        let Some(row) = row else { return };
        let index = row.index();
        let files = if index <= 0 {
            catalog_for_selection.all(500).unwrap_or_default()
        } else {
            let category = categories_for_selection[(index - 1) as usize].0;
            catalog_for_selection
                .all_with_embeddings()
                .unwrap_or_default()
                .into_iter()
                .filter(|file| file.category == category)
                .collect()
        };
        fill_list(&list_for_selection, &files);
        status_for_selection.set_text(&format!("{} files", files.len()));
    });

    let files = catalog.all(500).unwrap_or_default();
    fill_list(&list, &files);
    status.set_text(&format!("{} files in {}", files.len(), db_path.display()));

    window.present();
}

fn fill_list(list: &gtk::ListBox, files: &[IndexedFile]) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if files.is_empty() {
        let empty = gtk::Label::new(Some("Nothing here yet.\nRun the scanner: folderol scan ~"));
        empty.add_css_class("empty-state");
        empty.set_justify(gtk::Justification::Center);
        let row = gtk::ListBoxRow::new();
        row.set_activatable(false);
        row.set_child(Some(&empty));
        list.append(&row);
        return;
    }

    for file in files {
        list.append(&file_row(file));
    }
}

fn file_row(file: &IndexedFile) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("file-row");

    let container = gtk::Box::new(gtk::Orientation::Vertical, 3);
    container.set_margin_top(9);
    container.set_margin_bottom(9);
    container.set_margin_start(14);
    container.set_margin_end(14);

    let name = gtk::Label::new(Some(&file.name));
    name.add_css_class("file-name");
    name.set_xalign(0.0);
    name.set_ellipsize(pango::EllipsizeMode::Middle);

    let parent = file
        .path
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_default();

    let meta = gtk::Label::new(Some(&format!(
        "{}  ·  {:.0}%  ·  {}",
        file.category.label(),
        file.confidence * 100.0,
        parent
    )));
    meta.add_css_class("file-meta");
    meta.set_xalign(0.0);
    meta.set_ellipsize(pango::EllipsizeMode::Middle);

    container.append(&name);
    container.append(&meta);
    row.set_child(Some(&container));
    row
}

fn nav_label(title: &str, count: Option<usize>) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    container.set_margin_top(7);
    container.set_margin_bottom(7);
    container.set_margin_start(10);
    container.set_margin_end(10);

    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.set_hexpand(true);

    container.append(&label);

    if let Some(count) = count {
        let badge = gtk::Label::new(Some(&count.to_string()));
        badge.add_css_class("badge");
        container.append(&badge);
    }

    container
}

fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("section-label");
    label.set_xalign(0.0);
    label.set_margin_top(18);
    label.set_margin_start(10);
    label
}

fn install_css() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };

    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        .brand { font-size: 20px; font-weight: 800; letter-spacing: -0.4px; }
        .sidebar { background: alpha(@sidebar_bg_color, 0.55); padding: 14px 8px; }
        .section-label { font-size: 11px; font-weight: 700; opacity: 0.55;
                         letter-spacing: 0.8px; text-transform: uppercase; }
        .navigation-sidebar row { border-radius: 9px; margin: 1px 4px; }
        .badge { opacity: 0.55; font-size: 11px; }
        .file-list { background: transparent; padding: 10px 12px; }
        .file-row { border-radius: 12px; margin: 2px 6px; }
        .file-row:hover { background: alpha(@accent_bg_color, 0.10); }
        .file-name { font-size: 14px; font-weight: 600; }
        .file-meta { font-size: 11px; opacity: 0.55; }
        .empty-state { padding: 80px 20px; opacity: 0.6; font-size: 15px; }
        .status { font-size: 11px; opacity: 0.5; }
        ",
    );

    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn default_db_path() -> PathBuf {
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data_home).join("folderol/index.sqlite3");
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/folderol/index.sqlite3")
}
