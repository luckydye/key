mod app;
mod browser;
mod config;
mod library;
mod settings;
mod unlock;
mod vault;

use gpui::{prelude::*, px, size, Bounds, WindowBounds, WindowOptions};
use gpui_component::{Root, Theme, TitleBar};

fn main() {
  gpui_platform::application()
    .with_assets(gpui_kit_assets::AllAssets)
    .run(|cx| {
      gpui_component::init(cx);
      let bounds = Bounds::centered(None, size(px(1000.), px(640.)), cx);
      let window_options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..TitleBar::window_options()
      };
      cx.open_window(window_options, |window, cx| {
        Theme::sync_system_appearance(Some(window), cx);
        let library = cx.new(|cx| library::Library::load(config::config_path(), cx));
        let view = cx.new(|cx| app::KeyApp::new(library, window, cx));
        cx.new(|cx| Root::new(view, window, cx))
      })
      .expect("failed to open main window");
      cx.activate(true);
    });
}
