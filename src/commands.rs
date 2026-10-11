use crate::error::{AppError, AppResult, ErrSuperseded, code};
use crate::radio::Station;
use crate::store::FavoritesStore;
// AppState is defined in this file
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState {
    pub player: std::sync::Arc<crate::player::Player>,
    pub favorites: std::sync::Arc<crate::store::FavoritesStore>,
    pub last_station: std::sync::Arc<crate::store::LastStationStore>,
    pub client: std::sync::Arc<crate::radio::RadioClient>,
    pub browse_pool: std::sync::Arc<tokio::sync::Mutex<Vec<crate::radio::Station>>>,
    pub fav_pool: std::sync::Arc<tokio::sync::Mutex<Vec<crate::radio::Station>>>,
    pub search_seq: std::sync::Arc<tokio::sync::Mutex<u32>>,
}

use tauri::async_runtime::spawn_blocking;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StationView {
    pub key: String,
    pub uuid: String,
    pub name: String,
    pub country: String,
    pub country_code: String,
    pub language: String,
    pub codec: String,
    /// Whether the catalogue named a codec at all.
    ///
    /// Carried separately because `codec` holds the label to display, and the
    /// label for a station that named none is a sentence rather than data.
    /// Comparing against that sentence is what the UI used to do, which breaks
    /// the moment the wording changes or the backend speaks another language.
    pub codec_known: bool,
    pub bitrate: u32,
    pub votes: u32,
    /// Kept as a list: a tag can contain a comma, so a joined string could not
    /// be split back into the same tags.
    pub tag_list: Vec<String>,
    pub stream_url: String,
    pub homepage: String,
    pub favicon: String,
    pub display_name: String,
    pub meta: String,
    /// First tag, kept apart from `meta` so the UI can label it on its own.
    pub genre: String,
    pub favorite: bool,
}

impl StationView {
    fn from_station(station: &Station, favorites: &FavoritesStore) -> Self {
        let keys = favorites.keys();
        Self {
            key: station.key(),
            uuid: station.uuid.clone(),
            name: station.display_name(),
            country: station.country_name(),
            country_code: station.country_code.to_lowercase(),
            language: station.language.trim().to_string(),
            codec: station.codec_name(),
            codec_known: !station.codec.trim().is_empty(),
            bitrate: station.bitrate,
            votes: station.votes,
            tag_list: station.tags.clone(),
            stream_url: station.stream_url.clone(),
            homepage: station.homepage.clone(),
            favicon: station.favicon.clone(),
            display_name: station.display_name(),
            meta: station.meta(),
            genre: station.genre(),
            favorite: keys.contains_key(&station.key()),
        }
    }

    pub fn to_station(&self) -> Station {
        let tags = self.tag_list.clone();

        let mut name = self.name.trim().to_string();
        if name.is_empty() {
            name = self.display_name.trim().to_string();
        }

        let mut station = Station {
            uuid: self.uuid.clone(),
            name,
            stream_url: self.stream_url.clone(),
            backup_url: String::new(),
            homepage: self.homepage.clone(),
            favicon: self.favicon.clone(),
            country: self.country.clone(),
            country_code: self.country_code.clone(),
            state: String::new(),
            language: self.language.clone(),
            tags,
            codec: self.codec.clone(),
            bitrate: self.bitrate,
            votes: self.votes,
            click_count: 0,
            hls: false,
            last_check_ok: true,
        };

        if station.uuid.is_empty() {
            station.uuid = self.key.clone();
        }
        station
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub playing: bool,
    pub playing_key: Option<String>,
    pub paused: bool,
    pub name: String,
    pub title: String,
    pub backend: String,
    pub error: Option<AppError>,
    pub error_sticky: bool,
    pub volume: f32,
    pub station: Option<StationView>,
}

const SEARCH_LIMIT: u32 = 150;

#[tauri::command]
pub async fn top_stations(state: State<'_, AppState>) -> AppResult<Vec<StationView>> {
    let current_seq = {
        let mut seq = state.search_seq.lock().await;
        *seq += 1;
        *seq
    };

    let client = state.client.clone();
    let stations = spawn_blocking(move || client.top(SEARCH_LIMIT))
        .await
        ?
        ?;

    // A newer request has been issued. Its result is the one the user wants, so
    // this one is dropped -- but reporting it as an empty list would clear what
    // is on screen, so it is reported as a cancellation the caller ignores.
    if *state.search_seq.lock().await != current_seq {
        return Err(ErrSuperseded.into());
    }

    {
        let mut pool = state.browse_pool.lock().await;
        *pool = stations.clone();
    }

    let views = stations
        .iter()
        .map(|s| StationView::from_station(s, &state.favorites))
        .collect();
    Ok(views)
}

#[tauri::command]
pub async fn search_stations(
    term: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<StationView>> {
    let term = term.trim().to_string();

    let current_seq = {
        let mut seq = state.search_seq.lock().await;
        *seq += 1;
        *seq
    };

    let client = state.client.clone();
    let term2 = term.clone();
    let stations = spawn_blocking(move || {
        if term2.is_empty() {
            client.top(SEARCH_LIMIT)
        } else {
            client.search(&term2, SEARCH_LIMIT)
        }
    })
    .await
    ?
    ?;

    if *state.search_seq.lock().await != current_seq {
        return Err(ErrSuperseded.into());
    }

    {
        let mut pool = state.browse_pool.lock().await;
        *pool = stations.clone();
    }

    let views = stations
        .iter()
        .map(|s| StationView::from_station(s, &state.favorites))
        .collect();
    Ok(views)
}

#[tauri::command]
pub async fn get_favorites(
    filter: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<StationView>> {
    let stored = state.favorites.list();
    let needle = filter.as_deref().unwrap_or_default().trim().to_lowercase();

    let filtered: Vec<Station> = if needle.is_empty() {
        stored.clone()
    } else {
        stored
            .iter()
            .filter(|s| {
                format!(
                    "{} {} {} {}",
                    s.display_name().to_lowercase(),
                    s.country_name().to_lowercase(),
                    s.codec_name().to_lowercase(),
                    s.tags.join(" ").to_lowercase(),
                )
                .contains(&needle)
            })
            .cloned()
            .collect()
    };

    // Only the unfiltered list feeds the random pool: a filter narrows what is
    // on screen, and letting it narrow the pool too would make "random from
    // favourites" report the collection as empty while it was being filtered.
    if needle.is_empty() {
        let mut pool = state.fav_pool.lock().await;
        *pool = stored;
    }

    let views = filtered
        .iter()
        .map(|s| StationView::from_station(s, &state.favorites))
        .collect();
    Ok(views)
}

#[tauri::command]
pub async fn toggle_favorite(
    station: StationView,
    state: State<'_, AppState>,
) -> AppResult<bool> {
    let station_obj = station.to_station();
    let key = station_obj.key();

    let favorite = if state.favorites.has(&key) {
        state.favorites.remove(&key)?;
        false
    } else {
        state
            .favorites
            .add(station_obj.clone())
            ?;
        true
    };

    // Keep the random pool in step, or a station saved a moment ago cannot be
    // reached by the random button until the list happens to be reloaded.
    let mut pool = state.fav_pool.lock().await;
    pool.retain(|s| s.key() != key);
    if favorite {
        pool.push(station_obj);
    }

    Ok(favorite)
}

#[tauri::command]
pub async fn play_station(station: StationView, state: State<'_, AppState>) -> AppResult<()> {
    let station_obj = station.to_station();
    state
        .player
        .play(station_obj.clone())
        ?;
    let _ = state.last_station.save(station_obj);
    Ok(())
}

#[tauri::command]
pub async fn play_pause(state: State<'_, AppState>) -> AppResult<()> {
    state.player.play_pause().map_err(AppError::from)
}

#[tauri::command]
pub async fn stop_playback(state: State<'_, AppState>) -> AppResult<()> {
    state.player.stop();
    Ok(())
}

#[tauri::command]
pub async fn play_random(list: String, state: State<'_, AppState>) -> AppResult<()> {
    let pool = match list.as_str() {
        "favorites" => state.fav_pool.lock().await.clone(),
        "browse" | "" => state.browse_pool.lock().await.clone(),
        _ => return Err(AppError::with_detail(code::UNKNOWN_LIST, format!("未知列表 {:?}", list), list)),
    };

    if pool.is_empty() {
        return Err(AppError::new(code::EMPTY_LIST, "当前列表为空"));
    }

    let station = pool[fastrand::usize(..pool.len())].clone();
    state
        .player
        .play(station.clone())
        ?;
    let _ = state.last_station.save(station);
    Ok(())
}

#[tauri::command]
pub async fn set_volume(v: f32, state: State<'_, AppState>) -> AppResult<f32> {
    state.player.set_volume(v);
    Ok(state.player.volume())
}

#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> AppResult<PlayerView> {
    Ok(player_view(&state.player.state(), &state.favorites))
}

/// Builds the view the web layer renders.
///
/// The player reports a plain `PlayerState`, which carries no knowledge of what
/// is stored, so the favourite flag has to be looked up here. Both the poll and
/// the `player` event go through this, so the two cannot drift apart.
pub fn player_view(state: &crate::player::PlayerState, favorites: &FavoritesStore) -> PlayerView {
    let mut view = PlayerView {
        playing: state.playing,
        playing_key: state.playing_key.clone(),
        paused: state.paused,
        name: String::new(),
        title: state.title.clone(),
        backend: state.backend.clone(),
        error: state.error.clone(),
        error_sticky: false,
        volume: state.volume,
        station: None,
    };

    if let Some(ref station) = state.station {
        view.name = station.display_name();
        view.station = Some(StationView::from_station(station, favorites));
        if state.playing {
            view.playing_key = Some(station.key());
        }
    }

    // A format the app cannot decode is a property of the station, so the notice
    // stays up until another one is chosen; a dropped connection is worth only a
    // brief mention. Decided on the code rather than on the wording, which used
    // to make this silently stop matching the moment the copy was reworded or
    // translated.
    if let Some(ref err) = state.error {
        view.error_sticky = matches!(err.code.as_str(), code::UNSUPPORTED_FORMAT | code::FORMAT_NOT_SUPPORTED);
    }

    view
}

#[cfg(test)]
#[path = "commands_test.rs"]
mod commands_test;
