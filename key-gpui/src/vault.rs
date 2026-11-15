use gpui::{Context, EventEmitter, SharedString, Task};
use key::{
  db::{get_database, get_database_key, KeeOptions},
  Database, Node,
};

use crate::config::DatabaseConfig;

#[derive(Clone)]
pub struct EntryRow {
  pub title: SharedString,
  pub user: SharedString,
  pub password: SharedString,
  pub website: SharedString,
  pub notes: SharedString,
  pub otp_secret: Option<SharedString>,
}

#[derive(PartialEq)]
pub enum Status {
  Locked,
  Unlocking,
  Unlocked,
}

pub struct Unlocked;

pub struct Vault {
  pub config: DatabaseConfig,
  /// Configured through environment variables instead of the config file.
  pub from_env: bool,
  pub status: Status,
  pub error: Option<SharedString>,
  pub entries: Vec<EntryRow>,
  /// The pending unlock; dropping it cancels the unlock.
  pub unlocking: Option<Task<()>>,
}

impl EventEmitter<Unlocked> for Vault {}

impl Vault {
  pub fn new(config: DatabaseConfig, from_env: bool) -> Self {
    Self {
      config,
      from_env,
      status: Status::Locked,
      error: None,
      entries: Vec::new(),
      unlocking: None,
    }
  }

  pub fn lock(&mut self, cx: &mut Context<Self>) {
    self.unlocking = None;
    self.status = Status::Locked;
    self.error = None;
    self.entries = Vec::new();
    cx.notify();
  }

  pub fn unlock(&mut self, password: String, cx: &mut Context<Self>) {
    assert!(self.status == Status::Locked, "unlock while not locked");
    self.status = Status::Unlocking;
    self.error = None;
    cx.notify();

    let options = self.config.options(password);
    let task = cx.background_executor().spawn(async move { open_database(&options) });
    self.unlocking = Some(cx.spawn(async move |this, cx| {
      let result = task.await;
      // Fails only when the vault was dropped, i.e. the database was removed meanwhile.
      let _ = this.update(cx, |vault, cx| {
        match result {
          Ok(db) => {
            vault.entries = entry_rows(&db.root.children);
            vault.status = Status::Unlocked;
            cx.emit(Unlocked);
          }
          Err(err) => {
            vault.status = Status::Locked;
            vault.error = Some(format!("{err:#}").into());
          }
        }
        cx.notify();
      });
    }));
  }

  pub fn search(&self, query: &str) -> Vec<EntryRow> {
    let query = query.to_lowercase();
    self
      .entries
      .iter()
      .filter(|e| {
        e.title.to_lowercase().contains(&query) || e.user.to_lowercase().contains(&query)
      })
      .cloned()
      .collect()
  }
}

fn open_database(options: &KeeOptions) -> anyhow::Result<Database> {
  let key = get_database_key(options)?;
  let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
  runtime.block_on(get_database(options, &key))
}

fn entry_rows(nodes: &[Node]) -> Vec<EntryRow> {
  nodes
    .iter()
    .flat_map(|node| match node {
      Node::Group(group) => entry_rows(&group.children),
      Node::Entry(entry) => {
        let field = |name: &str| SharedString::from(entry.get(name).unwrap_or("").to_string());
        vec![EntryRow {
          title: entry.get_title().expect("entry without title").to_string().into(),
          user: field("UserName"),
          password: field("Password"),
          website: field("URL"),
          notes: field("Notes"),
          otp_secret: entry.get("otp").map(|secret| secret.to_string().into()),
        }]
      }
    })
    .collect()
}
