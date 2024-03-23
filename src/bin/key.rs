use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use demand::Input;
use keepass::{
    db::{Entry, Node, NodeRef, NodeRefMut, Value},
    Database, DatabaseKey,
};
use log::debug;
use minio::s3::{args::ObjectConditionalReadArgs, client::Client, http::BaseUrl};
use std::{env, fs::File};
use url::Url;

#[derive(Debug)]
struct CliOptions {
    keepassdb: String,
    keepassdb_keyfile: Option<String>,
    keepassdb_password: Option<String>,
}

impl CliOptions {
    fn from_cli(cli: &Cli) -> Result<Self> {
        let keepassdb = cli.kdbx.clone();
        let keepassdb_keyfile = cli.keyfile.clone();
        let keepassdb_password = cli.password.clone().or(env::var("KEEPASSDB_PASSWORD").ok());

        if keepassdb.is_none() {
            return Err(anyhow::format_err!("No database url provided."));
        }

        Ok(Self {
            keepassdb: keepassdb.unwrap(),
            keepassdb_keyfile,
            keepassdb_password,
        })
    }
}

/// Command Line Interface to a local or remote keepass database.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the keyfile
    #[arg(short = 'k', long, env = "KEEPASSDB_KEYFILE")]
    keyfile: Option<String>,

    /// Url to the keepass database file (supports file:// and s3:// schemas)
    #[arg(long, env = "KEEPASSDB")]
    kdbx: Option<String>,

    /// Database password [env: KEEPASSDB_PASSWORD]
    #[arg(long)]
    password: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all entries of the database
    List {},

    /// Get a specific entry from the database
    Get {
        /// Name of entry
        name: String,
    },

    /// Set the value of a specific entry in the database
    Set {
        /// Name of entry
        name: String,
        /// Password to set
        value: String,
    },
}

fn read_password(title: String) -> String {
    let t = Input::new(title).placeholder("Password").password(true);
    t.run().expect("error running input")
}

fn get_database_key(options: &CliOptions) -> Result<DatabaseKey> {
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

async fn get_database(options: &CliOptions, key: &DatabaseKey) -> Result<Database> {
    let dburl = &options.keepassdb.as_str();
    let dburl_parsed = Url::parse(dburl)?;
    let schema = dburl_parsed.scheme();

    let source = match schema {
        "file" => {
            let path = dburl_parsed.path();
            let file = File::open(path)?;
            Ok(file)
        }
        "s3" => {
            // let static_provider = StaticProvider::new(
            //     "Q3AM3UQ867SPQQA43P2F",
            //     "zuf+tfteSlswRu7BJ86wekitnifILbZam1KYY3TG",
            //     None,
            // );

            let base_url: BaseUrl = dburl_parsed.host_str().unwrap().parse::<BaseUrl>()?;
            let client = Client::new(base_url, None, None, None).unwrap();

            let bucket_and_path = dburl_parsed.path()[1..].split_once('/');
            let bucket = bucket_and_path.unwrap().0;
            let object_path = bucket_and_path.unwrap().1;

            debug!("bucket={}  object={}", bucket, object_path);

            let args = &ObjectConditionalReadArgs::new(bucket, object_path).unwrap();
            let obj = client.get_object(args).await;

            debug!("{:?}", obj);

            Err(anyhow::format_err!("S3 not supported yet"))
        }
        _ => Err(anyhow::format_err!("Unsupported schema \"{}\"", schema)),
    };

    if let Err(e) = source {
        return Err(e);
    }

    let db = Database::open(&mut source.unwrap(), key.clone())?;

    Ok(db)
}

async fn write_database(options: &CliOptions, db: &mut Database, key: &DatabaseKey) -> Result<()> {
    debug!("writing database");

    let dburl_parsed = Url::parse(&options.keepassdb)?;
    let schema = dburl_parsed.scheme();

    match schema {
        "file" => {
            let path = dburl_parsed.path();
            db.save(&mut File::create(path)?, key.clone())?;
            Ok(())
        }
        "s3" => Err(anyhow::format_err!("S3 not supported yet")),
        _ => Err(anyhow::format_err!("Unsupported schema \"{}\"", schema)),
    }
}

async fn command_list(options: &CliOptions) -> Result<()> {
    let key = get_database_key(&options)?;
    let db = get_database(&options, &key).await?;

    let entries = db.root.children;

    for entry in entries.iter() {
        let entry = match entry {
            Node::Entry(e) => e,
            _ => continue,
        };
        println!("{}", entry.get_title().unwrap().to_string());
    }

    Ok(())
}

async fn command_get(options: &CliOptions, name: &String) -> Result<()> {
    let key = get_database_key(&options)?;
    let db = get_database(&options, &key).await?;

    if let Some(NodeRef::Entry(e)) = db.root.get(&[name]) {
        println!("{}", e.get_password().unwrap().to_string());
        return Ok(());
    }

    Err(anyhow::format_err!("Entry not found"))
}

async fn command_set(options: &CliOptions, name: &String, value: &String) -> Result<()> {
    let key = get_database_key(&options)?;
    let mut db = get_database(&options, &key).await?;

    let entry = db.root.get_mut(&[name]);

    if entry.is_none() {
        // add a new one
        let mut new_entry = Entry::new();
        new_entry
            .fields
            .insert("Title".to_string(), Value::Unprotected(name.to_string()));
        new_entry.fields.insert(
            "Password".to_string(),
            Value::Protected(value.as_bytes().into()),
        );
        db.root.add_child(new_entry);
    } else {
        if let Some(NodeRefMut::Entry(entry)) = entry {
            let pw = entry.fields.get_mut("Password");

            if pw.is_none() {
                entry.fields.insert(
                    "Password".to_string(),
                    Value::Protected(value.as_bytes().into()),
                );
            } else if let Some(pw) = pw {
                *pw = Value::Protected(value.as_bytes().into());
            }

            println!("{}", entry.get_password().unwrap().to_string());

            write_database(&options, &mut db, &key).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let options = CliOptions::from_cli(&cli)?;

    debug!("options {:?}", options);

    match &cli.command {
        Some(Commands::List {}) => command_list(&options).await,
        Some(Commands::Get { name }) => command_get(&options, name).await,
        Some(Commands::Set { name, value }) => command_set(&options, name, value).await,
        None => {
            Cli::command().print_help()?;
            println!("No command provided.");
            Ok(())
        }
    }
}
