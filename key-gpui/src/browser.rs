use std::time::Duration;

use gpui::{
  div, prelude::*, px, App, ClipboardItem, Context, Entity, FontWeight, ScrollHandle,
  SharedString, Subscription, Task, Window,
};
use gpui_component::{
  button::{Button, ButtonVariants},
  clipboard::Clipboard,
  h_flex,
  input::{Input, InputEvent, InputState, MoveDown, MoveUp, Textarea, TextareaState},
  progress::ProgressCircle,
  tag::Tag,
  v_flex, ActiveTheme, Icon, Sizable, WindowExt,
};
use gpui_kit_assets::IconName;

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
    let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
    let user = cx.new(|cx| InputState::new(window, cx));
    let secret = cx.new(|cx| InputState::new(window, cx).masked(true));
    let website = cx.new(|cx| InputState::new(window, cx));
    let otp = cx.new(|cx| InputState::new(window, cx));
    let notes = cx.new(|cx| TextareaState::new(window, cx).rows(6));
    query.update(cx, |input, cx| input.focus(window, cx));

    let subscriptions = vec![cx.subscribe_in(&query, window, |this, _, event, window, cx| {
      match event {
        InputEvent::Change => this.select(0, window, cx),
        InputEvent::PressEnter { .. } => this.copy_password(window, cx),
        _ => {}
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

  pub fn copy_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(entry) = self.results(cx).into_iter().nth(self.selected) else {
      return;
    };
    cx.write_to_clipboard(ClipboardItem::new_string(entry.password.to_string()));
    window.push_notification(format!("Copied password of {}", entry.name), cx);
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
      let active = index == self.selected;
      v_flex()
        .children(header.map(|group| {
          h_flex()
            .gap_1p5()
            .px_2()
            .pt_3()
            .pb_1()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.muted_foreground)
            .child(Icon::new(IconName::Folder).xsmall())
            .child(truncated(group))
        }))
        .child(
          v_flex()
            .id(index)
            .px_2()
            .py_1p5()
            .rounded_md()
            .cursor_pointer()
            .when(active, |row| row.bg(theme.list_active))
            .when(!active, |row| row.hover(|row| row.bg(theme.list_hover)))
            .child(truncated(entry.name.clone()).text_sm().font_weight(FontWeight::MEDIUM))
            .when(!entry.user.is_empty(), |row| {
              row.child(truncated(entry.user.clone()).text_xs().text_color(theme.muted_foreground))
            })
            .on_click(cx.listener(move |this, _, window, cx| this.select(index, window, cx))),
        )
    });
    let empty_list = results.is_empty().then(|| {
      div().p_4().text_sm().text_center().text_color(theme.muted_foreground).child("No entries")
    });

    let list = v_flex()
      .w(px(280.))
      .h_full()
      .p_2()
      .gap_2()
      .border_r_1()
      .border_color(theme.border)
      .bg(theme.sidebar)
      .capture_action(cx.listener(|this, _: &MoveUp, window, cx| {
        this.select(this.selected.saturating_sub(1), window, cx);
        cx.stop_propagation();
      }))
      .capture_action(cx.listener(|this, _: &MoveDown, window, cx| {
        this.select(this.selected + 1, window, cx);
        cx.stop_propagation();
      }))
      .child(
        Input::new(&self.query)
          .cleanable(true)
          .prefix(Icon::new(IconName::Search).small().text_color(theme.muted_foreground)),
      )
      .child(
        div()
          .id("entries")
          .flex_1()
          .overflow_y_scroll()
          .track_scroll(&self.list_scroll)
          .children(rows)
          .children(empty_list),
      )
      .child(
        div()
          .px_2()
          .text_xs()
          .text_color(theme.muted_foreground)
          .child(format!("{} entries  ·  ↵ copies password", results.len())),
      );

    let details = results.get(self.selected).map(|entry| self.render_details(entry, cx));
    h_flex().size_full().child(list).child(
      div().id("details").flex_1().h_full().overflow_y_scroll().children(details),
    )
  }
}

impl EntryBrowser {
  pub fn render_details(&self, entry: &EntryRow, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme();
    let copy = |id: &'static str, value: &SharedString| {
      Clipboard::new(id).value(value.clone()).tooltip("Copy")
    };
    let custom_fields = entry.custom_fields.iter().zip(&self.custom_fields).enumerate().map(
      |(index, (field, input))| {
        field_row(
          field.name.clone(),
          Input::new(input)
            .readonly(true)
            .when(field.protected, Input::mask_toggle)
            .suffix(Clipboard::new(("copy-field", index)).value(field.value.clone()).tooltip("Copy")),
          cx,
        )
      },
    );
    let times = [("Created", &entry.created), ("Modified", &entry.modified), ("Expires", &entry.expires)]
      .into_iter()
      .filter_map(|(name, time)| time.as_ref().map(|time| format!("{name} {time}")))
      .collect::<Vec<_>>()
      .join("  ·  ");
    let otp_suffix = self.otp_code.as_ref().map(|otp| {
      let color = if otp.ttl <= 5 { theme.danger } else { theme.progress_bar };
      h_flex()
        .gap_2()
        .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{}s", otp.ttl)))
        .child(
          ProgressCircle::new("otp-ttl")
            .small()
            .color(color)
            .value(otp.ttl as f32 / otp.period as f32 * 100.),
        )
        .child(Clipboard::new("otp-copy").value(otp.code.clone()).tooltip("Copy code"))
    });
    let website = entry.website.clone();

    v_flex()
      .w_full()
      .max_w(px(680.))
      .p_6()
      .gap_4()
      .child(
        v_flex()
          .gap_1()
          .children(entry.group.clone().map(|group| {
            div().text_xs().text_color(theme.muted_foreground).child(group)
          }))
          .child(div().text_2xl().font_weight(FontWeight::SEMIBOLD).child(entry.name.clone()))
          .when(!entry.tags.is_empty(), |header| {
            header.child(
              h_flex()
                .pt_1()
                .flex_wrap()
                .gap_1()
                .children(entry.tags.iter().map(|tag| Tag::secondary().small().child(tag.clone()))),
            )
          }),
      )
      .when(!entry.user.is_empty(), |details| {
        details.child(field_row(
          "Username",
          Input::new(&self.user).readonly(true).suffix(copy("copy-user", &entry.user)),
          cx,
        ))
      })
      .when(!entry.password.is_empty(), |details| {
        details.child(field_row(
          "Password",
          Input::new(&self.secret)
            .mask_toggle()
            .readonly(true)
            .suffix(copy("copy-password", &entry.password)),
          cx,
        ))
      })
      .when(entry.otp_secret.is_some(), |details| {
        details.child(field_row(
          "One-time code",
          Input::new(&self.otp).readonly(true).when_some(otp_suffix, Input::suffix),
          cx,
        ))
      })
      .when(!entry.website.is_empty(), |details| {
        details.child(field_row(
          "Website",
          Input::new(&self.website).readonly(true).suffix(
            h_flex()
              .child(
                Button::new("open-website")
                  .ghost()
                  .xsmall()
                  .icon(IconName::ExternalLink)
                  .tooltip("Open in browser")
                  .on_click(move |_, _, cx| cx.open_url(&website)),
              )
              .child(copy("copy-website", &entry.website)),
          ),
          cx,
        ))
      })
      .children(custom_fields)
      .when(!entry.notes.is_empty(), |details| {
        details.child(field_row("Notes", Textarea::new(&self.notes).readonly(true), cx))
      })
      .when(!times.is_empty(), |details| {
        details.child(
          div()
            .pt_3()
            .border_t_1()
            .border_color(theme.border)
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(times),
        )
      })
  }
}

fn field_row(label: impl Into<SharedString>, input: impl IntoElement, cx: &App) -> impl IntoElement {
  v_flex()
    .gap_1p5()
    .child(
      div()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(cx.theme().muted_foreground)
        .child(label.into()),
    )
    .child(input)
}

/// Single line that ends in an ellipsis when it does not fit.
fn truncated(text: SharedString) -> gpui::Div {
  div().w_full().overflow_hidden().whitespace_nowrap().text_ellipsis().child(text)
}
