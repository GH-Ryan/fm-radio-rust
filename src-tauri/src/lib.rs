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

/// Seconds since the last keyboard or mouse input.
///
/// The watchdog uses this to tell a wedged renderer from a display that simply
/// turned off while the user was away: reloading at that moment runs against a
/// sleeping compositor, so the repair waits for the user to come back.
#[cfg(target_os = "windows")]
fn user_idle_seconds() -> u64 {
    #[repr(C)]
    struct LastInputInfo {
        cb_size: u32,
        dw_time: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(info: *mut LastInputInfo) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount() -> u32;
    }

    let mut info = LastInputInfo {
        cb_size: std::mem::size_of::<LastInputInfo>() as u32,
        dw_time: 0,
    };
    let ok = unsafe { GetLastInputInfo(&mut info) };
    if ok == 0 {
        return 0;
    }
    let now = unsafe { GetTickCount() };
    (now.wrapping_sub(info.dw_time) / 1000) as u64
}

#[cfg(not(target_os = "windows"))]
fn user_idle_seconds() -> u64 {
    0
}

/// Keeps the display (and the machine) awake while audio is playing.
///
/// The black window is triggered by the display turning off: Chromium then
/// freezes the page and does not repaint when the screen comes back. Holding an
/// execution-state request while the window is visible and playing removes that
/// trigger. It is released as soon as playback stops or the window is
/// minimised, so the screen is not pinned on for a radio left in the background.
#[cfg(target_os = "windows")]
fn set_display_awake(awake: bool) {
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetThreadExecutionState(flags: u32) -> u32;
    }

    let flags = if awake {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    unsafe {
        SetThreadExecutionState(flags);
    }
}

#[cfg(not(target_os = "windows"))]
fn set_display_awake(_awake: bool) {}

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

            // WebView2 can stop painting after the machine wakes from sleep,
            // after the display turns off, or after its compositor decides the
            // window is occluded: the native window keeps answering the OS, but
            // the page is gone and only the window background is left, which
            // reads as a black window. Three signals catch that here:
            //
            // - A five-second tick that wakes up far later than it asked means
            //   the machine was suspended.
            // - The web layer checks in every few seconds; check-ins that stop
            //   catch a wedged renderer.
            // - Input that resumes after the user was away is the moment to
            //   repair a page that froze while the display was off. Reloading
            //   while the display is still off runs against a sleeping
            //   compositor and does not stick, so a stale page is left alone
            //   until the user comes back.
            let heartbeat = app.state::<AppState>().ui_heartbeat.clone();
            let player_for_wake = app.state::<AppState>().player.clone();
            let watchdog = app.handle().clone();
            std::thread::spawn(move || {
                watchdog_log("watchdog started");
                let mut last_reload: Option<std::time::Instant> = None;
                let mut last_tick = std::time::Instant::now();
                let mut waiting_for_user = false;
                let mut display_awake = false;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let now = std::time::Instant::now();
                    let gap = now.duration_since(last_tick);
                    last_tick = now;

                    let idle = user_idle_seconds();
                    let away = idle >= 20;
                    let resumed_from_sleep = gap > std::time::Duration::from_secs(30);
                    let heartbeat_stale = heartbeat
                        .lock()
                        .unwrap()
                        .is_some_and(|t| t.elapsed() > std::time::Duration::from_secs(40));

                    // Hold the display on only while the window is visible and
                    // audio is actually running. Releasing it when minimised
                    // keeps a radio left in the background from pinning the
                    // screen on.
                    let playing = {
                        let state = player_for_wake.state();
                        state.playing && !state.paused
                    };
                    let visible = watchdog
                        .get_webview_window("main")
                        .map(|window| !window.is_minimized().unwrap_or(false))
                        .unwrap_or(false);
                    let want_awake = playing && visible;
                    if want_awake != display_awake {
                        set_display_awake(want_awake);
                        display_awake = want_awake;
                        watchdog_log(if want_awake {
                            "display kept awake while playing"
                        } else {
                            "display sleep allowed"
                        });
                    }

                    if !resumed_from_sleep {
                        if heartbeat_stale && away {
                            if !waiting_for_user {
                                watchdog_log(&format!(
                                    "web layer stale while user is away (idle {idle}s); waiting for return"
                                ));
                                waiting_for_user = true;
                            }
                            continue;
                        }
                        if waiting_for_user {
                            waiting_for_user = false;
                            watchdog_log("user is back; checking the page");
                            // A renderer that was only throttled may resume on
                            // its own within a moment, which is better than a
                            // reload that throws the interface away.
                            std::thread::sleep(std::time::Duration::from_millis(800));
                            let fresh = heartbeat
                                .lock()
                                .unwrap()
                                .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(10));
                            if fresh {
                                continue;
                            }
                        } else if !heartbeat_stale {
                            continue;
                        }
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
                        watchdog_log(&format!(
                            "web layer stopped checking in (idle {idle}s); reloading"
                        ));
                    }

                    // Reattach the compositor before reloading. Hiding the
                    // window is deliberately not done: a hidden WebView2 can
                    // come back without a surface, which leaves exactly the
                    // black window this is meant to repair. A one-pixel resize
                    // forces a new frame without moving the window.
                    let _ = window.show();
                    let _ = window.set_focus();
                    if let Ok(size) = window.inner_size() {
                        let _ = window.set_size(tauri::PhysicalSize::new(size.width + 1, size.height));
                        std::thread::sleep(std::time::Duration::from_millis(120));
                        let _ = window.set_size(tauri::PhysicalSize::new(size.width, size.height));
                    }
                    let reloaded = window.reload();
                    watchdog_log(&format!("reload issued (result: {reloaded:?})"));

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
