use std::time::Duration;

use gpui::{
  div, prelude::*, px, App, Context, Entity, ScrollHandle, SharedString, Subscription, Task,
  Window,
};
use gpui_component::{
  h_flex,
  input::{Input, InputEvent, InputState, MoveDown, MoveUp, Textarea, TextareaState},
  v_flex, ActiveTheme,
};

use crate::vault::{EntryRow, Vault};

/// Filterable entry list and details of an unlocked vault.
pub struct EntryBrowser {
  pub vault: Entity<Vault>,
  pub query: Entity<InputState>,
  pub user: Entity<InputState>,
  pub secret: Entity<InputState>,
  pub website: Entity<InputState>,
  pub otp: Entity<InputState>,
  pub notes: Entity<TextareaState>,
  pub selected: usize,
  pub list_scroll: ScrollHandle,
  pub _subscriptions: Vec<Subscription>,
  pub _otp_refresh: Task<()>,
}

impl EntryBrowser {
  pub fn new(vault: Entity<Vault>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let query = cx.new(|cx| InputState::new(window, cx).placeholder("Type to filter..."));
    let user = cx.new(|cx| InputState::new(window, cx));
    let secret = cx.new(|cx| InputState::new(window, cx).masked(true));
    let website = cx.new(|cx| InputState::new(window, cx));
    let otp = cx.new(|cx| InputState::new(window, cx));
    let notes = cx.new(|cx| TextareaState::new(window, cx).rows(6));
    query.update(cx, |input, cx| input.focus(window, cx));

    let subscriptions = vec![cx.subscribe_in(&query, window, |this, _, event, window, cx| {
      if matches!(event, InputEvent::Change) {
        this.select(0, window, cx);
      }
    })];

    // Ends once the browser is released.
    let otp_refresh = cx.spawn_in(window, async move |this, cx| loop {
      cx.background_executor().timer(Duration::from_secs(1)).await;
      if this.update_in(cx, |this, window, cx| this.refresh_otp(window, cx)).is_err() {
        break;
      }
    });

    let mut browser = Self {
      vault,
      query,
      user,
      secret,
      website,
      otp,
      notes,
      selected: 0,
      list_scroll: ScrollHandle::new(),
      _subscriptions: subscriptions,
      _otp_refresh: otp_refresh,
    };
    browser.select(0, window, cx);
    browser
  }

  pub fn results(&self, cx: &App) -> Vec<EntryRow> {
    self.vault.read(cx).search(&self.query.read(cx).value())
  }

  pub fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
    let results = self.results(cx);
    self.selected = index.min(results.len().saturating_sub(1));
    let entry = results.get(self.selected);
    let value = |f: fn(&EntryRow) -> &SharedString| entry.map(f).cloned().unwrap_or_default();
    let (user, secret, website) =
      (value(|e| &e.user), value(|e| &e.password), value(|e| &e.website));
    let notes = value(|e| &e.notes);
    self.user.update(cx, |input, cx| input.set_value(user, window, cx));
    self.secret.update(cx, |input, cx| input.set_value(secret, window, cx));
    self.website.update(cx, |input, cx| input.set_value(website, window, cx));
    self.notes.update(cx, |input, cx| input.set_value(notes, window, cx));
    self.refresh_otp(window, cx);
    self.list_scroll.scroll_to_item(self.selected);
    cx.notify();
  }

  pub fn refresh_otp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let secret = self.results(cx).get(self.selected).and_then(|e| e.otp_secret.clone());
    let Some(secret) = secret else {
      return;
    };
    let code: SharedString = match key::otp(secret.to_string(), None, None) {
      Ok(code) => code.into(),
      Err(err) => format!("{err:#}").into(),
    };
    if self.otp.read(cx).value() != code {
      self.otp.update(cx, |input, cx| input.set_value(code, window, cx));
    }
  }
}

impl Render for EntryBrowser {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let results = self.results(cx);
    let theme = cx.theme();
    let rows = results.iter().enumerate().map(|(index, entry)| {
      div()
        .id(index)
        .px_3()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .when(index == self.selected, |row| row.bg(theme.list_active))
        .hover(|row| row.bg(theme.list_hover))
        .child(entry.title.clone())
        .child(div().text_xs().text_color(theme.muted_foreground).child(entry.user.clone()))
        .on_click(cx.listener(move |this, _, window, cx| this.select(index, window, cx)))
    });
    let label = |text: &'static str| div().text_sm().text_color(theme.muted_foreground).child(text);
    let selected = results.get(self.selected);
    let title = selected.map(|entry| entry.title.clone());
    let has_otp = selected.is_some_and(|entry| entry.otp_secret.is_some());

    h_flex()
      .size_full()
      .child(
        v_flex()
          .w(px(260.))
          .h_full()
          .p_2()
          .gap_2()
          .border_r_1()
          .border_color(theme.border)
          .capture_action(cx.listener(|this, _: &MoveUp, window, cx| {
            this.select(this.selected.saturating_sub(1), window, cx);
            cx.stop_propagation();
          }))
          .capture_action(cx.listener(|this, _: &MoveDown, window, cx| {
            this.select(this.selected + 1, window, cx);
            cx.stop_propagation();
          }))
          .child(Input::new(&self.query).cleanable(true))
          .child(
            div()
              .id("entries")
              .flex_1()
              .overflow_y_scroll()
              .track_scroll(&self.list_scroll)
              .children(rows),
          ),
      )
      .child(
        v_flex()
          .flex_1()
          .h_full()
          .p_4()
          .gap_2()
          .children(title.map(|title| div().text_lg().pb_2().child(title)))
          .child(label("User"))
          .child(Input::new(&self.user).readonly(true))
          .child(label("Password"))
          .child(Input::new(&self.secret).mask_toggle().readonly(true))
          .when(has_otp, |details| {
            details.child(label("OTP")).child(Input::new(&self.otp).readonly(true))
          })
          .child(label("Website"))
          .child(Input::new(&self.website).readonly(true))
          .child(label("Notes"))
          .child(Textarea::new(&self.notes).readonly(true)),
      )
  }
}
