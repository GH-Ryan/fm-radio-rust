//! Tests for the conversion layer between the store, the player and the
//! frontend. `StationView` is the only shape the webview sees, so anything lost
//! or invented here is lost or invented for the user, and this boundary is
//! where that happens.

use super::*;
use crate::player::PlayerState;
use crate::store::FavoritesStore;
use std::path::PathBuf;

/// A store backed by a temp file, so the favourite lookup is a real one rather
/// than a stub.
fn store_for(name: &str) -> FavoritesStore {
    let path: PathBuf =
        std::env::temp_dir().join(format!("fm-cmd-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_file(&path);
    FavoritesStore::open(&path).expect("打开临时 store")
}

fn view() -> StationView {
    StationView {
        key: "uuid-1".into(),
        uuid: "uuid-1".into(),
        name: "Radio Example".into(),
        country: "法国".into(),
        country_code: "fr".into(),
        language: "French".into(),
        codec: "MP3".into(),
        bitrate: 128,
        votes: 7,
        tag_list: vec!["Jazz".into(), "Eclectic".into()],
        stream_url: "https://example.com/stream".into(),
        homepage: "https://example.com".into(),
        favicon: "https://example.com/logo.png".into(),
        display_name: "Radio Example".into(),
        meta: "法国 - Jazz".into(),
        genre: "Jazz".into(),
        favorite: false,
    }
}

/// A station sent back from the frontend must not be able to smuggle in fields
/// the view does not carry. backup_url in particular would be a second stream
/// for the player to open, chosen by whoever sent the request.
#[test]
fn a_station_from_the_frontend_carries_no_extra_authority() {
    let station = view().to_station();
    assert!(station.backup_url.is_empty(), "backup_url 被带进来了");
    assert!(station.state.is_empty(), "state 被带进来了");
    assert_eq!(station.click_count, 0, "click_count 被带进来了");
    assert!(!station.hls, "hls 被带进来了");
}

/// The name is what the player shows, and a station with only a display name
/// still has to be playable.
#[test]
fn an_empty_name_falls_back_to_the_display_name() {
    let mut v = view();
    v.name = "   ".into();
    assert_eq!(v.to_station().name, "Radio Example");

    v.display_name = "备用名".into();
    assert_eq!(v.to_station().name, "备用名");
}

/// A nameless view with nothing to fall back to must not end up playing under
/// an empty label.
#[test]
fn a_name_that_is_empty_everywhere_stays_empty_rather_than_being_invented() {
    let mut v = view();
    v.name = String::new();
    v.display_name = String::new();
    assert!(v.to_station().name.is_empty());
}

/// The key is what favourites are filed under, and it falls back to the stream
/// URL. Losing that would silently detach a station from its favourite entry.
#[test]
fn the_key_survives_the_round_trip() {
    let store = store_for("key");
    let station = view().to_station();
    let back = StationView::from_station(&station, &store);
    assert_eq!(back.key, "uuid-1");
    assert_eq!(back.stream_url, "https://example.com/stream");
}

/// A station with no uuid is keyed by its stream URL. Round-tripping it must
/// not change which station it is, or toggling a favourite would create a
/// second entry instead of removing the first.
#[test]
fn a_station_keyed_by_url_keeps_its_key() {
    let store = store_for("url-key");
    let mut v = view();
    v.uuid = String::new();
    v.key = "https://example.com/stream".into();
    let station = v.to_station();
    let back = StationView::from_station(&station, &store);
    assert_eq!(back.key, "https://example.com/stream");
    assert_eq!(back.uuid, "https://example.com/stream");
}

#[test]
fn the_favourite_flag_comes_from_the_store_and_not_from_the_request() {
    let store = store_for("fav");
    let station = view().to_station();

    let before = StationView::from_station(&station, &store);
    assert!(!before.favorite);

    store.add(station.clone()).expect("写入收藏");
    let after = StationView::from_station(&station, &store);
    assert!(after.favorite, "收藏后仍报未收藏");
}

/// The flag is read from the store on every conversion, so a view arriving with
/// favorite set is not evidence of anything. Checked against a station that is
/// genuinely not in the store.
#[test]
fn a_claimed_favourite_does_not_survive_the_round_trip() {
    let store = store_for("forged");
    let mut forged = view();
    forged.key = "not-in-store".into();
    forged.uuid = "not-in-store".into();
    forged.favorite = true;
    assert!(!StationView::from_station(&forged.to_station(), &store).favorite);
}

/// The country code reaches the frontend as a CSS class and a flag lookup, so
/// it is lowercased on the way out.
#[test]
fn the_country_code_is_lowercased() {
    let store = store_for("cc");
    let mut station = view().to_station();
    station.country_code = "FR".into();
    assert_eq!(
        StationView::from_station(&station, &store).country_code,
        "fr"
    );
}

/// Tags carry commas, so they travel as a list. A round trip that rejoined them
/// would split one tag into two and change what is shown under the name.
#[test]
fn tags_survive_the_round_trip_as_a_list() {
    let store = store_for("tags");
    let mut v = view();
    v.tag_list = vec!["Jazz, Fusion".into(), "Talk".into()];
    let back = StationView::from_station(&v.to_station(), &store);
    assert_eq!(back.tag_list, vec!["Jazz, Fusion", "Talk"]);
}

/// player_view is the single place the favourite flag is attached, and both the
/// poll and the event go through it. The key is only reported while playing, so
/// a paused station does not look like the one still being heard.
#[test]
fn the_playing_key_is_reported_only_while_playing() {
    let store = store_for("pkey");
    let station = view().to_station();

    let mut state = PlayerState {
        station: Some(station.clone()),
        volume: 0.5,
        ..Default::default()
    };

    let paused_view = player_view(&state, &store);
    assert!(!paused_view.playing);
    assert!(paused_view.playing_key.is_none(), "暂停时仍在报播放键");
    assert_eq!(paused_view.name, "Radio Example");

    state.playing = true;
    let playing_view = player_view(&state, &store);
    assert!(playing_view.playing);
    assert_eq!(playing_view.playing_key.as_deref(), Some("uuid-1"));

    let key = station.key();
    let station_view = playing_view.station.expect("播放时应带上电台");
    assert_eq!(station_view.key, key);
}

/// With nothing loaded the view still has to be a complete object: the frontend
/// reads these fields unconditionally, so a missing station must not take the
/// name or the key with it.
#[test]
fn an_empty_player_still_produces_a_complete_view() {
    let store = store_for("empty");
    let state = PlayerState {
        volume: 0.25,
        ..Default::default()
    };
    let v = player_view(&state, &store);
    assert!(!v.playing);
    assert!(!v.paused);
    assert!(v.playing_key.is_none());
    assert!(v.station.is_none());
    assert!(v.name.is_empty());
    assert_eq!(v.volume, 0.25);
}
