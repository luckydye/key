use key::NodeRef;
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

  if let Some(db) = db {
    if let Some(NodeRef::Entry(e)) = db.root.get(&[name.as_str()]) {
      return Ok(Entry::from(e.clone()));
    } else {
      return Err("Cant find entry".into());
    }
  }

  Err("Err".into())
}

#[tauri::command]
async fn list(app: AppHandle) -> Result<String, ()> {
  let state = app.state::<AppState>();

  if let Some(db) = state.db.clone() {
    let res = key::to_json(db).unwrap();
    return Ok(res);
  }

  Err(())
}

struct AppState {
  db: Option<key::Database>,
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
}
