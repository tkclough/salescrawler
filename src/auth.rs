use base64::{engine::general_purpose, Engine};

pub fn make_basic_auth_header(username: &str, password: &str) -> String {
    let raw = format!("{username}:{password}");
    let encoded = general_purpose::STANDARD.encode(raw);
    format!("Basic {encoded}")
}