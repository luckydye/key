use key::db::{get_database, get_database_key, KeeOptions};
use std::env;
use tauri::{AppHandle, Manager};

#[tauri::command]
async fn list(app: AppHandle) -> Result<String, ()> {
  let options = app.state::<KeeOptions>();

  let key = &get_database_key(&options).unwrap();
  let db = get_database(&options, key).await.unwrap();
  let res = key::to_json(db).unwrap();

  Ok(res)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      let options: KeeOptions = env::vars().into();
      println!("{:?}", options);
      app.manage(options);
      Ok(())
    })
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![list])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
