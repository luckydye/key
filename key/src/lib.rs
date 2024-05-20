use std::{fs, io::Cursor};

use anyhow::{anyhow, Result};
use keepass::{Database, DatabaseKey};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};

pub use keepass::db::{Entry, Node, NodeRef, NodeRefMut, Value};

#[cfg(not(target_arch = "wasm32"))]
pub mod db;

static PASSWORD_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\
    0123456789!@#$%^&*()_+-=[]{}|;':,.<>?";

pub fn generate_password(length: &usize) -> String {
  random_string::generate(*length, PASSWORD_CHARSET)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEntry {
  uuid: String,
  title: String,
  user: Option<String>,
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
  let entries = db.root.children;
  let nodes: Vec<KeyNode> = entries
    .iter()
    .map(|n| {
      let node = parse_node_tree(n);
      node
    })
    .collect();
  Ok(serde_json::to_string(&nodes)?)
}

pub fn delete_entry(db: &mut Database, name: &String) -> Result<()> {
  let entry = db.root.get_mut(&[name]);
  if entry.is_none() {
    Err(anyhow::format_err!("Entry not found"))?;
  }

  let index = db
    .root
    .children
    .iter()
    .position(|n| {
      if let Node::Entry(e) = n {
        e.get_title().unwrap() == name
      } else {
        false
      }
    })
    .unwrap();

  db.root.children.remove(index);

  Ok(())
}

pub fn rename_entry(db: &mut Database, name: &String, new_name: &String) -> Result<()> {
  let entry = db.root.get_mut(&[name]);

  if entry.is_none() {
    Err(anyhow::format_err!("Entry not found"))?;
  }

  if let Some(NodeRefMut::Entry(entry)) = entry {
    let title = entry.fields.get_mut("Title").unwrap();
    *title = Value::Unprotected(new_name.clone());
    return Ok(());
  }

  Err(anyhow!("failed to rename entry"))
}

pub fn set_entry(
  db: &mut Database,
  name: &String,
  value: &String,
  field: &String,
) -> Result<()> {
  let entry = db.root.get_mut(&[name]);

  if entry.is_none() {
    // add a new one
    let mut new_entry = Entry::new();
    new_entry
      .fields
      .insert("Title".to_string(), Value::Unprotected(name.to_string()));
    new_entry
      .fields
      .insert(field.to_string(), Value::Protected(value.as_bytes().into()));
    db.root.add_child(new_entry);

    return Ok(());
  }

  if let Some(NodeRefMut::Entry(entry)) = entry {
    let pw = entry.fields.get_mut(&field.to_string());

    if pw.is_none() {
      entry
        .fields
        .insert(field.to_string(), Value::Protected(value.as_bytes().into()));
    } else if let Some(pw) = pw {
      *pw = Value::Protected(value.as_bytes().into());
    }

    return Ok(());
  }

  Err(anyhow!("failed to set value"))
}

pub fn get_entry(db: &Database, name: &String, field: &String) -> Result<String> {
  if let Some(NodeRef::Entry(e)) = db.root.get(&[name]) {
    return Ok(e.get(field).unwrap().to_string());
  }
  Err(anyhow!("Entry not found"))
}

pub fn get_entry_file(db: &Database, _name: &String, _file: &String) -> Result<String> {
  for a in db.header_attachments.clone() {
    let path =
      String::from_utf8(a.content.clone()).expect("Our bytes should be valid utf8");
    fs::write(path, a.content)?;
  }
  Err(anyhow!("failed to get value"))
}

pub fn get_entry_otp(db: &Database, name: &String, field: &String) -> Result<String> {
  if let Some(NodeRef::Entry(e)) = db.root.get(&[name]) {
    let password = e.get(field).unwrap().to_string();
    return Ok(otp(password, None, None)?);
  }
  Err(anyhow::format_err!("Entry not found or does not have otp"))
}

pub fn parse_node_tree(node: &Node) -> KeyNode {
  match node {
    Node::Group(g) => {
      let entries: Vec<KeyNode> = g.children.iter().map(parse_node_tree).collect();
      KeyNode::Group(KeyGroup {
        uuid: g.uuid.to_string(),
        title: g.name.clone(),
        entries,
      })
    }
    Node::Entry(e) => KeyNode::Entry(KeyEntry {
      uuid: e.uuid.to_string(),
      title: e.get_title().unwrap().to_string(),
      user: e.get_username().map(str::to_string),
      has_otp: e.fields.contains_key("otp"),
    }),
  }
}

pub fn otp(
  secret: String,
  issuer: Option<String>,
  account: Option<String>,
) -> Result<String> {
  if secret.starts_with("otpauth:") {
    let totp = TOTP::from_url_unchecked(secret).unwrap();
    Ok(totp.generate_current()?)
  } else {
    let totp = TOTP::new_unchecked(
      Algorithm::SHA1,
      6,
      1,
      30,
      Secret::Encoded(secret).to_bytes()?,
      issuer,
      account.or(Some("".to_string())).unwrap(),
    );
    Ok(totp.generate_current()?)
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
