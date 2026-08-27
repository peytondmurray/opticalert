use serde::Deserialize;
use chrono::{NaiveDateTime};

#[derive(Debug)]
enum Status {
    Sold,
    None
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    config: Config
}

#[derive(Debug, Deserialize)]
pub struct Config {
    gotify_server: String,
    gotify_am_key: String,
}

#[derive(Debug)]
pub struct Posting {
    pub post_type: String,
    pub title: String,
    pub seller: String,
    pub price: f32,
    pub hits: i32,
    pub posted: NaiveDateTime,
    pub url: String,
    pub status: Status,
}
