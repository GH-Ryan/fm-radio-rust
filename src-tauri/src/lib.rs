pub mod commands;
pub mod error;
pub mod player;
pub mod radio;
pub mod store;

use crate::commands::AppState;
use crate::player::Player;
use crate::store::{FavoritesStore, LastStationStore};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex as TMutex;

/// Starts the application.
///
/// The binary target is a shim that calls this. Keeping the wiring here rather
/// than in main.rs is what lets the crate be built as a library, which is how
/// Tauri loads an application on the platforms that host one.
pub fn run() {
    tracing_subscriber::fmt::init();

    let player = Arc::new(Player::new());
    let favorites = Arc::new(FavoritesStore::new().expect("Failed to create favorites store"));
    let last_station =
        Arc::new(LastStationStore::new().expect("Failed to create last station store"));
    let client = Arc::new(radio::RadioClient::new().expect("Failed to create radio client"));

    if let Ok((station, _)) = last_station.get() {
        player.restore(station);
    }

    let state = AppState {
        player: player.clone(),
        favorites,
        last_station,
        client,
        browse_pool: Arc::new(TMutex::new(Vec::new())),
        fav_pool: Arc::new(TMutex::new(Vec::new())),
        search_seq: Arc::new(TMutex::new(0)),
    };

    let app = tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            // The player reports every state change, including the song title
            // arriving mid-stream, and each one is pushed to the web layer.
            // Without this the interface only learns about a station when it
            // happens to be polled, and the title lags behind the music.
            let handle = app.handle().clone();
            let favorites_for_events = app.state::<AppState>().favorites.clone();
            app.state::<AppState>().player.set_on_change(move |state| {
                let view = commands::player_view(&state, &favorites_for_events);
                // A window that is closing cannot receive events, and asking it
                // to would abort the process.
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.emit("player", view);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::top_stations,
            commands::search_stations,
            commands::get_favorites,
            commands::toggle_favorite,
            commands::play_station,
            commands::play_pause,
            commands::stop_playback,
            commands::play_random,
            commands::set_volume,
            commands::get_state,
        ])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => {
            app.run(|_handle, _event| {});
        }
        Err(e) => eprintln!("启动失败: {e}"),
    }
}
