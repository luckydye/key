use gpui::{div, prelude::*, px, AnyElement, Context, Entity, Subscription, Window};
use gpui_component::{
  button::{Button, ButtonVariants},
  h_flex, v_flex, ActiveTheme,
};

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

/// Database list on the left, the selected database on the right.
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

  pub fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme();
    let rows = self.library.read(cx).vaults.iter().enumerate().map(|(index, vault)| {
      let state = vault.read(cx);
      let status = match state.status {
        Status::Locked => "Locked",
        Status::Unlocking => "Unlocking…",
        Status::Unlocked => "Unlocked",
      };
      let status = if state.from_env { format!("{status} · from env") } else { status.into() };
      let vault = vault.clone();
      div()
        .id(index)
        .px_3()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .when(self.selected.as_ref() == Some(&vault), |row| row.bg(theme.list_active))
        .hover(|row| row.bg(theme.list_hover))
        .child(state.config.name.clone())
        .child(div().text_xs().text_color(theme.muted_foreground).child(status))
        .on_click(cx.listener(move |this, _, window, cx| this.open(vault.clone(), window, cx)))
    });

    v_flex()
      .w(px(200.))
      .h_full()
      .p_2()
      .gap_1()
      .border_r_1()
      .border_color(theme.border)
      .child(div().px_3().py_1().text_xs().text_color(theme.muted_foreground).child("Databases"))
      .children(rows)
      .child(div().flex_1())
      .child(
        Button::new("add-database")
          .ghost()
          .label("Add database")
          .on_click(cx.listener(|this, _, window, cx| this.settings(None, window, cx))),
      )
  }

  pub fn render_header(&self, vault: &Entity<Vault>, cx: &mut Context<Self>) -> impl IntoElement {
    let state = vault.read(cx);
    let unlocked = state.status == Status::Unlocked;
    let from_env = state.from_env;
    let lock_vault = vault.clone();
    let settings_vault = vault.clone();
    h_flex()
      .justify_between()
      .px_4()
      .py_2()
      .border_b_1()
      .border_color(cx.theme().border)
      .child(div().text_lg().child(state.config.name.clone()))
      .child(
        h_flex()
          .gap_2()
          .when(unlocked, |buttons| {
            buttons.child(Button::new("lock").ghost().label("Lock").on_click(cx.listener(
              move |_, _, _, cx| lock_vault.update(cx, |vault, cx| vault.lock(cx)),
            )))
          })
          // Environment databases are configured outside the app.
          .when(!from_env, |buttons| {
            buttons.child(Button::new("settings").ghost().label("Settings").on_click(
              cx.listener(move |this, _, window, cx| {
                this.settings(Some(settings_vault.clone()), window, cx)
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
    let header = match (&self.screen, &self.selected) {
      (Screen::Unlock(_) | Screen::Browse(_), Some(vault)) => {
        Some(self.render_header(vault, cx))
      }
      _ => None,
    };
    let body = match &self.screen {
      Screen::Empty => self.render_empty(cx),
      Screen::Unlock(view) => view.clone().into_any_element(),
      Screen::Browse(view) => view.clone().into_any_element(),
      Screen::Settings(view) => view.clone().into_any_element(),
    };
    h_flex()
      .size_full()
      .bg(cx.theme().background)
      .text_color(cx.theme().foreground)
      .child(self.render_sidebar(cx))
      .child(v_flex().flex_1().h_full().children(header).child(div().flex_1().min_h_0().child(body)))
  }
}
