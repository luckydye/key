use anyhow::{anyhow, Result};
use keepass::{db::Node, Database, DatabaseKey};
use log::{debug, info};
use minio::s3::{
  args::{BucketExistsArgs, ObjectConditionalReadArgs, PutObjectArgs},
  client::Client,
  creds::StaticProvider,
  http::BaseUrl,
};
use serde::{Deserialize, Serialize};
use std::{
  env,
  fs::{self, File},
  io::{Cursor, Read, Write},
  path::PathBuf,
};
use url::Url;

static PASSWORD_CHARSET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\
    0123456789!@#$%^&*()_+-=[]{}|;':,.<>?";

#[derive(Debug)]
pub struct KeeOptions {
  keepassdb: String,
  keepassdb_keyfile: Option<String>,
  keepassdb_password: Option<String>,
  s3_access_key: Option<String>,
  s3_secret_key: Option<String>,
}

impl KeeOptions {
  pub fn from_env() -> Result<Self> {
    let keepassdb = env::var("KEY_PASSWORD").ok();
    let keepassdb_keyfile = env::var("KEY_PASSWORD").ok();
    let keepassdb_password = env::var("KEY_PASSWORD").ok();
    let s3_access_key = env::var("KEY_S3_ACCESS_KEY").ok();
    let s3_secret_key = env::var("KEY_S3_SECRET_KEY").ok();

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

pub fn generate_password(length: &usize) -> String {
  random_string::generate(*length, PASSWORD_CHARSET)
}

pub fn get_database_key(options: &KeeOptions) -> Result<DatabaseKey> {
  let mut key = DatabaseKey::new();

  let keypath = &options.keepassdb_keyfile;
  if let Some(keypath) = keypath {
    key = key.with_keyfile(&mut File::open(keypath)?)?;
  }

  let password = &options.keepassdb_password;
  if let Some(password) = password {
    key = key.with_password(password.as_str())
  }

  Ok(key)
}

#[derive(Debug)]
pub struct S3Location {
  pub bucket: String,
  pub object: String,
}

pub fn parse_s3_url(dburl_parsed: Url) -> S3Location {
  let bucket_and_path = dburl_parsed.path()[1..].split_once('/');
  let bucket = bucket_and_path.unwrap().0;
  let object_path = bucket_and_path.unwrap().1;

  debug!("bucket={}  object={}", bucket, object_path);

  S3Location {
    bucket: bucket.to_string(),
    object: object_path.to_string(),
  }
}

pub fn get_s3_client(options: &KeeOptions, dburl_parsed: &Url) -> Result<Client> {
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

pub fn cache_dir() -> Result<PathBuf> {
  let dir = match home::home_dir() {
    Some(path) if !path.as_os_str().is_empty() => Ok(path),
    _ => Err(()),
  };

  if dir.is_err() {
    return Err(anyhow!("Could not determine home directory"));
  }

  // Cache in home dir at ~/.key/cache/
  let cache_dir = dir.unwrap().join(".key/cache");

  if !cache_dir.exists() {
    fs::create_dir_all(&cache_dir)?;
  }

  Ok(cache_dir)
}

pub fn cache_database(name: String, file: &Vec<u8>) -> Result<()> {
  let dir = cache_dir()?;
  let mut cache_file = File::create(dir.join(name))?;
  cache_file.write(file)?;
  Ok(())
}

pub fn get_cache_database(name: String) -> Result<Vec<u8>> {
  let dir = cache_dir()?;
  let mut cache_file = File::open(dir.join(name))?;
  let mut buffer = Vec::new();
  cache_file.read_to_end(&mut buffer)?;
  Ok(buffer)
}

pub async fn get_database(options: &KeeOptions, key: &DatabaseKey) -> Result<Database> {
  let dburl = &options.keepassdb.as_str();
  let dburl_parsed = Url::parse(dburl)?;
  let schema = dburl_parsed.scheme();
  let name = dburl_parsed.path().split('/').last().unwrap().to_string();

  let source = match schema {
    "file" => {
      let mut file = File::open(dburl_parsed.path())?;
      let mut buffer = Vec::new();
      file.read_to_end(&mut buffer)?;
      Ok(buffer)
    }
    "s3" => {
      let client = get_s3_client(&options, &dburl_parsed)?;
      let s3_location = parse_s3_url(dburl_parsed);

      debug!("Reading from {:?}", s3_location);

      let args = &ObjectConditionalReadArgs::new(&s3_location.bucket, &s3_location.object).unwrap();
      let object = client.get_object(args).await;

      if let Ok(obj) = object {
        let file = obj.bytes().await?.to_vec();
        // Cache is read-only
        cache_database(name, &file)?;
        Ok(file)
      } else {
        debug!("Failed to get object from S3, {:?}", object);
        debug!("Fallback to cache.");
        if let Ok(file) = get_cache_database(name) {
          Ok(file)
        } else {
          debug!("Failed to get object from cache.");
          Err(anyhow::format_err!(
            "Failed to get object from S3 or cache, {:?}",
            object.err().unwrap()
          ))
        }
      }
    }
    _ => Err(anyhow::format_err!("Unsupported schema \"{}\"", schema)),
  };

  if let Err(e) = source {
    return Err(e);
  }

  let file = source.unwrap();
  let mut cursor = Cursor::new(file);
  Ok(Database::open(&mut cursor, key.clone())?)
}

pub async fn write_database(
  options: &KeeOptions,
  db: &mut Database,
  key: &DatabaseKey,
) -> Result<()> {
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

pub async fn upload_to_s3(
  options: &KeeOptions,
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
