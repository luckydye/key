use std::io::Cursor;

use anyhow::{anyhow, Result};
pub use keepass::{Database, DatabaseKey};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};

pub use keepass::db::{fields, Entry, EntryRef, GroupRef, Value};

pub static PASSWORD_CHARSET: &str =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\
    0123456789!@#$%^&*()_+-=[]{}|;':,.<>?";

pub fn generate_password(length: &usize) -> String {
  random_string::generate(*length, PASSWORD_CHARSET)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEntry {
  uuid: String,
  title: String,
  user: Option<String>,
  website: Option<String>,
  has_otp: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyGroup {
  uuid: String,
  title: String,
  entries: Vec<KeyNode>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum KeyNode {
  #[serde(rename = "entry")]
  Entry(KeyEntry),
  #[serde(rename = "group")]
  Group(KeyGroup),
}

pub fn to_json(db: Database) -> Result<String> {
  Ok(serde_json::to_string(&group_nodes(&db.root()))?)
}

pub fn delete_entry(db: &mut Database, name: &String) -> Result<()> {
  let mut root = db.root_mut();
  let entry = root.entry_by_name_mut(name).ok_or_else(|| anyhow!("Entry not found"))?;
  entry.remove();
  Ok(())
}

pub fn rename_entry(db: &mut Database, name: &String, new_name: &String) -> Result<()> {
  let mut root = db.root_mut();
  let mut entry = root.entry_by_name_mut(name).ok_or_else(|| anyhow!("Entry not found"))?;
  entry.set_unprotected(fields::TITLE, new_name.as_str());
  Ok(())
}

/// Sets `field` of the entry titled `name`, creating the entry when it does not exist.
pub fn set_entry(
  db: &mut Database,
  name: &String,
  value: &String,
  field: &String,
) -> Result<()> {
  let mut root = db.root_mut();
  if root.as_ref().entry_by_name(name).is_none() {
    root.add_entry().set_unprotected(fields::TITLE, name.as_str());
  }
  let mut entry = root.entry_by_name_mut(name).expect("entry was just created");
  entry.set_protected(field.as_str(), value.as_str());
  Ok(())
}

pub fn get_entry(db: &Database, name: &String, field: &String) -> Result<String> {
  let root = db.root();
  let entry = root.entry_by_name(name).ok_or_else(|| anyhow!("Entry not found"))?;
  Ok(entry.get(field).ok_or_else(|| anyhow!("Entry has no field {field}"))?.to_string())
}

pub fn get_entry_otp(db: &Database, name: &String, field: &String) -> Result<String> {
  otp(get_entry(db, name, field)?, None, None)
}

/// Entries followed by subgroups of `group`, in database order.
pub fn group_nodes(group: &GroupRef) -> Vec<KeyNode> {
  let entries = group.entries().map(|e| {
    KeyNode::Entry(KeyEntry {
      uuid: e.id().uuid().to_string(),
      title: e.get_title().expect("entry without title").to_string(),
      user: e.get_username().map(str::to_string),
      website: e.get_url().map(str::to_string),
      has_otp: e.fields.contains_key(fields::OTP),
    })
  });
  let groups = group.groups().map(|g| {
    KeyNode::Group(KeyGroup {
      uuid: g.id().uuid().to_string(),
      title: g.name.clone(),
      entries: group_nodes(&g),
    })
  });
  entries.chain(groups).collect()
}

pub fn otp(
  secret: String,
  issuer: Option<String>,
  account: Option<String>,
) -> Result<String> {
  Ok(totp(secret, issuer, account)?.generate_current()?)
}

pub fn totp(secret: String, issuer: Option<String>, account: Option<String>) -> Result<TOTP> {
  if secret.starts_with("otpauth:") {
    TOTP::from_url_unchecked(secret).map_err(|e| anyhow::anyhow!("{:?}", e))
  } else {
    Ok(TOTP::new_unchecked(
      Algorithm::SHA1,
      6,
      1,
      30,
      Secret::Encoded(secret).to_bytes()?,
      issuer,
      account.or(Some("".to_string())).unwrap(),
    ))
  }
}

pub fn key_from(
  password: Option<String>,
  keyfile: Option<Vec<u8>>,
) -> Result<DatabaseKey> {
  let mut key = DatabaseKey::new();

  if let Some(keyfile) = keyfile {
    let mut cursor = Cursor::new(keyfile);
    key = key.with_keyfile(&mut cursor)?;
  }

  if let Some(password) = password {
    key = key.with_password(password.as_str())
  }

  Ok(key)
}

pub fn db_from(source: Vec<u8>, key: DatabaseKey) -> Result<Database> {
  let mut cursor = Cursor::new(source);
  Ok(Database::open(&mut cursor, key.clone())?)
}
