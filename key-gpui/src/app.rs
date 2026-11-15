use gpui::{
  div, prelude::*, px, AnyElement, Context, Entity, ScrollHandle, SharedString,
  Subscription, Window,
};
use gpui_component::{
  button::{Button, ButtonVariants},
  h_flex,
  input::{Input, InputEvent, InputState, MoveDown, MoveUp, Textarea, TextareaState},
  v_flex, ActiveTheme,
};

use crate::vault::{EntryRow, Status, Unlocked, Vault};

pub struct KeyApp {
  pub vault: Entity<Vault>,
  pub password: Entity<InputState>,
  pub query: Entity<InputState>,
  pub user: Entity<InputState>,
  pub secret: Entity<InputState>,
  pub website: Entity<InputState>,
  pub notes: Entity<TextareaState>,
  pub selected: usize,
  pub list_scroll: ScrollHandle,
  pub _subscriptions: Vec<Subscription>,
}

impl KeyApp {
  pub fn new(vault: Entity<Vault>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let password =
      cx.new(|cx| InputState::new(window, cx).masked(true).placeholder("Password"));
    let query = cx.new(|cx| InputState::new(window, cx).placeholder("Type to filter..."));
    let user = cx.new(|cx| InputState::new(window, cx));
    let secret = cx.new(|cx| InputState::new(window, cx).masked(true));
    let website = cx.new(|cx| InputState::new(window, cx));
    let notes = cx.new(|cx| TextareaState::new(window, cx).rows(6));
    password.update(cx, |input, cx| input.focus(window, cx));

    let subscriptions = vec![
      cx.observe(&vault, |_, _, cx| cx.notify()),
      cx.subscribe_in(&vault, window, |this, _, _: &Unlocked, window, cx| {
        this.query.update(cx, |input, cx| input.focus(window, cx));
        this.select(0, window, cx);
      }),
      cx.subscribe_in(&password, window, |this, _, event, window, cx| {
        if matches!(event, InputEvent::PressEnter { .. }) {
          this.unlock(window, cx);
        }
      }),
      cx.subscribe_in(&query, window, |this, _, event, window, cx| {
        if matches!(event, InputEvent::Change) {
          this.select(0, window, cx);
        }
      }),
    ];

    Self {
      vault,
      password,
      query,
      user,
      secret,
      website,
      notes,
      selected: 0,
      list_scroll: ScrollHandle::new(),
      _subscriptions: subscriptions,
    }
  }

  pub fn unlock(&mut self, _: &mut Window, cx: &mut Context<Self>) {
    if self.vault.read(cx).status != Status::Locked {
      return;
    }
    let password = self.password.read(cx).value().to_string();
    self.vault.update(cx, |vault, cx| vault.unlock(password, cx));
  }

  pub fn results(&self, cx: &Context<Self>) -> Vec<EntryRow> {
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
    self.list_scroll.scroll_to_item(self.selected);
    cx.notify();
  }

  pub fn render_locked(&self, cx: &mut Context<Self>) -> AnyElement {
    let vault = self.vault.read(cx);
    let unlocking = vault.status == Status::Unlocking;
    v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .child(
        v_flex()
          .w(px(320.))
          .gap_2()
          .child(Input::new(&self.password).mask_toggle().disabled(unlocking))
          .child(
            Button::new("unlock")
              .primary()
              .label("Unlock")
              .loading(unlocking)
              .on_click(cx.listener(|this, _, window, cx| this.unlock(window, cx))),
          )
          .children(
            vault
              .error
              .clone()
              .map(|error| div().text_sm().text_color(cx.theme().danger).child(error)),
          ),
      )
      .into_any_element()
  }

  pub fn render_unlocked(&self, cx: &mut Context<Self>) -> AnyElement {
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
    let title = results.get(self.selected).map(|entry| entry.title.clone());

    h_flex()
      .size_full()
      .child(
        v_flex()
          .w(px(280.))
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
          .child(label("Website"))
          .child(Input::new(&self.website).readonly(true))
          .child(label("Notes"))
          .child(Textarea::new(&self.notes).readonly(true)),
      )
      .into_any_element()
  }
}

impl Render for KeyApp {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let body = match self.vault.read(cx).status {
      Status::Unlocked => self.render_unlocked(cx),
      Status::Locked | Status::Unlocking => self.render_locked(cx),
    };
    div()
      .size_full()
      .bg(cx.theme().background)
      .text_color(cx.theme().foreground)
      .child(body)
  }
}
