use gpui::{
  div, prelude::*, px, AnyElement, Context, Entity, EventEmitter, FontWeight, PathPromptOptions,
  PromptLevel, SharedString, Subscription, Window,
};
use gpui_component::{
  button::{Button, ButtonVariants},
  h_flex,
  input::{Input, InputEvent, InputState},
  v_flex, ActiveTheme,
};

use crate::config::DatabaseConfig;

pub enum SettingsEvent {
  Save(DatabaseConfig),
  Cancel,
  Remove,
}

/// Connection settings of one database, or of a new one.
pub struct DatabaseSettings {
  pub is_new: bool,
  pub name: Entity<InputState>,
  pub url: Entity<InputState>,
  pub keyfile: Entity<InputState>,
  pub s3_access_key: Entity<InputState>,
  pub s3_secret_key: Entity<InputState>,
  pub error: Option<SharedString>,
  pub _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SettingsEvent> for DatabaseSettings {}

impl DatabaseSettings {
  pub fn new(
    config: Option<DatabaseConfig>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    let is_new = config.is_none();
    let config = config.unwrap_or_default();
    let mut input = |value: Option<String>, placeholder: &'static str| {
      cx.new(|cx| {
        InputState::new(window, cx)
          .placeholder(placeholder)
          .default_value(value.unwrap_or_default())
      })
    };
    let name = input(Some(config.name), "Personal");
    let url = input(Some(config.url), "file:///path/to/db.kdbx or s3://host/bucket/db.kdbx");
    let keyfile = input(config.keyfile, "/path/to/keyfile");
    let s3_access_key = input(config.s3_access_key, "Access key");
    let s3_secret_key = input(config.s3_secret_key, "Secret key");
    s3_secret_key.update(cx, |input, cx| input.set_masked(true, window, cx));
    name.update(cx, |input, cx| input.focus(window, cx));
    let subscriptions = [&name, &url, &keyfile, &s3_access_key, &s3_secret_key]
      .into_iter()
      .map(|input| {
        cx.subscribe(input, |this, _, event, cx| {
          if matches!(event, InputEvent::PressEnter { .. }) {
            this.save(cx);
          }
        })
      })
      .collect();
    Self {
      is_new,
      name,
      url,
      keyfile,
      s3_access_key,
      s3_secret_key,
      error: None,
      _subscriptions: subscriptions,
    }
  }

  pub fn config(&self, cx: &Context<Self>) -> Result<DatabaseConfig, &'static str> {
    let value = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
    let optional = |input: &Entity<InputState>| Some(value(input)).filter(|v| !v.is_empty());
    let config = DatabaseConfig {
      name: value(&self.name),
      url: value(&self.url),
      keyfile: optional(&self.keyfile),
      s3_access_key: optional(&self.s3_access_key),
      s3_secret_key: optional(&self.s3_secret_key),
    };
    if config.name.is_empty() {
      return Err("Name is required");
    }
    match config.url.split_once("://") {
      Some(("file", path)) if !path.is_empty() => Ok(config),
      // `key` expects s3://host/bucket/object.
      Some(("s3", rest)) if rest.split('/').filter(|p| !p.is_empty()).count() >= 3 => {
        Ok(config)
      }
      _ => Err("Database URL must look like file:///path/db.kdbx or s3://host/bucket/db.kdbx"),
    }
  }

  pub fn save(&mut self, cx: &mut Context<Self>) {
    match self.config(cx) {
      Ok(config) => cx.emit(SettingsEvent::Save(config)),
      Err(error) => {
        self.error = Some(error.into());
        cx.notify();
      }
    }
  }

  pub fn remove(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let name = self.name.read(cx).value();
    let answer = window.prompt(
      PromptLevel::Warning,
      &format!("Remove “{name}”?"),
      Some("Only the connection settings are removed, the database file is kept."),
      &["Remove", "Cancel"],
      cx,
    );
    cx.spawn(async move |this, cx| {
      if answer.await == Ok(0) {
        let _ = this.update(cx, |_, cx| cx.emit(SettingsEvent::Remove));
      }
    })
    .detach();
  }

  /// Fills `target` with a file picked in the native file dialog.
  pub fn browse(
    &mut self,
    target: Entity<InputState>,
    prefix: &'static str,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
      files: true,
      directories: false,
      multiple: false,
      prompt: None,
    });
    cx.spawn_in(window, async move |_, cx| {
      let picked = paths.await.expect("file dialog dropped").expect("file dialog failed");
      let Some(path) = picked.and_then(|paths| paths.into_iter().next()) else {
        return;
      };
      let value = format!("{prefix}{}", path.display());
      let _ = cx.update(|window, cx| {
        target.update(cx, |input, cx| input.set_value(value, window, cx))
      });
    })
    .detach();
  }

  pub fn browse_button(
    &self,
    id: &'static str,
    target: &Entity<InputState>,
    prefix: &'static str,
    cx: &mut Context<Self>,
  ) -> Button {
    let target = target.clone();
    Button::new(id).label("Browse…").on_click(cx.listener(move |this, _, window, cx| {
      this.browse(target.clone(), prefix, window, cx)
    }))
  }
}

impl Render for DatabaseSettings {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let url_browse = self.browse_button("browse-url", &self.url, "file://", cx);
    let keyfile_browse = self.browse_button("browse-keyfile", &self.keyfile, "", cx);
    let (muted, danger) = (cx.theme().muted_foreground, cx.theme().danger);
    let field = |label: &'static str, input: AnyElement| {
      v_flex().gap_1().child(div().text_sm().text_color(muted).child(label)).child(input)
    };

    v_flex().id("settings").size_full().overflow_y_scroll().items_center().child(
      v_flex()
      .w_full()
      .p_6()
      .gap_4()
      .max_w(px(560.))
      .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(if self.is_new { "New database" } else { "Database settings" }))
      .child(field("Name", Input::new(&self.name).into_any_element()))
      .child(field(
        "Database URL",
        h_flex().gap_2().child(Input::new(&self.url)).child(url_browse).into_any_element(),
      ))
      .child(field(
        "Keyfile (optional)",
        h_flex().gap_2().child(Input::new(&self.keyfile)).child(keyfile_browse).into_any_element(),
      ))
      .child(field("S3 access key (optional)", Input::new(&self.s3_access_key).into_any_element()))
      .child(field(
        "S3 secret key (optional)",
        Input::new(&self.s3_secret_key).mask_toggle().into_any_element(),
      ))
      .children(
        self.error.clone().map(|error| div().text_sm().text_color(danger).child(error)),
      )
      .child(
        h_flex()
          .gap_2()
          .child(
            Button::new("save")
              .primary()
              .label("Save")
              .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
          )
          .child(
            Button::new("cancel")
              .ghost()
              .label("Cancel")
              .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Cancel))),
          )
          .child(div().flex_1())
          .when(!self.is_new, |buttons| {
            buttons.child(
              Button::new("remove")
                .danger()
                .label("Remove")
                .on_click(cx.listener(|this, _, window, cx| this.remove(window, cx))),
            )
          }),
      ),
    )
  }
}
