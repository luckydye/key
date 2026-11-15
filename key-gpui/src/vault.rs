use gpui::{Context, EventEmitter, SharedString, Task};
use key::{
  db::{get_database, get_database_key, KeeOptions},
  fields, Database, Entry, GroupRef,
};

use crate::config::DatabaseConfig;

#[derive(Clone)]
pub struct EntryRow {
  pub title: SharedString,
  /// Title part before the last "/", e.g. "work/aws" for "work/aws/prod".
  pub group: Option<SharedString>,
  /// Title part after the last "/", the whole title without a group.
  pub name: SharedString,
  pub user: SharedString,
  pub password: SharedString,
  pub website: SharedString,
  pub notes: SharedString,
  pub otp_secret: Option<SharedString>,
  /// Fields besides the standard ones above, sorted by name.
  pub custom_fields: Vec<CustomField>,
  pub tags: Vec<SharedString>,
  pub created: Option<SharedString>,
  pub modified: Option<SharedString>,
  /// Set only when the entry is marked to expire.
  pub expires: Option<SharedString>,
}

#[derive(Clone)]
pub struct CustomField {
  pub name: SharedString,
  pub value: SharedString,
  pub protected: bool,
}

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M";

const STANDARD_FIELDS: [&str; 6] =
  [fields::TITLE, fields::USERNAME, fields::PASSWORD, fields::URL, fields::NOTES, fields::OTP];

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
            vault.entries = entry_rows(&db.root());
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
    let mut results = self
      .entries
      .iter()
      .filter(|e| {
        e.title.to_lowercase().contains(&query) || e.user.to_lowercase().contains(&query)
      })
      .cloned()
      .collect::<Vec<_>>();
    // Stable, so entries keep their database order within a group.
    results.sort_by(|a, b| a.group.cmp(&b.group));
    results
  }
}

fn open_database(options: &KeeOptions) -> anyhow::Result<Database> {
  let key = get_database_key(options)?;
  let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
  runtime.block_on(get_database(options, &key))
}

/// Entries of `group` and all its subgroups, in database order.
fn entry_rows(group: &GroupRef) -> Vec<EntryRow> {
  let entries = group.entries().map(|entry| {
    let field = |name: &str| SharedString::from(entry.get(name).unwrap_or("").to_string());
    let title = entry.get_title().expect("entry without title");
    let (group, name) = split_title(title);
    let times = &entry.times;
    EntryRow {
      title: title.to_string().into(),
      group,
      name,
      user: field(fields::USERNAME),
      password: field(fields::PASSWORD),
      website: field(fields::URL),
      notes: field(fields::NOTES),
      otp_secret: entry.get(fields::OTP).map(|secret| secret.to_string().into()),
      custom_fields: custom_fields(&entry),
      tags: entry.tags.iter().map(|tag| tag.clone().into()).collect(),
      created: times.creation.map(|time| time.format(TIME_FORMAT).to_string().into()),
      modified: times.last_modification.map(|time| time.format(TIME_FORMAT).to_string().into()),
      expires: (times.expires == Some(true))
        .then(|| times.expiry.format(TIME_FORMAT).to_string().into()),
    }
  });
  entries.chain(group.groups().flat_map(|group| entry_rows(&group))).collect()
}

fn custom_fields(entry: &Entry) -> Vec<CustomField> {
  let mut fields: Vec<CustomField> = entry
    .fields
    .iter()
    .filter(|(name, _)| !STANDARD_FIELDS.contains(&name.as_str()))
    .map(|(name, value)| CustomField {
      name: name.clone().into(),
      value: value.get().clone().into(),
      protected: value.is_protected(),
    })
    .collect();
  fields.sort_by(|a, b| a.name.cmp(&b.name));
  fields
}

/// Splits "group/name" at the last "/". Titles without both parts have no group.
fn split_title(title: &str) -> (Option<SharedString>, SharedString) {
  match title.rsplit_once('/').map(|(group, name)| (group.trim(), name.trim())) {
    Some((group, name)) if !group.is_empty() && !name.is_empty() => {
      (Some(group.to_string().into()), name.to_string().into())
    }
    _ => (None, title.to_string().into()),
  }
}
