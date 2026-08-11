pub mod commands;
pub mod db;
pub mod market;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&data_dir.join("portfolio.db"))?;
            db::seed_demo_if_first_run(&conn)?;
            app.manage(commands::AppState {
                conn: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_csv,
            commands::get_portfolio,
            commands::refresh_quotes,
            commands::add_manual_transaction,
            commands::update_manual_transaction,
            commands::get_manual_transactions,
            commands::delete_manual_transaction,
            commands::update_transaction,
            commands::revert_transaction,
            commands::delete_demo_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
