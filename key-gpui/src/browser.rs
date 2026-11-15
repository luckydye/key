use std::time::Duration;

use gpui::{
  div, prelude::*, px, App, Context, Entity, FontWeight, ScrollHandle, SharedString,
  Subscription, Task, Window,
};
use gpui_component::{
  clipboard::Clipboard,
  h_flex,
  input::{Input, InputEvent, InputState, MoveDown, MoveUp, Textarea, TextareaState},
  progress::ProgressCircle,
  tag::Tag,
  v_flex, ActiveTheme, Sizable,
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
  /// Current code of the selected entry with its validity, `None` without a valid otp secret.
  pub otp_code: Option<OtpCode>,
  pub notes: Entity<TextareaState>,
  /// One input per custom field of the selected entry, in the same order.
  pub custom_fields: Vec<Entity<InputState>>,
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
      otp_code: None,
      notes,
      custom_fields: Vec::new(),
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
    self.custom_fields = entry
      .map(|entry| entry.custom_fields.clone())
      .unwrap_or_default()
      .into_iter()
      .map(|field| {
        let input = cx.new(|cx| InputState::new(window, cx).masked(field.protected));
        input.update(cx, |input, cx| input.set_value(field.value, window, cx));
        input
      })
      .collect();
    self.refresh_otp(window, cx);
    self.list_scroll.scroll_to_item(self.selected);
    cx.notify();
  }

  pub fn refresh_otp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let secret = self.results(cx).get(self.selected).and_then(|e| e.otp_secret.clone());
    let Some(secret) = secret else {
      return;
    };
    let code = key::totp(secret.to_string(), None, None).and_then(|totp| {
      Ok(OtpCode { code: totp.generate_current()?.into(), ttl: totp.ttl()?, period: totp.step })
    });
    let (text, code): (SharedString, _) = match code {
      Ok(code) => (code.code.clone(), Some(code)),
      Err(err) => (format!("{err:#}").into(), None),
    };
    if self.otp_code != code {
      self.otp_code = code;
      cx.notify();
    }
    if self.otp.read(cx).value() != text {
      self.otp.update(cx, |input, cx| input.set_value(text, window, cx));
    }
  }
}

#[derive(PartialEq)]
pub struct OtpCode {
  pub code: SharedString,
  /// Seconds until the code rotates.
  pub ttl: u64,
  /// Seconds a code is valid.
  pub period: u64,
}

impl Render for EntryBrowser {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let results = self.results(cx);
    let theme = cx.theme();
    let rows = results.iter().enumerate().map(|(index, entry)| {
      let previous_group = index.checked_sub(1).map(|previous| &results[previous].group);
      // The header lives inside the group's first row, keeping one list child per entry for scrolling.
      let header = entry.group.clone().filter(|_| previous_group != Some(&entry.group));
      v_flex()
        .children(header.map(|group| {
          div()
            .px_3()
            .pt_3()
            .pb_1()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.muted_foreground)
            .child(group)
        }))
        .child(
          div()
            .id(index)
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .when(index == self.selected, |row| row.bg(theme.list_active))
            .hover(|row| row.bg(theme.list_hover))
            .child(entry.name.clone())
            .child(div().text_xs().text_color(theme.muted_foreground).child(entry.user.clone()))
            .on_click(cx.listener(move |this, _, window, cx| this.select(index, window, cx))),
        )
    });
    let selected = results.get(self.selected);
    let title = selected.map(|entry| entry.title.clone());
    let has_otp = selected.is_some_and(|entry| entry.otp_secret.is_some());
    let tags = selected.map(|entry| entry.tags.clone()).unwrap_or_default();
    let custom_fields = selected
      .map(|entry| entry.custom_fields.iter().zip(&self.custom_fields))
      .into_iter()
      .flatten()
      .map(|(field, input)| {
        v_flex()
          .gap_2()
          .child(label(field.name.clone(), cx))
          .child(Input::new(input).readonly(true).when(field.protected, Input::mask_toggle))
      });
    let times = selected.map(|entry| {
      [("Created", &entry.created), ("Modified", &entry.modified), ("Expires", &entry.expires)]
        .into_iter()
        .filter_map(|(name, time)| time.as_ref().map(|time| format!("{name} {time}")))
        .collect::<Vec<_>>()
        .join("  ·  ")
    });
    let otp_suffix = self.otp_code.as_ref().map(|otp| {
      let color = if otp.ttl <= 5 { theme.danger } else { theme.progress_bar };
      h_flex()
        .gap_2()
        .child(
          div().text_xs().text_color(theme.muted_foreground).child(format!("{}s", otp.ttl)),
        )
        .child(
          ProgressCircle::new("otp-ttl")
            .small()
            .color(color)
            .value(otp.ttl as f32 / otp.period as f32 * 100.),
        )
        .child(Clipboard::new("otp-copy").value(otp.code.clone()).tooltip("Copy code"))
    });

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
          .id("details")
          .overflow_y_scroll()
          .p_4()
          .gap_2()
          .children(title.map(|title| div().text_lg().child(title)))
          .when(!tags.is_empty(), |details| {
            details.child(
              h_flex().flex_wrap().gap_1().children(tags.into_iter().map(|t| Tag::new().child(t))),
            )
          })
          .child(label("User", cx))
          .child(Input::new(&self.user).readonly(true))
          .child(label("Password", cx))
          .child(Input::new(&self.secret).mask_toggle().readonly(true))
          .when(has_otp, |details| {
            details
              .child(label("OTP", cx))
              .child(Input::new(&self.otp).readonly(true).when_some(otp_suffix, Input::suffix))
          })
          .child(label("Website", cx))
          .child(Input::new(&self.website).readonly(true))
          .child(label("Notes", cx))
          .child(Textarea::new(&self.notes).readonly(true))
          .children(custom_fields)
          .children(
            times.map(|times| div().pt_2().text_xs().text_color(theme.muted_foreground).child(times)),
          ),
      )
  }
}

fn label(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
  div().text_sm().text_color(cx.theme().muted_foreground).child(text.into())
}
