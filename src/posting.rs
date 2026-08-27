use chrono::{NaiveDateTime};

#[derive(Debug)]
pub enum Status {
    Sold,
    None
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
