use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use demand::Input;
use keepass::{
  db::{Entry, Node, NodeRef, NodeRefMut, Value},
  DatabaseKey,
};
use key::{generate_password, get_database, parse_node_tree, write_database, KeeOptions, KeyNode};
use log::debug;
use std::{env, fs::File};
use url::Url;

/// Command Line Interface to a local or remote keepass database.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  /// Path to the keyfile
  #[arg(short = 'k', long, env = "KEY_KEYFILE")]
  keyfile: Option<String>,

  /// Url to the keepass database file (supports file:// and s3:// schemas)
  #[arg(long, env = "KEY_DATABASE_URL")]
  kdbx: Option<String>,

  /// Database password [env: KEY_PASSWORD]
  #[arg(short = 'p', long)]
  password: Option<String>,

  /// S3 access key [env: KEY_S3_ACCESS_KEY]
  #[arg(long)]
  s3_access_key: Option<String>,

  /// S3 secret key [env: KEY_S3_SECRET_KEY]
  #[arg(long)]
  s3_secret_key: Option<String>,

  #[command(subcommand)]
  command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
  /// Generate a One time password
  OTP {
    /// Name of entry
    name: String,

    /// Field to get
    #[arg(long, default_value = "otp")]
    field: String,
  },

  /// Generate a new password
  Gen {
    /// Length of password
    #[arg(long, default_value = "18")]
    length: usize,
  },

  /// List all entries of the database
  List {
    /// Output format (json, yaml, toml)
    #[arg(short = 'o', long)]
    output: Option<String>,
  },

  /// Get a specific entry from the database
  Get {
    /// Name of entry
    name: String,

    /// Field to get
    #[arg(long, default_value = "Password")]
    field: String,
  },

  /// Set the value of a specific entry in the database
  Set {
    /// Name of entry
    name: String,
    /// Password to set
    value: String,

    /// Field to set
    #[arg(long, default_value = "Password")]
    field: String,
  },

  /// Delete a specific entry from the database
  Delete {
    /// Name of entry
    name: String,
  },

  /// Rename a specific entry in the database
  Rename {
    /// Name of entry
    name: String,

    /// New name of entry
    new_name: String,
  },
}

fn options_from_cli(cli: &Cli) -> Result<KeeOptions> {
  let keepassdb = cli.kdbx.clone();
  let keepassdb_keyfile = cli.keyfile.clone();
  let keepassdb_password = cli.password.clone().or(env::var("KEY_PASSWORD").ok());
  let s3_access_key = cli
    .s3_access_key
    .clone()
    .or(env::var("KEY_S3_ACCESS_KEY").ok());
  let s3_secret_key = cli
    .s3_secret_key
    .clone()
    .or(env::var("KEY_S3_SECRET_KEY").ok());

  if keepassdb.is_none() {
    return Err(anyhow::format_err!("No database url provided."));
  }

  Ok(KeeOptions {
    keepassdb: keepassdb.unwrap(),
    keepassdb_keyfile,
    keepassdb_password,
    s3_access_key,
    s3_secret_key,
  })
}

fn read_password(title: String) -> String {
  let t = Input::new(title).placeholder("Password").password(true);
  t.run().expect("error running input")
}

fn get_database_key(options: &KeeOptions) -> Result<DatabaseKey> {
  let dburl_parsed = Url::parse(&options.keepassdb.as_str())?;
  let name = dburl_parsed.path().split('/').last().unwrap().to_string();

  let mut key = DatabaseKey::new();

  let keypath = &options.keepassdb_keyfile;
  if let Some(keypath) = keypath {
    key = key.with_keyfile(&mut File::open(keypath)?)?;
  }

  let password = &options.keepassdb_password;
  if let Some(password) = password {
    key = key.with_password(password.as_str())
  } else {
    key = key.with_password(read_password(format!("Password for {}", name)).as_str());
  }

  Ok(key)
}

async fn command_list(options: &KeeOptions, format: &str) -> Result<()> {
  let key = get_database_key(&options)?;
  let db = get_database(&options, &key).await?;

  let entries = db.root.children;

  match format {
    "json" => {
      let nodes: Vec<KeyNode> = entries.iter().map(|n| parse_node_tree(n)).collect();
      println!("{}", serde_json::to_string(&nodes)?);
    }
    _ => {
      for entry in entries.iter() {
        match entry {
          Node::Group(g) => {
            for child in g.children.iter() {
              match child {
                Node::Entry(e) => {
                  println!("{}/{}", g.name, e.get_title().unwrap().to_string());
                }
                _ => continue,
              }
            }
          }
          Node::Entry(e) => {
            println!("{}", e.get_title().unwrap().to_string());
          }
        };
      }
    }
  }

  Ok(())
}

async fn command_get(options: &KeeOptions, name: &String, field: &String) -> Result<()> {
  let key = get_database_key(&options)?;
  let db = get_database(&options, &key).await?;

  if let Some(NodeRef::Entry(e)) = db.root.get(&[name]) {
    println!("{}", e.get(field).unwrap().to_string());
    return Ok(());
  }

  Err(anyhow::format_err!("Entry not found"))
}

async fn command_otp(options: &KeeOptions, name: &String, field: &String) -> Result<()> {
  let key = get_database_key(&options)?;
  let db = get_database(&options, &key).await?;

  if let Some(NodeRef::Entry(e)) = db.root.get(&[name]) {
    let value = e.get(field).unwrap().to_string();
    let mut password = value.clone();
    if let Ok(url) = Url::parse(&value) {
      let mut query = url.query_pairs();
      let secret = query.find(|x| x.0.eq("secret")).unwrap();
      password = secret.1.to_string();
    }
    let result = key::otp(password)?;

    println!("{}", result);

    return Ok(());
  }

  Err(anyhow::format_err!("Entry not found or does not have otp"))
}

async fn command_set(
  options: &KeeOptions,
  name: &String,
  value: &String,
  field: &String,
) -> Result<()> {
  let key = get_database_key(&options)?;
  let mut db = get_database(&options, &key).await?;

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

    debug!("Added entry {}", name);

    write_database(&options, &mut db, &key).await?;
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

    debug!("Set entry field {} to {}", field, value);

    write_database(&options, &mut db, &key).await?;
  }

  Ok(())
}

async fn command_rename(options: &KeeOptions, name: &String, new_name: &String) -> Result<()> {
  let key = get_database_key(&options)?;
  let mut db = get_database(&options, &key).await?;

  let entry = db.root.get_mut(&[name]);

  if entry.is_none() {
    Err(anyhow::format_err!("Entry not found"))?;
  }

  if let Some(NodeRefMut::Entry(entry)) = entry {
    let title = entry.fields.get_mut("Title").unwrap();
    *title = Value::Unprotected(new_name.clone());
    debug!("Set Title of field {} to {}", name, new_name);
    write_database(&options, &mut db, &key).await?;
  }

  Ok(())
}

async fn command_delete(options: &KeeOptions, name: &String) -> Result<()> {
  let key = get_database_key(&options)?;
  let mut db = get_database(&options, &key).await?;

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

  debug!("Deleted entry {} at {}", name, index);
  write_database(&options, &mut db, &key).await?;
  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  let cli = Cli::parse();
  let options = options_from_cli(&cli)?;

  debug!("options {:?}", options);

  match &cli.command {
    Some(Commands::List { output }) => {
      command_list(&options, output.as_deref().unwrap_or("text")).await
    }
    Some(Commands::Get { name, field }) => command_get(&options, name, field).await,
    Some(Commands::OTP { name, field }) => command_otp(&options, name, field).await,
    Some(Commands::Set { name, value, field }) => command_set(&options, name, value, field).await,
    Some(Commands::Delete { name }) => command_delete(&options, name).await,
    Some(Commands::Rename { name, new_name }) => command_rename(&options, name, new_name).await,
    Some(Commands::Gen { length }) => {
      println!("{}", generate_password(length));
      Ok(())
    }
    None => {
      Cli::command().print_help()?;
      println!("No command provided.");
      Ok(())
    }
  }
}
