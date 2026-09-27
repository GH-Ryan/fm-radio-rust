use crate::radio::Station;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default::Default;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("配置目录不可用")]
    ConfigDirUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredStation {
    #[serde(rename = "stationuuid", default)]
    uuid: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "url_resolved", default)]
    stream_url: String,
    #[serde(rename = "url", default)]
    backup_url: String,
    #[serde(default)]
    homepage: String,
    #[serde(default)]
    favicon: String,
    #[serde(default)]
    country: String,
    #[serde(rename = "countrycode", default)]
    country_code: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    codec: String,
    #[serde(default)]
    bitrate: u32,
    #[serde(default)]
    votes: u32,
    #[serde(rename = "clickcount", default)]
    click_count: u32,
    #[serde(default, with = "bool_serde")]
    hls: bool,
    #[serde(rename = "lastcheckok", default, with = "bool_serde")]
    last_check_ok: bool,
}

mod bool_serde {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(*value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoolVisitor;
        impl<'de> serde::de::Visitor<'de> for BoolVisitor {
            type Value = bool;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean value")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v != 0)
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let text = v.to_lowercase();
                match text.as_str() {
                    "" | "false" | "0" => Ok(false),
                    "true" | "1" => Ok(true),
                    _ => {
                        if let Ok(n) = v.parse::<f64>() {
                            Ok(n != 0.0)
                        } else {
                            Err(E::custom(format!("无法解析布尔值: {}", v)))
                        }
                    }
                }
            }
        }
        deserializer.deserialize_any(BoolVisitor)
    }
}

impl From<Station> for StoredStation {
    fn from(s: Station) -> Self {
        Self {
            uuid: s.uuid,
            name: s.name,
            stream_url: s.stream_url,
            backup_url: s.backup_url,
            homepage: s.homepage,
            favicon: s.favicon,
            country: s.country,
            country_code: s.country_code,
            state: s.state,
            language: s.language,
            tags: s.tags,
            codec: s.codec,
            bitrate: s.bitrate,
            votes: s.votes,
            click_count: s.click_count,
            hls: s.hls,
            last_check_ok: s.last_check_ok,
        }
    }
}

impl From<StoredStation> for Station {
    fn from(s: StoredStation) -> Self {
        Station {
            uuid: s.uuid,
            name: s.name,
            stream_url: s.stream_url,
            backup_url: s.backup_url,
            homepage: s.homepage,
            favicon: s.favicon,
            country: s.country,
            country_code: s.country_code,
            state: s.state,
            language: s.language,
            tags: s.tags,
            codec: s.codec,
            bitrate: s.bitrate,
            votes: s.votes,
            click_count: s.click_count,
            hls: s.hls,
            last_check_ok: s.last_check_ok,
        }
    }
}

#[derive(Debug)]
pub struct FavoritesStore {
    path: PathBuf,
    items: Arc<Mutex<Vec<Station>>>,
}

impl FavoritesStore {
    pub fn new() -> Result<Self, StoreError> {
        let config_dir = dirs::config_dir().ok_or(StoreError::ConfigDirUnavailable)?;
        Self::open(&config_dir.join("fmradio").join("favorites.json"))
    }

    /// Opens the store held in `path`, reading it if it is already there.
    pub fn open(path: &PathBuf) -> Result<Self, StoreError> {
        let items = Self::load_from(path)?;
        Ok(Self {
            path: path.clone(),
            items: Arc::new(Mutex::new(items)),
        })
    }

    fn load_from(path: &PathBuf) -> Result<Vec<Station>, StoreError> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(path)?;
        // A file that cannot be read is reported rather than swallowed, but one
        // that cannot be parsed is treated as empty: a hand edit or a truncated
        // write should not cost the user their collection, nor stop the app from
        // starting. It is kept aside so it can still be looked at.
        let stored: Vec<StoredStation> = match serde_json::from_str(&data) {
            Ok(list) => list,
            Err(_) => {
                let _ = fs::rename(path, path.with_extension("json.corrupt"));
                return Ok(Vec::new());
            }
        };
        Ok(stored.into_iter().map(Station::from).collect())
    }

    fn save(&self) -> Result<(), StoreError> {
        let items = self.items.lock().unwrap();
        let stored: Vec<StoredStation> = items.iter().cloned().map(StoredStation::from).collect();
        let data = serde_json::to_string_pretty(&stored)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, data)?;
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<Station> {
        let mut items = self.items.lock().unwrap().clone();
        items.sort_by_key(|a| a.display_name());
        items
    }

    pub fn keys(&self) -> HashMap<String, bool> {
        let items = self.items.lock().unwrap();
        let mut keys = HashMap::new();
        for station in items.iter() {
            keys.insert(station.key(), true);
        }
        keys
    }

    pub fn has(&self, key: &str) -> bool {
        let items = self.items.lock().unwrap();
        items.iter().any(|s| s.key() == key)
    }

    pub fn add(&self, station: Station) -> Result<bool, StoreError> {
        let mut items = self.items.lock().unwrap();
        let mut added = true;
        for existing in items.iter_mut() {
            if existing.key() == station.key() {
                *existing = station.clone();
                added = false;
                break;
            }
        }
        if added {
            items.push(station);
        }
        drop(items);
        self.save()?;
        Ok(added)
    }

    pub fn remove(&self, key: &str) -> Result<bool, StoreError> {
        let mut items = self.items.lock().unwrap();
        let len_before = items.len();
        items.retain(|s| s.key() != key);
        let removed = items.len() != len_before;
        drop(items);
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn clear(&self) -> Result<(), StoreError> {
        let mut items = self.items.lock().unwrap();
        items.clear();
        drop(items);
        self.save()
    }
}

#[derive(Debug)]
pub struct LastStationStore {
    path: PathBuf,
    station: Arc<Mutex<Option<Station>>>,
}

impl LastStationStore {
    pub fn new() -> Result<Self, StoreError> {
        let config_dir = dirs::config_dir().ok_or(StoreError::ConfigDirUnavailable)?;
        Self::open(&config_dir.join("fmradio").join("last.json"))
    }

    /// Opens the store held in `path`, reading it if it is already there.
    pub fn open(path: &PathBuf) -> Result<Self, StoreError> {
        let station = Self::load_last(path)?;
        Ok(Self {
            path: path.clone(),
            station: Arc::new(Mutex::new(station)),
        })
    }

    fn load_last(path: &PathBuf) -> Result<Option<Station>, StoreError> {
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(path)?;
        // Unreadable, or holding a station with nothing to play, counts as no
        // memory at all, which is the same state as a first run.
        let Ok(stored) = serde_json::from_str::<StoredStation>(&data) else {
            return Ok(None);
        };
        if stored.stream_url.is_empty() {
            return Ok(None);
        }
        Ok(Some(Station::from(stored)))
    }

    pub fn get(&self) -> Result<(Station, bool), StoreError> {
        let station = self.station.lock().unwrap().clone();
        let has_station = station.is_some();
        let station = station.unwrap_or_default();
        Ok((station, has_station))
    }

    pub fn save(&self, station: Station) -> Result<(), StoreError> {
        let stored = StoredStation::from(station.clone());
        let data = serde_json::to_string(&stored)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, data)?;
        fs::rename(tmp_path, &self.path)?;
        *self.station.lock().unwrap() = Some(station);
        Ok(())
    }
}

impl Default for Station {
    fn default() -> Self {
        Station {
            uuid: String::new(),
            name: String::new(),
            stream_url: String::new(),
            backup_url: String::new(),
            homepage: String::new(),
            favicon: String::new(),
            country: String::new(),
            country_code: String::new(),
            state: String::new(),
            language: String::new(),
            tags: Vec::new(),
            codec: String::new(),
            bitrate: 0,
            votes: 0,
            click_count: 0,
            hls: false,
            last_check_ok: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Each case gets its own directory so cases cannot see each other's files,
    /// and removes it so a run leaves nothing in the temp directory.
    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("fm-store-test-{}-{}", std::process::id(), name));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Fixture(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn station(name: &str, url: &str) -> Station {
        Station {
            uuid: format!("uuid-{name}"),
            name: name.to_string(),
            stream_url: url.to_string(),
            tags: vec!["pop".into(), "music".into()],
            codec: "MP3".into(),
            bitrate: 128,
            ..Default::default()
        }
    }

    #[test]
    fn add_remove_round_trips_through_disk() {
        let fx = Fixture::new("roundtrip");
        let path = fx.path("favorites.json");
        let store = FavoritesStore::open(&path).unwrap();

        assert!(store.add(station("A", "http://a")).unwrap());
        assert!(store.add(station("B", "http://b")).unwrap());
        assert_eq!(store.list().len(), 2);

        // A fresh store over the same file must see what was written, which is
        // what happens on the next launch.
        let reopened = FavoritesStore::open(&path).unwrap();
        assert_eq!(
            reopened.list().len(),
            2,
            "favourites did not survive a reload"
        );

        assert!(reopened.remove("uuid-A").unwrap());
        assert_eq!(reopened.list().len(), 1);
        assert!(
            !reopened.remove("uuid-A").unwrap(),
            "removing twice reported a change"
        );
    }

    #[test]
    fn adding_the_same_station_twice_does_not_duplicate_it() {
        let fx = Fixture::new("dupe");
        let path = fx.path("favorites.json");
        let store = FavoritesStore::open(&path).unwrap();

        assert!(store.add(station("A", "http://a")).unwrap());
        assert!(
            !store.add(station("A", "http://a-new")).unwrap(),
            "second add claimed a new entry"
        );
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn an_unreadable_file_reads_as_empty_rather_than_failing() {
        let fx = Fixture::new("broken");
        let path = fx.path("favorites.json");
        std::fs::write(&path, "{not json").unwrap();
        let store = FavoritesStore::open(&path).unwrap();
        assert!(store.list().is_empty(), "a corrupt file blocked start-up");
    }

    #[test]
    fn last_station_is_kept_and_restored() {
        let fx = Fixture::new("last");
        let path = fx.path("last.json");
        let store = LastStationStore::open(&path).unwrap();
        assert!(!store.get().unwrap().1);

        store.save(station("Z", "http://z")).unwrap();
        let reloaded = LastStationStore::open(&path).unwrap();
        let (got, ok) = reloaded.get().unwrap();
        assert!(ok && got.name == "Z");
    }
}
