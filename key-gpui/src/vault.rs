use gpui::{Context, EventEmitter, SharedString};
use key::{
  db::{get_database, get_database_key, KeeOptions},
  Database, Node,
};

#[derive(Clone)]
pub struct EntryRow {
  pub title: SharedString,
  pub user: SharedString,
  pub password: SharedString,
  pub website: SharedString,
  pub notes: SharedString,
}

#[derive(PartialEq)]
pub enum Status {
  Locked,
  Unlocking,
  Unlocked,
}

pub struct Unlocked;

pub struct Vault {
  pub options: KeeOptions,
  pub status: Status,
  pub error: Option<SharedString>,
  pub entries: Vec<EntryRow>,
}

impl EventEmitter<Unlocked> for Vault {}

impl Vault {
  pub fn new(options: KeeOptions) -> Self {
    Self { options, status: Status::Locked, error: None, entries: Vec::new() }
  }

  pub fn unlock(&mut self, password: String, cx: &mut Context<Self>) {
    assert!(self.status == Status::Locked, "unlock while not locked");
    self.status = Status::Unlocking;
    self.error = None;
    cx.notify();

    let options = KeeOptions {
      keepassdb: self.options.keepassdb.clone(),
      keepassdb_keyfile: self.options.keepassdb_keyfile.clone(),
      keepassdb_password: Some(password),
      s3_access_key: self.options.s3_access_key.clone(),
      s3_secret_key: self.options.s3_secret_key.clone(),
    };
    let task = cx.background_executor().spawn(async move { open_database(&options) });
    cx.spawn(async move |this, cx| {
      let result = task.await;
      // Fails only when the vault was dropped, i.e. the window closed meanwhile.
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
    })
    .detach();
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
        }]
      }
    })
    .collect()
}
