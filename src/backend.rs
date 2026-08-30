use std::fmt::Debug;
use std::error::Error;
use async_trait::async_trait;
use scraper::{ElementRef};
use crate::posting::Posting;

#[async_trait]
pub trait Backend: Debug {
    fn new_posting(&self, value: ElementRef) -> Result<Posting, Box<dyn Error>>;
    async fn get_postings(&self) -> Result<Vec<Posting>, Box<dyn std::error::Error>>;
}
