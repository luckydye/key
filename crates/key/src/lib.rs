use anyhow::Result;
use keepass::db::Node;
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};

#[cfg(not(target_arch = "wasm32"))]
pub mod db;

#[cfg(not(target_arch = "wasm32"))]
pub mod pw;

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEntry {
  title: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyGroup {
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

pub fn parse_node_tree(node: &Node) -> KeyNode {
  match node {
    Node::Group(g) => {
      let entries: Vec<KeyNode> = g.children.iter().map(parse_node_tree).collect();
      KeyNode::Group(KeyGroup {
        title: g.name.clone(),
        entries,
      })
    }
    Node::Entry(e) => KeyNode::Entry(KeyEntry {
      title: e.get_title().unwrap().to_string(),
    }),
  }
}

pub fn otp(secret: String, issuer: Option<String>, account: Option<String>) -> Result<String> {
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
