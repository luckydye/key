use gpui::{div, prelude::*, px, AnyElement, Context, Entity, SharedString, Subscription, Window};
use gpui_component::{
  button::{Button, ButtonVariants},
  h_flex,
  menu::{DropdownMenu, PopupMenuItem},
  v_flex, ActiveTheme, Icon, Sizable, TitleBar,
};
use gpui_kit_assets::IconName;

use crate::{
  browser::EntryBrowser,
  library::Library,
  settings::{DatabaseSettings, SettingsEvent},
  unlock::UnlockForm,
  vault::{Status, Vault},
};

pub enum Screen {
  Empty,
  Unlock(Entity<UnlockForm>),
  Browse(Entity<EntryBrowser>),
  Settings(Entity<DatabaseSettings>),
}

/// The selected database below a header to switch databases.
pub struct KeyApp {
  pub library: Entity<Library>,
  pub selected: Option<Entity<Vault>>,
  pub screen: Screen,
  pub _subscriptions: Vec<Subscription>,
  /// Subscriptions to the selected vault and the current screen.
  pub screen_subscriptions: Vec<Subscription>,
}

impl KeyApp {
  pub fn new(library: Entity<Library>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let mut app = Self {
      _subscriptions: vec![cx.observe(&library, |_, _, cx| cx.notify())],
      library,
      selected: None,
      screen: Screen::Empty,
      screen_subscriptions: Vec::new(),
    };
    match app.library.read(cx).vaults.first().cloned() {
      Some(vault) => app.open(vault, window, cx),
      None => app.settings(None, window, cx),
    }
    app
  }

  pub fn open(&mut self, vault: Entity<Vault>, window: &mut Window, cx: &mut Context<Self>) {
    self.screen_subscriptions = vec![cx.observe_in(&vault, window, |this, vault, window, cx| {
      let status_changed = match (&this.screen, &vault.read(cx).status) {
        (Screen::Unlock(_), Status::Unlocked) | (Screen::Browse(_), Status::Locked) => true,
        _ => false,
      };
      if status_changed {
        this.show_vault(&vault, window, cx);
      }
    })];
    self.selected = Some(vault.clone());
    self.show_vault(&vault, window, cx);
  }

  pub fn open_first(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    match self.library.read(cx).vaults.first().cloned() {
      Some(vault) => self.open(vault, window, cx),
      None => {
        self.selected = None;
        self.screen_subscriptions.clear();
        self.screen = Screen::Empty;
        cx.notify();
      }
    }
  }

  pub fn show_vault(&mut self, vault: &Entity<Vault>, window: &mut Window, cx: &mut Context<Self>) {
    self.screen = match vault.read(cx).status {
      Status::Unlocked => {
        Screen::Browse(cx.new(|cx| EntryBrowser::new(vault.clone(), window, cx)))
      }
      Status::Locked | Status::Unlocking => {
        Screen::Unlock(cx.new(|cx| UnlockForm::new(vault.clone(), window, cx)))
      }
    };
    cx.notify();
  }

  /// Settings of `vault`, or of a new database when `None`.
  pub fn settings(
    &mut self,
    vault: Option<Entity<Vault>>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if vault.is_none() {
      self.selected = None;
      self.screen_subscriptions.clear();
    }
    let config = vault.map(|vault| vault.read(cx).config.clone());
    let form = cx.new(|cx| DatabaseSettings::new(config, window, cx));
    self.screen_subscriptions.push(cx.subscribe_in(&form, window, |this, _, event, window, cx| {
      this.on_settings(event, window, cx)
    }));
    self.screen = Screen::Settings(form);
    cx.notify();
  }

  pub fn on_settings(&mut self, event: &SettingsEvent, window: &mut Window, cx: &mut Context<Self>) {
    match (event, self.selected.clone()) {
      (SettingsEvent::Save(config), Some(vault)) => {
        self.library.update(cx, |library, cx| library.update(&vault, config.clone(), cx));
        self.show_vault(&vault, window, cx);
      }
      (SettingsEvent::Save(config), None) => {
        let vault = self.library.update(cx, |library, cx| library.add(config.clone(), cx));
        self.open(vault, window, cx);
      }
      (SettingsEvent::Cancel, Some(vault)) => self.show_vault(&vault, window, cx),
      (SettingsEvent::Cancel, None) => self.open_first(window, cx),
      (SettingsEvent::Remove, vault) => {
        let vault = vault.expect("removing a database that was never saved");
        self.library.update(cx, |library, cx| library.remove(&vault, cx));
        self.open_first(window, cx);
      }
    }
  }

  /// Button naming the current database, opening a menu to switch or add databases.
  pub fn render_switcher(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let name: SharedString = match &self.selected {
      Some(vault) => vault.read(cx).config.name.clone().into(),
      None => "New database".into(),
    };
    let (library, selected, app) = (self.library.clone(), self.selected.clone(), cx.entity());
    Button::new("switch-database")
      .ghost()
      .icon(IconName::Database)
      .label(name)
      .child(Icon::new(IconName::ChevronsUpDown).xsmall().text_color(cx.theme().muted_foreground))
      .dropdown_menu(move |menu, _, cx| {
        let vaults = library.read(cx).vaults.clone();
        let menu = vaults.into_iter().fold(menu.min_w(px(220.)), |menu, vault| {
          let app = app.clone();
          let checked = selected.as_ref() == Some(&vault);
          menu.item(
            PopupMenuItem::new(vault.read(cx).config.name.clone()).checked(checked).on_click(
              move |_, window, cx| app.update(cx, |app, cx| app.open(vault.clone(), window, cx)),
            ),
          )
        });
        let app = app.clone();
        menu.separator().item(
          PopupMenuItem::new("Add database…")
            .icon(IconName::Plus)
            .on_click(move |_, window, cx| app.update(cx, |app, cx| app.settings(None, window, cx))),
        )
      })
  }

  pub fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let vault = self.selected.clone().filter(|_| !matches!(self.screen, Screen::Settings(_)));
    let state = vault.as_ref().map(|vault| vault.read(cx));
    let unlocked = state.is_some_and(|state| state.status == Status::Unlocked);
    // Environment databases are configured outside the app.
    let configurable = state.is_some_and(|state| !state.from_env);
    let (lock_vault, settings_vault) = (vault.clone(), vault);
    TitleBar::new()
      .pr_2()
      .child(self.render_switcher(cx))
      .child(
        h_flex()
          .gap_1()
          .when(configurable, |buttons| {
            buttons.child(
              Button::new("settings").ghost().icon(IconName::Settings).tooltip("Database settings").on_click(
                cx.listener(move |this, _, window, cx| {
                  this.settings(settings_vault.clone(), window, cx)
                }),
              ),
            )
          })
          .when(unlocked, |buttons| {
            buttons.child(Button::new("lock").ghost().icon(IconName::Lock).label("Lock").on_click(
              cx.listener(move |_, _, _, cx| {
                let vault = lock_vault.as_ref().expect("lock without a database");
                vault.update(cx, |vault, cx| vault.lock(cx))
              }),
            ))
          }),
      )
  }

  pub fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
    v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap_2()
      .child(div().text_color(cx.theme().muted_foreground).child("No database configured"))
      .child(
        Button::new("add-first-database")
          .primary()
          .label("Add database")
          .on_click(cx.listener(|this, _, window, cx| this.settings(None, window, cx))),
      )
      .into_any_element()
  }
}

impl Render for KeyApp {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let has_databases = !self.library.read(cx).vaults.is_empty();
    let header = match has_databases {
      true => self.render_header(cx).into_any_element(),
      // Still draws the window controls and a drag area.
      false => TitleBar::new().into_any_element(),
    };
    let body = match &self.screen {
      Screen::Empty => self.render_empty(cx),
      Screen::Unlock(view) => view.clone().into_any_element(),
      Screen::Browse(view) => view.clone().into_any_element(),
      Screen::Settings(view) => view.clone().into_any_element(),
    };
    v_flex()
      .size_full()
      .bg(cx.theme().background)
      .text_color(cx.theme().foreground)
      .child(header)
      .child(div().flex_1().min_h_0().child(body))
  }
}
