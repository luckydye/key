use gpui::{div, prelude::*, px, Context, Entity, FontWeight, Subscription, Window};
use gpui_component::{
  button::{Button, ButtonVariants},
  input::{Input, InputEvent, InputState},
  v_flex, ActiveTheme, Icon, Sizable,
};
use gpui_kit_assets::IconName;

use crate::vault::{Status, Vault};

/// Password prompt of a locked vault.
pub struct UnlockForm {
  pub vault: Entity<Vault>,
  pub password: Entity<InputState>,
  pub _subscriptions: Vec<Subscription>,
}

impl UnlockForm {
  pub fn new(vault: Entity<Vault>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let password =
      cx.new(|cx| InputState::new(window, cx).masked(true).placeholder("Password"));
    password.update(cx, |input, cx| input.focus(window, cx));
    let subscriptions = vec![
      cx.observe(&vault, |_, _, cx| cx.notify()),
      cx.subscribe(&password, |this, _, event, cx| {
        if matches!(event, InputEvent::PressEnter { .. }) {
          this.unlock(cx);
        }
      }),
    ];
    Self { vault, password, _subscriptions: subscriptions }
  }

  pub fn unlock(&mut self, cx: &mut Context<Self>) {
    if self.vault.read(cx).status != Status::Locked {
      return;
    }
    let password = self.password.read(cx).value().to_string();
    self.vault.update(cx, |vault, cx| vault.unlock(password, cx));
  }
}

impl Render for UnlockForm {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let vault = self.vault.read(cx);
    let unlocking = vault.status == Status::Unlocking;
    let theme = cx.theme();
    v_flex().size_full().items_center().justify_center().child(
      v_flex()
        .w(px(340.))
        .gap_3()
        .items_center()
        .child(
          div()
            .p_3()
            .rounded_full()
            .bg(theme.secondary)
            .child(Icon::new(IconName::Lock).large().text_color(theme.muted_foreground)),
        )
        .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(vault.config.name.clone()))
        .child(
          div()
            .w_full()
            .text_sm()
            .text_center()
            .text_ellipsis()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_color(theme.muted_foreground)
            .child(vault.config.url.clone()),
        )
        .child(
          v_flex()
            .w_full()
            .pt_2()
            .gap_2()
            .child(Input::new(&self.password).mask_toggle().disabled(unlocking))
            .child(
              Button::new("unlock")
                .primary()
                .w_full()
                .label("Unlock")
                .loading(unlocking)
                .on_click(cx.listener(|this, _, _, cx| this.unlock(cx))),
            ),
        )
        .children(
          vault
            .error
            .clone()
            .map(|error| div().w_full().text_sm().text_color(theme.danger).child(error)),
        ),
    )
  }
}
