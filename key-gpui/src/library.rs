use std::path::PathBuf;

use gpui::{AppContext, Context, Entity, Subscription};

use crate::{config, config::DatabaseConfig, vault::Vault};

/// The configured databases, persisted to the config file on every change.
pub struct Library {
  pub path: PathBuf,
  pub vaults: Vec<Entity<Vault>>,
  /// Re-notifies on any vault change, so the database list shows lock states.
  pub _observers: Vec<Subscription>,
}

impl Library {
  pub fn load(path: PathBuf, cx: &mut Context<Self>) -> Self {
    let env = config::from_env().map(|config| cx.new(|_| Vault::new(config, true)));
    let saved = config::load(&path).into_iter().map(|config| cx.new(|_| Vault::new(config, false)));
    let vaults = env.into_iter().chain(saved).collect();
    let mut library = Self { path, vaults, _observers: Vec::new() };
    library.observe_vaults(cx);
    library
  }

  pub fn observe_vaults(&mut self, cx: &mut Context<Self>) {
    self._observers =
      self.vaults.iter().map(|vault| cx.observe(vault, |_, _, cx| cx.notify())).collect();
  }

  pub fn save(&self, cx: &Context<Self>) {
    let databases: Vec<DatabaseConfig> = self
      .vaults
      .iter()
      .map(|vault| vault.read(cx))
      .filter(|vault| !vault.from_env)
      .map(|vault| vault.config.clone())
      .collect();
    config::save(&self.path, &databases);
  }

  pub fn add(&mut self, config: DatabaseConfig, cx: &mut Context<Self>) -> Entity<Vault> {
    let vault = cx.new(|_| Vault::new(config, false));
    self.vaults.push(vault.clone());
    self.observe_vaults(cx);
    self.save(cx);
    cx.notify();
    vault
  }

  /// Locks the vault, since the connection it was unlocked with may have changed.
  pub fn update(&mut self, vault: &Entity<Vault>, config: DatabaseConfig, cx: &mut Context<Self>) {
    vault.update(cx, |vault, cx| {
      vault.lock(cx);
      vault.config = config;
    });
    self.save(cx);
    cx.notify();
  }

  pub fn remove(&mut self, vault: &Entity<Vault>, cx: &mut Context<Self>) {
    self.vaults.retain(|v| v != vault);
    self.observe_vaults(cx);
    self.save(cx);
    cx.notify();
  }
}
