use std::{fs, path::PathBuf};

use key::db::KeeOptions;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DatabaseConfig {
  pub name: String,
  /// `file://` or `s3://` url of the kdbx file.
  pub url: String,
  pub keyfile: Option<String>,
  pub s3_access_key: Option<String>,
  pub s3_secret_key: Option<String>,
}

impl DatabaseConfig {
  pub fn options(&self, password: String) -> KeeOptions {
    KeeOptions {
      keepassdb: self.url.clone(),
      keepassdb_keyfile: self.keyfile.clone(),
      keepassdb_password: Some(password),
      s3_access_key: self.s3_access_key.clone(),
      s3_secret_key: self.s3_secret_key.clone(),
    }
  }
}

/// The database the CLI uses, configured through `KEY_*` environment variables.
pub fn from_env() -> Option<DatabaseConfig> {
  let url = std::env::var("KEY_DATABASE_URL").ok()?;
  let name = url.rsplit('/').next().expect("rsplit yields at least one item").to_string();
  Some(DatabaseConfig {
    name,
    keyfile: std::env::var("KEY_KEYFILE").ok(),
    s3_access_key: std::env::var("KEY_S3_ACCESS_KEY").ok(),
    s3_secret_key: std::env::var("KEY_S3_SECRET_KEY").ok(),
    url,
  })
}

/// `~/.key/databases.json`, next to the database cache in `~/.key/cache`.
pub fn config_path() -> PathBuf {
  home::home_dir().expect("could not determine home directory").join(".key/databases.json")
}

/// A missing file means nothing has been configured yet.
pub fn load(path: &PathBuf) -> Vec<DatabaseConfig> {
  if !path.exists() {
    return Vec::new();
  }
  let json = fs::read_to_string(path).expect("failed to read database config");
  serde_json::from_str(&json).expect("failed to parse database config")
}

pub fn save(path: &PathBuf, databases: &[DatabaseConfig]) {
  fs::create_dir_all(path.parent().expect("config path has no parent"))
    .expect("failed to create config directory");
  let json = serde_json::to_string_pretty(databases).expect("failed to serialize config");
  fs::write(path, json).expect("failed to write database config");
}
