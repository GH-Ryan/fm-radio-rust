use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use url::Url;

const DEFAULT_MIRROR: &str = "https://de1.api.radio-browser.info";
const MIRROR_HOST: &str = "all.api.radio-browser.info";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15 wails.io/605.1.15";
const MAX_FALLBACKS: usize = 4;
const FALLBACK_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_CATALOGUE_RESPONSE_BYTES: u64 = 2 << 20;
const MAX_SERVER_RESPONSE_BYTES: u64 = 1 << 20;

#[derive(Error, Debug)]
pub enum RadioError {
    #[error("网络请求失败: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("所有镜像均不可用: {0}")]
    AllMirrorsFailed(String),
    #[error("未找到可用镜像")]
    NoMirrors,
    #[error("HTTP 错误: {0}")]
    Http(u16),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("URL 解析失败: {0}")]
    UrlParse(#[from] url::ParseError),
}

#[derive(Debug, Clone, Serialize)]
pub struct Station {
    // Every field defaults: the catalogue omits fields on some records, and one
    // absent field must not cost the whole response.
    #[serde(rename = "stationuuid", default)]
    pub uuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "url_resolved", default)]
    pub stream_url: String,
    #[serde(rename = "url", default)]
    pub backup_url: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub favicon: String,
    #[serde(default)]
    pub country: String,
    #[serde(rename = "countrycode", default)]
    pub country_code: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub language: String,
    #[serde(default, with = "string_list_serde")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub codec: String,
    #[serde(default)]
    pub bitrate: u32,
    #[serde(default)]
    pub votes: u32,
    #[serde(rename = "clickcount", default)]
    pub click_count: u32,
    #[serde(default, with = "bool_serde")]
    pub hls: bool,
    #[serde(rename = "lastcheckok", default, with = "bool_serde")]
    pub last_check_ok: bool,

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

mod string_list_serde {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.join(","))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringListVisitor;
        impl<'de> serde::de::Visitor<'de> for StringListVisitor {
            type Value = Vec<String>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("string list")
            }
            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut list = Vec::new();
                let mut seq = seq;
                while let Some(s) = seq.next_element()? {
                    list.push(s);
                }
                Ok(list)
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let parts: Vec<String> = v
                    .split(&[',', ';'][..])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                Ok(parts)
            }

            // The API sends a bare null for a station with no tags.
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Vec::new())
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Vec::new())
            }
        }
        deserializer.deserialize_any(StringListVisitor)
    }
}

impl<'de> Deserialize<'de> for Station {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "stationuuid", default)]
            uuid: String,
            #[serde(default)]
            name: String,
            /// The API renamed stationname to name, and records written before
            /// that still carry the old spelling. Folding it in here keeps the
            /// fallback out of the type entirely.
            #[serde(rename = "stationname", default)]
            stationname: String,
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
            #[serde(default, with = "string_list_serde")]
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

        let w = Wire::deserialize(d)?;
        let name = if w.name.trim().is_empty() { w.stationname } else { w.name };
        Ok(Station {
            uuid: w.uuid,
            name,
            stream_url: w.stream_url,
            backup_url: w.backup_url,
            homepage: w.homepage,
            favicon: w.favicon,
            country: w.country,
            country_code: w.country_code,
            state: w.state,
            language: w.language,
            tags: w.tags,
            codec: w.codec,
            bitrate: w.bitrate,
            votes: w.votes,
            click_count: w.click_count,
            hls: w.hls,
            last_check_ok: w.last_check_ok,
        })
    }
}

impl Station {
    pub fn key(&self) -> String {
        if !self.uuid.is_empty() {
            self.uuid.clone()
        } else {
            self.stream_url.clone()
        }
    }

    pub fn display_name(&self) -> String {
        let name = self.name.trim();
        if name.is_empty() {
            self.stream_url.clone()
        } else {
            name.to_string()
        }
    }

    pub fn codec_name(&self) -> String {
        let codec = self.codec.trim().to_uppercase();
        match codec.as_str() {
            "" => "未知格式".to_string(),
            "AAC+" | "AAC" => "AAC".to_string(),
            _ => codec,
        }
    }

    pub fn country_name(&self) -> String {
        self.country.trim().to_string()
    }

    pub fn genre(&self) -> String {
        for tag in &self.tags {
            let tag = tag.trim();
            if !tag.is_empty() {
                return tag.to_string();
            }
        }
        String::new()
    }

    pub fn meta(&self) -> String {
        let mut parts = Vec::new();
        let country = self.country_name();
        if !country.is_empty() {
            parts.push(country);
        }
        let language = self.language.trim();
        if !language.is_empty() {
            parts.push(language.to_string());
        }
        let genre = self.genre();
        if !genre.is_empty() {
            parts.push(genre);
        }
        if self.bitrate > 0 {
            parts.push(format!("{} kbps", self.bitrate));
        }
        parts.push(self.codec_name());
        parts.join(" · ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Server {
    name: String,
    url: String,
}

impl Server {
    fn base(&self) -> String {
        let url = self.url.trim().trim_end_matches('/');
        if !url.is_empty() {
            return url.to_string();
        }
        let name = self.name.trim().trim_end_matches(|c| c == '.' || c == '/');
        if name.is_empty() {
            return String::new();
        }
        if name.contains("://") {
            name.to_string()
        } else {
            format!("https://{}", name)
        }
    }
}

#[derive(Debug, Clone)]
struct Query {
    limit: u32,
    order: String,
    reverse: bool,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 100,
            order: "votes".to_string(),
            reverse: true,
        }
    }
}

#[derive(Debug)]
pub struct RadioClient {
    client: reqwest::blocking::Client,
    mirrors: Arc<Mutex<Vec<String>>>,
    cached_mirrors: Arc<Mutex<Vec<String>>>,
    last_fetch: Arc<Mutex<Option<Instant>>>,
}

impl RadioClient {
    pub fn new() -> Result<Self, RadioError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self {
            client,
            mirrors: Arc::new(Mutex::new(vec![DEFAULT_MIRROR.to_string()])),
            cached_mirrors: Arc::new(Mutex::new(Vec::new())),
            last_fetch: Arc::new(Mutex::new(None)),
        })
    }

    fn primary(&self) -> String {
        self.mirrors.lock().unwrap().first().cloned().unwrap_or_else(|| DEFAULT_MIRROR.to_string())
    }

    fn known_mirrors(&self) -> Vec<String> {
        self.mirrors.lock().unwrap().clone()
    }

    fn promote(&self, mirror: &str) {
        let mut mirrors = self.mirrors.lock().unwrap();
        mirrors.retain(|m| m != mirror);
        mirrors.insert(0, mirror.to_string());
    }

    fn fallbacks(&self) -> Vec<String> {
        let fresh = {
            let cached = self.cached_mirrors.lock().unwrap();
            let last = self.last_fetch.lock().unwrap();
            !cached.is_empty() && last.map_or(false, |t| t.elapsed() < FALLBACK_TTL)
        };
        if fresh {
            return self.cached_mirrors.lock().unwrap().clone();
        }

        let found = self.pick_fallbacks(self.discover());
        if found.is_empty() {
            return Vec::new();
        }
        *self.cached_mirrors.lock().unwrap() = found.clone();
        *self.last_fetch.lock().unwrap() = Some(Instant::now());
        found
    }

    fn pick_fallbacks(&self, servers: Vec<Server>) -> Vec<String> {
        let known: std::collections::HashSet<_> = self.known_mirrors().into_iter().collect();
        let mut found = Vec::new();
        for entry in servers {
            let mirror = entry.base();
            if mirror.is_empty() || known.contains(&mirror) {
                continue;
            }
            found.push(mirror);
            if found.len() == MAX_FALLBACKS {
                break;
            }
        }
        found
    }

    fn discover(&self) -> Vec<Server> {
        let mut out = Vec::new();
        for host in std::iter::once(MIRROR_HOST.to_string()).chain(self.known_mirrors()) {
            if let Ok(servers) = self.fetch_servers(&host) {
                out = servers;
                break;
            }
        }
        out.extend(self.dns_mirrors());
        out
    }

    fn fetch_servers(&self, mirror: &str) -> Result<Vec<Server>, RadioError> {
        let url = format!("{}/json/servers", mirror.trim_end_matches('/'));
        let resp = self.client.get(&url).send()?;
        if !resp.status().is_success() {
            return Err(RadioError::Http(resp.status().as_u16()));
        }
        let bytes = resp.bytes()?;
        if bytes.len() as u64 > MAX_SERVER_RESPONSE_BYTES {
            return Err(RadioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "响应过大",
            )));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn dns_mirrors(&self) -> Vec<Server> {
        use std::net::ToSocketAddrs;
        let addrs = match (MIRROR_HOST, 443).to_socket_addrs() {
            Ok(a) => a.collect::<Vec<_>>(),
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for addr in addrs {
            if let Ok(names) = addr.ip().to_string().parse::<std::net::IpAddr>() {
                // Skip reverse DNS for now to avoid std::net::lookup_addr issues
                out.push(Server {
                    name: names.to_string(),
                    url: String::new(),
                });
            }
        }
        out
    }

    fn stations(&self, path: &str, params: &mut HashMap<String, String>, query: Query) -> Result<Vec<Station>, RadioError> {
        let mut q = query;
        if q.limit == 0 {
            q.limit = 100;
        }
        if q.order.is_empty() {
            q.order = "votes".to_string();
        }
        params.insert("limit".to_string(), q.limit.to_string());
        params.insert("order".to_string(), q.order);
        params.insert("reverse".to_string(), q.reverse.to_string());
        params.insert("hidebroken".to_string(), "true".to_string());

        let primary = self.primary();
        // The first failure is the one worth reporting: it names what actually
        // went wrong (a timeout, a DNS miss, an HTTP status) instead of the
        // useless "all mirrors failed" that every fallback repeats.
        let first_err = match self.fetch(&primary, path, params) {
            Ok(result) => return Ok(clean(result)),
            Err(e) => e.to_string(),
        };
        let mut tried = 1;

        for mirror in self.fallbacks() {
            if mirror == primary {
                continue;
            }
            tried += 1;
            match self.fetch(&mirror, path, params) {
                Ok(result) => {
                    self.promote(&mirror);
                    return Ok(clean(result));
                }
                Err(_) => continue,
            }
        }

        Err(RadioError::AllMirrorsFailed(format!(
            "{}（已尝试 {} 个镜像）",
            first_err, tried
        )))
    }

    fn fetch(&self, mirror: &str, path: &str, params: &HashMap<String, String>) -> Result<Vec<Station>, RadioError> {
        let mut url = Url::parse(&format!("{}{}", mirror.trim_end_matches('/'), path))?;
        url.query_pairs_mut().extend_pairs(params.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        let resp = self.client.get(url).send()?;
        if !resp.status().is_success() {
            return Err(RadioError::Http(resp.status().as_u16()));
        }
        let bytes = resp.bytes()?;
        if bytes.len() as u64 > MAX_CATALOGUE_RESPONSE_BYTES {
            return Err(RadioError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "响应过大",
            )));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn search(&self, name: &str, limit: u32) -> Result<Vec<Station>, RadioError> {
        let mut params = HashMap::new();
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            params.insert("name".to_string(), trimmed.to_string());
        }
        self.stations("/json/stations/search", &mut params, Query {
            limit,
            order: "clickcount".to_string(),
            reverse: true,
        })
    }

    pub fn top(&self, limit: u32) -> Result<Vec<Station>, RadioError> {
        let limit = if limit == 0 { 100 } else { limit };
        let mut params = HashMap::new();
        self.stations(&format!("/json/stations/topclick/{}", limit), &mut params, Query::default())
    }
}

fn clean(stations: Vec<Station>) -> Vec<Station> {
    let mut seen = std::collections::HashSet::new();
    stations
        .into_iter()
        .filter(|s| {
            !s.stream_url.trim().is_empty() && !s.name.trim().is_empty() && seen.insert(s.key())
        })
        .collect()
}

impl Default for RadioClient {
    fn default() -> Self {
        Self::new().expect("Failed to create RadioClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn station(name: &str, url: &str) -> Station {
        Station {
            uuid: format!("u-{name}"),
            name: name.to_string(),
            stream_url: url.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn key_falls_back_to_the_stream_url_without_a_uuid() {
        let mut s = station("A", "http://a");
        s.uuid = String::new();
        assert_eq!(s.key(), "http://a");
        s.uuid = "u-A".into();
        assert_eq!(s.key(), "u-A");
    }

    #[test]
    fn display_name_falls_back_to_the_url_when_unnamed() {
        let mut s = station("", "http://only-url");
        assert_eq!(s.display_name(), "http://only-url");
        s.name = "  ".into();
        assert_eq!(s.display_name(), "http://only-url");
        s.name = " Real ".into();
        assert_eq!(s.display_name(), "Real");
    }

    #[test]
    fn codec_names_are_normalised() {
        let mut s = station("A", "http://a");
        s.codec = "aac+".into();
        assert_eq!(s.codec_name(), "AAC");
        s.codec = String::new();
        assert_eq!(s.codec_name(), "未知格式");
        s.codec = " mp3 ".into();
        assert_eq!(s.codec_name(), "MP3");
    }

    #[test]
    fn meta_joins_the_fields_it_has_and_omits_the_rest() {
        let mut s = station("A", "http://a");
        s.country = " China ".into();
        s.language = "chinese".into();
        s.tags = vec!["music".into(), "pop".into()];
        s.bitrate = 128;
        s.codec = "MP3".into();
        assert_eq!(s.meta(), "China · chinese · music · 128 kbps · MP3");

        // A bitrate of zero is missing data, not a station playing at 0 kbps.
        s.bitrate = 0;
        assert_eq!(s.meta(), "China · chinese · music · MP3");
    }

    #[test]
    fn clean_drops_unplayable_and_duplicate_records() {
        let kept = clean(vec![
            station("A", "http://a"),
            station("A", "http://a"),   // same key
            station("B", "http://b"),
            station("", "http://c"),    // no name
            station("D", "  "),         // no stream
        ]);
        let names: Vec<_> = kept.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["A", "B"]);
    }

    #[test]
    fn a_station_reads_from_either_name_spelling() {
        // The API renamed stationname to name, so old records still use it.
        let old = r#"{"stationuuid":"x","stationname":"Old","url_resolved":"http://o"}"#;
        let s: Station = serde_json::from_str(old).unwrap();
        assert_eq!(s.name, "Old");

        let new = r#"{"stationuuid":"x","name":"New","url_resolved":"http://n"}"#;
        let s: Station = serde_json::from_str(new).unwrap();
        assert_eq!(s.name, "New");
    }

    #[test]
    fn booleanish_fields_accept_every_shape_the_api_uses() {
        for (raw, want) in [("true", true), ("false", false), ("1", true), ("0", false), (r#""""#, false)] {
            let json = format!(r#"{{"hls":{raw}}}"#);
            let s: Station = serde_json::from_str(&json).unwrap();
            assert_eq!(s.hls, want, "hls={raw}");
        }
        // and as real booleans and numbers
        for (raw, want) in [("true", true), ("false", false), ("1", true), ("0", false)] {
            let s: Station = serde_json::from_str(&format!(r#"{{"hls":{raw}}}"#)).unwrap();
            assert_eq!(s.hls, want);
        }
    }

    #[test]
    fn tags_read_from_an_array_or_a_string() {
        let a: Station = serde_json::from_str(r#"{"tags":["pop","music"]}"#).unwrap();
        assert_eq!(a.tags, vec!["pop", "music"]);
        let b: Station = serde_json::from_str(r#"{"tags":"pop, music; jazz"}"#).unwrap();
        assert_eq!(b.tags, vec!["pop", "music", "jazz"]);
        let c: Station = serde_json::from_str(r#"{"tags":null}"#).unwrap();
        assert!(c.tags.is_empty());
    }
}
