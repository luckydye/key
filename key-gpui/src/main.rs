mod app;
mod vault;

use gpui::{prelude::*, px, size, Bounds, WindowBounds, WindowOptions};
use gpui_component::{Root, Theme};
use key::db::KeeOptions;

fn main() {
  // Panics with "Missing db url" when KEY_DATABASE_URL is not set.
  let options = KeeOptions::from(std::env::vars());

  gpui_platform::application()
    .with_assets(gpui_kit_assets::Assets)
    .run(move |cx| {
      gpui_component::init(cx);
      let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
      let window_options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      };
      cx.open_window(window_options, |window, cx| {
        Theme::sync_system_appearance(Some(window), cx);
        let vault = cx.new(|_| vault::Vault::new(options));
        let view = cx.new(|cx| app::KeyApp::new(vault, window, cx));
        cx.new(|cx| Root::new(view, window, cx))
      })
      .expect("failed to open main window");
      cx.activate(true);
    });
}
