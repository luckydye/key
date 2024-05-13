use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use demand::{DemandOption, Input, Select};
use keepass::{db::Node, DatabaseKey};
use key::{
  db::{get_database, write_database, KeeOptions},
  delete_entry, get_entry, get_entry_file, get_entry_otp, rename_entry, to_json,
};
use key::{generate_password, set_entry};
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

    /// Extract as file
    #[arg(long)]
    file: bool,

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

  /// Chooser ui
  Choose {
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
  let db = get_database(&options, &get_database_key(&options)?).await?;

  match format {
    "json" => {
      println!("{}", to_json(db)?);
    }
    _ => {
      for entry in db.root.children.iter() {
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
  let db = get_database(&options, &get_database_key(&options)?).await?;
  let entry = get_entry(&db, name, field)?;
  println!("{}", entry);
  Ok(())
}

async fn command_get_file(options: &KeeOptions, name: &String, field: &String) -> Result<()> {
  let db = get_database(&options, &get_database_key(&options)?).await?;
  get_entry_file(&db, name, field)?;
  Ok(())
}

async fn command_otp(options: &KeeOptions, name: &String, field: &String) -> Result<()> {
  let db = get_database(&options, &get_database_key(&options)?).await?;
  println!("{}", get_entry_otp(&db, name, field)?);
  Ok(())
}

async fn command_set(
  options: &KeeOptions,
  name: &String,
  value: &String,
  field: &String,
) -> Result<()> {
  let key = get_database_key(&options)?;
  let mut db = get_database(&options, &key).await?;
  set_entry(&mut db, name, value, field)?;
  debug!("Set entry field {} to {}", field, value);
  write_database(&options, &mut db, &key).await?;
  Ok(())
}

async fn command_rename(options: &KeeOptions, name: &String, new_name: &String) -> Result<()> {
  let key = get_database_key(&options)?;
  let mut db = get_database(&options, &key).await?;
  rename_entry(&mut db, name, new_name)?;
  debug!("Set Title of field {} to {}", name, new_name);
  write_database(&options, &mut db, &key).await?;
  Ok(())
}

async fn command_delete(options: &KeeOptions, name: &String) -> Result<()> {
  let key = get_database_key(&options)?;
  let mut db = get_database(&options, &key).await?;
  delete_entry(&mut db, name)?;
  debug!("Deleted entry {}", name);
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
    Some(Commands::Get { name, field, file }) => {
      if file.clone() == true {
        return command_get_file(&options, name, field).await;
      }
      return command_get(&options, name, field).await;
    }
    Some(Commands::Choose { }) => {
    let ms = Select::new("Toppings")
            .description("Select your topping")
            .filterable(true)
            .option(DemandOption::new("Lettuce"))
            .option(DemandOption::new("Tomatoes"))
            .option(DemandOption::new("Charm Sauce"))
            .option(DemandOption::new("Jalapenos").label("Jalapeños"))
            .option(DemandOption::new("Cheese"))
            .option(DemandOption::new("Vegan Cheese"))
            .option(DemandOption::new("Nutella"));
        ms.run().expect("error running select");
      Ok(())
    }
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
