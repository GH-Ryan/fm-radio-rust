//! Tests for the parts of the player that decide what happens before any audio
//! exists: the content-type dispatch, the ICY title unescaping, and the
//! conversions between the player, the store and what the frontend receives.
//!
//! These are pure functions over untrusted input, which is exactly the kind
//! that gets exercised only by hand. sniff in particular has to stay in step
//! with the symphonia features in Cargo.toml: a content type it recognises but
//! that no enabled demuxer can open looks like a working station and fails at
//! playback time instead.

use super::*;

// ---------------------------------------------------------------- sniff

/// The content types sniff has to keep answering for. Adding an arm to sniff
/// without a row here, or a row here without an arm, is a silent change to what
/// the app claims it can play.
#[test]
fn every_documented_content_type_is_recognised() {
    let expected = [
        ("audio/mpeg", "mp3"),
        ("audio/mp3", "mp3"),
        ("audio/mpeg3", "mp3"),
        ("audio/x-mpeg", "mp3"),
        ("audio/aac", "aac"),
        ("audio/aacp", "aac"),
        ("audio/x-aac", "aac"),
        ("application/ogg", "ogg"),
        ("audio/ogg", "ogg"),
        ("audio/vorbis", "ogg"),
        ("application/vnd.ogg", "ogg"),
        ("audio/opus", "opus"),
        ("audio/flac", "flac"),
        ("audio/x-flac", "flac"),
        ("audio/wav", "wav"),
        ("audio/x-wav", "wav"),
        ("audio/wave", "wav"),
        ("audio/vnd.wave", "wav"),
    ];
    for (ct, codec) in expected {
        let got = sniff(ct).unwrap_or_else(|| panic!("{ct} 不再被识别"));
        assert_eq!(got.1, codec, "{ct} 认成了 {codec}");
    }
}

/// Servers spell the same type many ways. Normalisation is what keeps one arm
/// from having to cover every spelling.
#[test]
fn sniff_normalises_what_servers_actually_send() {
    for spelling in [
        "audio/mpeg",
        "  audio/mpeg  ",
        "AUDIO/MPEG",
        "Audio/Mp3; charset=binary",
        "audio/mpeg;charset=utf-8",
    ] {
        let got = sniff(spelling);
        assert_eq!(got.map(|f| f.1), Some("mp3"), "{spelling:?} 没有归一到 mp3");
    }
}

/// Some servers send a generic type for whatever they are serving, and a radio
/// station sending `application/octet-stream` is almost always MP3. Guessing
/// here is deliberate, so the behaviour is pinned rather than accidental.
#[test]
fn a_generic_content_type_is_treated_as_mp3() {
    for ct in ["audio", "application/octet-stream", "application/x-ogg"] {
        assert_eq!(sniff(ct).map(|f| f.1), Some("mp3"), "{ct}");
    }
}

#[test]
fn an_unrecognised_content_type_yields_nothing() {
    for ct in [
        "",
        " ",
        "text/html",
        "application/json",
        "video/mp4",
        "audio/",
        "/mpeg",
    ] {
        assert_eq!(sniff(ct), None, "{ct:?} 不该被识别");
    }
}

// ---------------------------------------------------------------- icy_title

/// Escaped inside the metadata block rather than in Rust. `\\'` is the one that
/// means a literal apostrophe; `\'` is the same character as `'` in a Rust
/// literal and needs no translation.
#[test]
fn an_escaped_apostrophe_is_unwrapped() {
    assert_eq!(icy_title(b"StreamTitle='It\\'s a Test';"), "It's a Test");
}

#[test]
fn a_title_with_no_apostrophe_is_returned_as_is() {
    assert_eq!(icy_title(b"StreamTitle='Plain Title';"), "Plain Title");
    assert_eq!(icy_title(b"StreamTitle='Live Set'"), "Live Set");
}

#[test]
fn surrounding_quotes_and_padding_are_trimmed() {
    assert_eq!(icy_title(b"StreamTitle='  Spaced  ';"), "Spaced");
    assert_eq!(icy_title(b"  StreamTitle='X'  "), "X");
}

#[test]
fn a_block_without_a_title_yields_an_empty_string() {
    for block in [
        &b""[..],
        &b"StreamUrl='http://example.com';"[..],
        &b";"[..],
        // Value is too short to hold quotes.
        &b"StreamTitle='';"[..],
        &b"StreamTitle=''"[..],
    ] {
        assert_eq!(icy_title(block), "", "{block:?}");
    }
}

/// The invariant for every one of these is that they do not panic: the block
/// length comes off the wire, so it can be cut anywhere. Only the single-quote
/// truncation is asserted on the value, because that is the only quoting ICY
/// uses and therefore the only one the unquoting is written for.
#[test]
fn a_truncated_value_does_not_panic() {
    for block in [
        &b"StreamTitle='"[..],
        &b"StreamTitle=\""[..],
        &b"'"[..],
        &b"\""[..],
        &b"StreamTitle='unterminated"[..],
    ] {
        let _ = icy_title(block);
    }
    // Cut off right after the opening quote: there is no title at all here, and
    // a single surviving quote is not a title.
    assert_eq!(icy_title(b"StreamTitle='"), "");
    assert_eq!(icy_title(b"StreamTitle=''"), "");
}
