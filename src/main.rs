#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

fn main() {
    // The DMABUF renderer hands framebuffers to Mesa through EGL, and tearing
    // that down as the web process exits corrupts the heap: glibc's free()
    // then walks a chunk whose fd pointer is garbage and segfaults, which shows
    // up seconds after the window closes as a WebKitWebProcess core dump.
    // Rendering through a shared-memory path instead avoids touching that
    // teardown. Must be set before the web process is spawned; the child
    // inherits our environment. Setting the variable to 0 restores DMABUF.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

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
            // Only Linux needs the decoration turned off here. The window is
            // already declared undecorated in tauri.conf.json, but WebKitGTK
            // ignores that and has to be told at runtime, so the call is kept
            // for Linux alone rather than run everywhere.
            #[cfg(target_os = "linux")]
            {
                let window = app.get_webview_window("main").unwrap();
                let _ = window.set_decorations(false);
            }

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
