use key::{
  db::{get_database, get_database_key, KeeOptions},
  NodeRef,
};
use std::{env, sync::Mutex};
use tauri::{AppHandle, Manager};

#[derive(serde::Serialize)]
struct Entry {
  uuid: String,
  title: Option<String>,
  user: Option<String>,
  password: Option<String>,
}

impl From<key::Entry> for Entry {
  fn from(entry: key::Entry) -> Self {
    Entry {
      uuid: entry.uuid.to_string(),
      title: entry.get_title().map(str::to_string),
      user: entry.get_username().map(str::to_string),
      password: entry.get_password().map(str::to_string),
    }
  }
}

#[tauri::command]
async fn entry(app: AppHandle, name: String) -> Result<Entry, String> {
  let state = app.state::<Mutex<AppState>>();
  let mut s = state.lock().unwrap();
  let db = s.db.as_mut().unwrap();

  if let Some(NodeRef::Entry(e)) = db.root.clone().get(&[name.as_str()]) {
    return Ok(Entry::from(e.clone()));
  } else {
    return Err("Cant find entry".into());
  }
}

#[tauri::command]
async fn set_entry_field(
  app: AppHandle,
  name: String,
  field: String,
  value: String,
) -> Result<(), String> {
  let state = app.state::<Mutex<AppState>>();
  let mut s = state.lock().unwrap();
  let db = s.db.as_mut().unwrap();

  if let Some(NodeRef::Entry(e)) = db.root.clone().get(&[name.as_str()]) {
    let entry = Entry::from(e.clone());
  } else {
    return Err("Cant find entry".into());
  }

  Ok(())
}

#[tauri::command]
async fn get_entry_field(
  app: AppHandle,
  name: String,
  field: String,
) -> Result<Option<String>, String> {
  let state = app.state::<Mutex<AppState>>();
  let s = state.lock().unwrap();
  let db = s.db.clone();

  if let Some(db) = db {
    if let Some(NodeRef::Entry(e)) = db.root.get(&[name.as_str()]) {
      let entry = e.clone();
      return Ok(entry.get(field.as_str()).map(|v| v.to_string()));
    } else {
      return Err("Cant find entry".into());
    }
  }
  Err("Fuck".to_string())
}

#[tauri::command]
async fn list(app: AppHandle) -> Result<String, ()> {
  let state = app.state::<Mutex<AppState>>();
  let s = state.lock().unwrap();
  let db = s.db.clone();

  if let Some(db) = db {
    let res = key::to_json(db).unwrap();
    return Ok(res);
  }
  Err(())
}

#[tauri::command]
async fn unlock(app: AppHandle, password: String) -> Result<(), String> {
  let state = app.state::<Mutex<AppState>>();

  let mut options: KeeOptions = env::vars().into();
  options.keepassdb_password = Some(password);

  let key = if let Ok(key) = get_database_key(&options) {
    key
  } else {
    return Err("Key failed".to_string());
  };

  let db = if let Ok(db) = get_database(&options, &key).await {
    db
  } else {
    return Err("Database fail".to_string());
  };

  state.lock().unwrap().db = Some(db);

  Ok(())
}

struct AppState {
  db: Option<key::Database>,
}

impl Default for AppState {
  fn default() -> Self {
    Self { db: None }
  }
}

impl AppState {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(move |app| {
      app.manage(Mutex::new(AppState::default()));
      Ok(())
    })
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![
      list,
      entry,
      unlock,
      get_entry_field
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
