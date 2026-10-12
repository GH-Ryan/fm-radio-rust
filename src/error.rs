use serde::{Deserialize, Serialize};

/// Stable identifiers for every failure the backend can report.
///
/// The code is what the UI branches on and translates; the message is only a
/// fallback for a code the frontend does not know yet. Recognising a failure by
/// matching its wording would tie the two sides to the same string and to one
/// language, so the wording could never be edited or translated.
///
/// The values are part of the contract with the frontend and must not be
/// renamed without updating frontend/src/i18n/backendErrors.ts.
pub mod code {
    // Player
    pub const OUTPUT_UNAVAILABLE: &str = "output_unavailable";
    pub const CONNECTION_FAILED: &str = "connection_failed";
    pub const UNSUPPORTED_FORMAT: &str = "unsupported_format";
    pub const DECODE_ERROR: &str = "decode_error";
    pub const STREAM_ENDED: &str = "stream_ended";
    pub const CANCELLED: &str = "cancelled";
    pub const NOTHING_PLAYING: &str = "nothing_playing";
    pub const EMPTY_LIST: &str = "empty_list";
    pub const UNKNOWN_LIST: &str = "unknown_list";

    // Catalogue
    pub const NETWORK: &str = "network";
    pub const JSON: &str = "json";
    pub const ALL_MIRRORS_FAILED: &str = "all_mirrors_failed";
    pub const NO_MIRRORS: &str = "no_mirrors";
    pub const HTTP: &str = "http";
    pub const IO: &str = "io";
    pub const URL_PARSE: &str = "url_parse";

    // Storage
    pub const CONFIG_DIR_UNAVAILABLE: &str = "config_dir_unavailable";

    /// A request overtaken by a newer one. The caller treats this as "nothing
    /// to show" rather than as a failure, since the newer request is about to
    /// deliver the answer anyway.
    pub const SUPERSEDED: &str = "superseded";

    /// A station the app recognises but will not decode. Distinct from an
    /// unrecognised content type because the remedy is different: another
    /// station helps, and installing something does not.
    pub const FORMAT_NOT_SUPPORTED: &str = "format_not_supported";
}

/// A failure on its way to the UI: what went wrong, and what to say about it.
///
/// The message is kept in the app's source language. The UI decides whether to
/// show it or to substitute its own translation for the code, which is what
/// lets the same backend serve both languages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    /// Free-form detail, e.g. an HTTP status. Never shown as the headline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(code: &str, message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            detail: Some(detail.into()),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

/// Reported when a request was overtaken by a newer one.
pub struct ErrSuperseded;

impl std::fmt::Display for ErrSuperseded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("请求已被新的操作取代")
    }
}

impl From<ErrSuperseded> for AppError {
    fn from(_: ErrSuperseded) -> Self {
        AppError::new(code::SUPERSEDED, "请求已被新的操作取代")
    }
}

macro_rules! from_player_error {
    ($ty:ty) => {
        impl From<$ty> for AppError {
            fn from(e: $ty) -> Self {
                use $ty as P;
                let c = match e {
                    P::OutputUnavailable(_) => code::OUTPUT_UNAVAILABLE,
                    P::ConnectionFailed(_) => code::CONNECTION_FAILED,
                    P::UnsupportedFormat(_) => code::UNSUPPORTED_FORMAT,
                    P::DecodeError(_) => code::DECODE_ERROR,
                    P::StreamEnded => code::STREAM_ENDED,
                    P::Cancelled => code::CANCELLED,
                    P::NothingPlaying => code::NOTHING_PLAYING,
                    P::Io(_) | P::Request(_) => code::IO,
                };
                AppError::new(c, e.to_string())
            }
        }
    };
}

from_player_error!(crate::player::PlayerError);

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::new(code::IO, e.to_string())
    }
}

impl From<crate::radio::RadioError> for AppError {
    fn from(e: crate::radio::RadioError) -> Self {
        use crate::radio::RadioError as R;
        let c = match e {
            R::Network(_) => code::NETWORK,
            R::Json(_) => code::JSON,
            R::AllMirrorsFailed(_) => code::ALL_MIRRORS_FAILED,
            R::NoMirrors => code::NO_MIRRORS,
            R::Http(_) => code::HTTP,
            R::Io(_) => code::IO,
            R::UrlParse(_) => code::URL_PARSE,
        };
        AppError::new(c, e.to_string())
    }
}

impl From<crate::store::StoreError> for AppError {
    fn from(e: crate::store::StoreError) -> Self {
        use crate::store::StoreError as S;
        let c = match e {
            S::Io(_) => code::IO,
            S::Json(_) => code::JSON,
            S::ConfigDirUnavailable => code::CONFIG_DIR_UNAVAILABLE,
        };
        AppError::new(c, e.to_string())
    }
}

/// Shorthand for the `?` in command bodies.
pub type AppResult<T> = Result<T, AppError>;
