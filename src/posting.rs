use chrono::{NaiveDateTime};

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Sold,
    None
}

#[derive(Debug, Clone)]
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
