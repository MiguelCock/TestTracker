#![deny(unsafe_code)]

mod app;
mod backend;
mod format;
mod theme;
mod ui;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::gpui::{AppContext as _, WindowBounds, WindowOptions, px, size};

use crate::app::RootView;
use crate::backend::Backend;

fn main() {
    let backend = Backend::new().unwrap_or_else(|error| {
        eprintln!("Unable to open TestTracker: {error}");
        std::process::exit(1);
    });
    let session = backend.state.session_info();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);

            let backend = backend.clone();
            let session = session.clone();
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(None, size(px(1440.), px(900.)), cx)),
                titlebar: Some(gpui_kit::gpui::TitlebarOptions {
                    title: Some("TestTracker".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(900.), px(600.))),
                app_id: Some("com.testtracker.desktop".into()),
                ..Default::default()
            };
            let root = gpui_kit::open_window(options, cx, move |window, cx| {
                cx.new(|cx| RootView::new(backend.clone(), session.clone(), window, cx))
            })
            .expect("failed to open TestTracker window");
            let _ = root;
        });
}
