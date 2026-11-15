use gpui::{div, prelude::*, px, Context, Entity, Subscription, Window};
use gpui_component::{
  button::{Button, ButtonVariants},
  input::{Input, InputEvent, InputState},
  v_flex, ActiveTheme,
};

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
    v_flex().size_full().items_center().justify_center().child(
      v_flex()
        .w(px(320.))
        .gap_2()
        .child(Input::new(&self.password).mask_toggle().disabled(unlocking))
        .child(
          Button::new("unlock")
            .primary()
            .label("Unlock")
            .loading(unlocking)
            .on_click(cx.listener(|this, _, _, cx| this.unlock(cx))),
        )
        .children(
          vault
            .error
            .clone()
            .map(|error| div().text_sm().text_color(cx.theme().danger).child(error)),
        ),
    )
  }
}
