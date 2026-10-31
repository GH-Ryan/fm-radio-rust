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

/// Appends a line to a small diagnostic log next to the system temp files.
///
/// The watchdog's reloads are otherwise invisible: they happen while the window
/// is black or unresponsive, and the release build has no console. Keeping a
/// short record is what makes a recurrence diagnosable instead of a guess.
fn watchdog_log(message: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("fm-radio-watchdog.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{secs}] {message}");
    }
}

/// Restarts the station that was playing when the machine went to sleep.
///
/// A suspend almost always kills the stream socket, and the failure is not
/// always immediate: the blocking read can stay parked on a half-open
/// connection while the interface still says "playing", which is silent
/// playback. The station is therefore started again rather than trusted.
///
/// A user action has the last word: nothing restarts when playback was stopped
/// before the wake, and nothing keeps retrying after the user stops, pauses or
/// switches station in the meantime.
fn resume_after_wake(player: Arc<Player>) {
    let state = player.state();
    let Some(station) = state.station.clone() else {
        return;
    };
    // Nothing was playing and nothing had failed: the user stopped playback
    // before the machine slept, so waking must not start audio on its own.
    if !state.playing && state.error.is_none() {
        return;
    }
    let key = station.key();

    // A resumed machine is often still bringing its network interface up, and
    // the first connection attempt right after the wake tends to fail.
    std::thread::sleep(std::time::Duration::from_secs(5));

    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(12));
        }

        let now = player.state();
        // A user action since the wake wins.
        if now.station.as_ref().map(|s| s.key()) != Some(key.clone()) {
            return;
        }
        if !now.playing && now.error.is_none() && now.playing_key.is_none() {
            return; // stopped by the user
        }
        if now.paused {
            return; // paused by the user
        }
        if attempt > 0 && now.playing && now.error.is_none() {
            return; // the previous attempt is running
        }

        let _ = player.play(station.clone());
        if attempt == 0 {
            watchdog_log("resumed station after wake");
        }
        // A failed connection shows up as an error on the next pass, which is
        // what makes the retry below useful.
    }
}

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
        ui_heartbeat: Arc::new(std::sync::Mutex::new(None)),
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

            // WebView2 can stop painting after the machine wakes from sleep, or
            // after its compositor decides the window is occluded: the native
            // window keeps answering the OS, but the page is gone and only the
            // window background is left, which reads as a black window. Two
            // signals catch that here, and a reload is the way back:
            //
            // - A five-second tick that wakes up far later than it asked means
            //   the machine was suspended. This is the case actually observed:
            //   the app was black only after a five-hour sleep.
            // - The web layer checks in every few seconds; check-ins that stop
            //   while the window is on screen catch a wedged renderer.
            let heartbeat = app.state::<AppState>().ui_heartbeat.clone();
            let player_for_wake = app.state::<AppState>().player.clone();
            let watchdog = app.handle().clone();
            std::thread::spawn(move || {
                watchdog_log("watchdog started");
                let mut last_reload: Option<std::time::Instant> = None;
                let mut last_tick = std::time::Instant::now();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let now = std::time::Instant::now();
                    let gap = now.duration_since(last_tick);
                    last_tick = now;

                    let resumed_from_sleep = gap > std::time::Duration::from_secs(30);
                    let heartbeat_stale = heartbeat
                        .lock()
                        .unwrap()
                        .is_some_and(|t| t.elapsed() > std::time::Duration::from_secs(40));
                    if !resumed_from_sleep && !heartbeat_stale {
                        continue;
                    }

                    let Some(window) = watchdog.get_webview_window("main") else {
                        continue;
                    };
                    // A sleep kills the stream socket; start the station the
                    // user left playing again, whether or not the window was
                    // minimised at the time.
                    if resumed_from_sleep {
                        let player = player_for_wake.clone();
                        std::thread::spawn(move || resume_after_wake(player));
                    }
                    // A minimised window is not being looked at, and a renderer
                    // there is allowed to idle. Reloading it would only throw
                    // the interface away; the check runs again once restored.
                    if window.is_minimized().unwrap_or(false) {
                        continue;
                    }
                    // One reload is not repeated for a minute, so a page that
                    // fails to come back is not reloaded in a loop.
                    if last_reload.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(60))
                    {
                        continue;
                    }

                    if resumed_from_sleep {
                        watchdog_log(&format!(
                            "sleep gap of {}s detected, reloading",
                            gap.as_secs()
                        ));
                    } else {
                        watchdog_log("web layer stopped checking in, reloading");
                    }

                    // Reloading recreates the page. Hiding and showing the
                    // window first also forces WebView2 to reattach its
                    // compositor, which a bare reload does not always do after
                    // a resume. The show brings the window back exactly where
                    // it was; it is not a new window.
                    let _ = window.hide();
                    let _ = window.show();
                    let _ = window.reload();

                    last_reload = Some(std::time::Instant::now());
                    // Give the new page a moment to boot before the watchdog
                    // can decide it is stale again.
                    *heartbeat.lock().unwrap() = Some(std::time::Instant::now());
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
            commands::ui_heartbeat,
        ])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => {
            app.run(|_handle, _event| {});
        }
        Err(e) => eprintln!("启动失败: {e}"),
    }
}
