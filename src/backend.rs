use crate::posting::Posting;
use async_trait::async_trait;
use scraper::ElementRef;
use std::error::Error;
use std::fmt::Debug;

#[async_trait]
pub trait Backend: Debug {
    fn new_posting(&self, value: ElementRef) -> Result<Posting, Box<dyn Error>>;
    async fn get_postings(&self) -> Result<Vec<Posting>, Box<dyn std::error::Error>>;
}
