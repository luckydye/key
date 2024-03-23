use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use demand::Input;
use keepass::{
    db::{Node, NodeRef},
    Database, DatabaseKey,
};
use log::debug;
use minio::s3::{args::ObjectConditionalReadArgs, client::Client, http::BaseUrl};
use std::{env, fs::File};
use url::Url;

/// Command Line Interface to a local or remote keepass database.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Provide a keyfile
    #[arg(short = 'k', long)]
    keyfile: Option<String>,

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
}

fn read_password(title: String) -> String {
    let t = Input::new(title).placeholder("Password").password(true);
    t.run().expect("error running input")
}

fn get_database_key(
    name: String,
    password: Option<String>,
    keypath: Option<String>,
) -> Result<DatabaseKey> {
    let mut key = DatabaseKey::new();

    if let Some(keypath) = keypath {
        key = key.with_keyfile(&mut File::open(keypath)?)?;
    }

    if let Some(password) = password {
        key = key.with_password(password.as_str())
    } else {
        key = key.with_password(read_password(format!("Password for {}", name)).as_str());
    }

    Ok(key)
}

async fn get_database(
    dburl: &str,
    keypath: Option<String>,
    password: Option<String>,
) -> Result<Database> {
    let dburl_parsed = Url::parse(dburl)?;

    let schema = dburl_parsed.scheme();

    let source = match schema {
        "file" => {
            let p = dburl_parsed.path();
            let f = File::open(p)?;
            Ok(f)
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

    let name = dburl_parsed.path().split('/').last().unwrap().to_string();
    let key = get_database_key(name, password, keypath)?;

    if let Err(e) = source {
        return Err(e);
    }

    let db = Database::open(&mut source.unwrap(), key)?;

    Ok(db)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let KEEPASSDB: Option<String> = env::var("KEEPASSDB").ok();
    let KEEPASSDB_KEYFILE: Option<String> = env::var("KEEPASSDB_KEYFILE").ok();
    let KEEPASSDB_PASSWORD: Option<String> = env::var("KEEPASSDB_PASSWORD").ok();

    if KEEPASSDB.is_none() {
        return Err(anyhow::format_err!("No database url provided."));
    }

    debug!("db url {:?}", KEEPASSDB);

    match &cli.command {
        Some(Commands::List {}) => {
            let db = get_database(
                KEEPASSDB.unwrap().as_str(),
                KEEPASSDB_KEYFILE,
                KEEPASSDB_PASSWORD,
            )
            .await?;

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
        Some(Commands::Get { name }) => {
            let db = get_database(
                KEEPASSDB.unwrap().as_str(),
                KEEPASSDB_KEYFILE,
                KEEPASSDB_PASSWORD,
            )
            .await?;

            if let Some(NodeRef::Entry(e)) = db.root.get(&[name]) {
                println!("{}", e.get_password().unwrap().to_string());
            }

            Ok(())
        }
        None => {
            Cli::command().print_help()?;

            println!("No command provided.");
            Ok(())
        }
    }
}
