use key::{
  db::{get_database, get_database_key, KeeOptions},
  NodeRef,
};
use std::env;
use tauri::{AppHandle, Manager};

#[derive(serde::Serialize)]
struct Entry {
  uuid: String,
  title: Option<String>,
  user: Option<String>,
}

impl From<key::Entry> for Entry {
  fn from(entry: key::Entry) -> Self {
    Entry {
      uuid: entry.uuid.to_string(),
      title: entry.get_title().map(str::to_string),
      user: entry.get_username().map(str::to_string),
    }
  }
}

#[tauri::command]
async fn entry(app: AppHandle, name: String) -> Result<Entry, String> {
  let state = app.state::<AppState>();
  let db = state.db.clone();

  if let Some(NodeRef::Entry(e)) = db.root.get(&[name.as_str()]) {
    Ok(Entry::from(e.clone()))
  } else {
    Err("Cant find entry".into())
  }
}

#[tauri::command]
async fn list(app: AppHandle) -> Result<String, ()> {
  let state = app.state::<AppState>();
  let res = key::to_json(state.db.clone()).unwrap();
  Ok(res)
}

struct AppState {
  db: key::Database,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> anyhow::Result<()> {
  tauri::Builder::default()
    .setup(move |app| {
      let db = tauri::async_runtime::spawn(async move {
        // also added move here
        let options: KeeOptions = env::vars().into();
        println!("{:?}", options);

        let key = get_database_key(&options).unwrap();
        get_database(&options, &key).await.unwrap()
      });

      app.manage(AppState { db });
      Ok(())
    })
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![list, entry])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");

  Ok(())
}
