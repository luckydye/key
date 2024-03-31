use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use demand::Input;
use keepass::{
    db::{Entry, Node, NodeRef, NodeRefMut, Value},
    Database, DatabaseKey,
};
use log::{debug, info};
use minio::s3::{
    args::{BucketExistsArgs, ObjectConditionalReadArgs, PutObjectArgs},
    client::Client,
    creds::StaticProvider,
    http::BaseUrl,
};
use std::{
    env,
    fs::File,
    io::{Cursor, Read},
};
use url::Url;

// TODO: refactor this into a separate module, s3, filesystem, and more backends can be added.

#[derive(Debug)]
struct CliOptions {
    keepassdb: String,
    keepassdb_keyfile: Option<String>,
    keepassdb_password: Option<String>,
    s3_access_key: Option<String>,
    s3_secret_key: Option<String>,
}

impl CliOptions {
    fn from_cli(cli: &Cli) -> Result<Self> {
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

        Ok(Self {
            keepassdb: keepassdb.unwrap(),
            keepassdb_keyfile,
            keepassdb_password,
            s3_access_key,
            s3_secret_key,
        })
    }
}

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
    /// List all entries of the database
    List {},

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

#[derive(Debug)]
struct S3Location {
    pub bucket: String,
    pub object: String,
}

fn parse_s3_url(dburl_parsed: Url) -> S3Location {
    let bucket_and_path = dburl_parsed.path()[1..].split_once('/');
    let bucket = bucket_and_path.unwrap().0;
    let object_path = bucket_and_path.unwrap().1;

    debug!("bucket={}  object={}", bucket, object_path);

    S3Location {
        bucket: bucket.to_string(),
        object: object_path.to_string(),
    }
}

fn get_s3_client(options: &CliOptions, dburl_parsed: &Url) -> Result<Client> {
    let base_url: BaseUrl = dburl_parsed.host_str().unwrap().parse::<BaseUrl>()?;

    if options.s3_access_key.is_some() && options.s3_secret_key.is_some() {
        debug!("Using provided S3 credentials");

        let static_provider = StaticProvider::new(
            options.s3_access_key.as_ref().unwrap().as_str(),
            options.s3_secret_key.as_ref().unwrap().as_str(),
            None,
        );

        let client = Client::new(
            base_url.clone(),
            Some(Box::new(static_provider)),
            None,
            None,
        )
        .unwrap();
        return Ok(client);
    }

    debug!("No S3 credentials provided");

    let client = Client::new(base_url.clone(), None, None, None).unwrap();
    Ok(client)
}

async fn get_database(options: &CliOptions, key: &DatabaseKey) -> Result<Database> {
    let dburl = &options.keepassdb.as_str();
    let dburl_parsed = Url::parse(dburl)?;
    let schema = dburl_parsed.scheme();

    let source = match schema {
        "file" => {
            let mut file = File::open(dburl_parsed.path())?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            Ok(buffer)
        }
        "s3" => {
            // TODO: Cache file locally? Also requires merging the databases then.

            let client = get_s3_client(&options, &dburl_parsed)?;
            let s3_location = parse_s3_url(dburl_parsed);

            debug!("Reading from {:?}", s3_location);

            let args =
                &ObjectConditionalReadArgs::new(&s3_location.bucket, &s3_location.object).unwrap();
            let object = client.get_object(args).await;

            if let Err(obj) = object {
                return Err(anyhow::format_err!("Failed to get object from S3: {}", obj));
            }

            Ok(object.unwrap().bytes().await?.to_vec())
        }
        _ => Err(anyhow::format_err!("Unsupported schema \"{}\"", schema)),
    };

    if let Err(e) = source {
        return Err(e);
    }

    let mut cursor = Cursor::new(source.unwrap());
    Ok(Database::open(&mut cursor, key.clone())?)
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
        "s3" => {
            let mut buf = Vec::new();
            let mut cur = Cursor::new(&mut buf);
            db.save(&mut cur, key.clone())?;

            let size = cur.position();

            cur.set_position(0);

            upload_to_s3(options, &mut cur, size).await?;
            Ok(())
        }
        _ => Err(anyhow::format_err!("Unsupported schema \"{}\"", schema)),
    }
}

async fn upload_to_s3(
    options: &CliOptions,
    file: &mut dyn std::io::Read,
    length: u64,
) -> Result<()> {
    let dburl_parsed = Url::parse(&options.keepassdb)?;
    let client = get_s3_client(&options, &dburl_parsed)?;
    let s3_location = parse_s3_url(dburl_parsed);

    // Check 'bucket_name' bucket exist or not.
    let exists: bool = client
        .bucket_exists(&BucketExistsArgs::new(&s3_location.bucket).unwrap())
        .await?;

    if !exists {
        Err(anyhow::format_err!(
            "Bucket `{}` does not exist",
            s3_location.bucket
        ))?;
    }

    debug!("Uploading to {:?}", s3_location);

    let args = &mut PutObjectArgs::new(
        &s3_location.bucket,
        &s3_location.object,
        file,
        Some(length as usize),
        None,
    )?;

    let res = client.put_object(args).await?;

    debug!("PutObjectResponse: {:?}", res);

    info!(
        "Successfully uploaded object `{}` to bucket `{}`.",
        s3_location.object, s3_location.bucket
    );
    Ok(())
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

async fn command_get(options: &CliOptions, name: &String, field: &String) -> Result<()> {
    let key = get_database_key(&options)?;
    let db = get_database(&options, &key).await?;

    if let Some(NodeRef::Entry(e)) = db.root.get(&[name]) {
        println!("{}", e.get(field).unwrap().to_string());
        return Ok(());
    }

    Err(anyhow::format_err!("Entry not found"))
}

async fn command_set(
    options: &CliOptions,
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
    } else {
        if let Some(NodeRefMut::Entry(entry)) = entry {
            let pw = entry.fields.get_mut("Password");

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
        Some(Commands::Get { name, field }) => command_get(&options, name, field).await,
        Some(Commands::Set { name, value, field }) => {
            command_set(&options, name, value, field).await
        }
        None => {
            Cli::command().print_help()?;
            println!("No command provided.");
            Ok(())
        }
    }
}
