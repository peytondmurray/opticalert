use std::collections::HashMap;
use std::fmt::Debug;
use futures::future::join_all;
use serde::Deserialize;
use crate::posting::{Posting};
use crate::{astromart, cloudynights};
use std::error::Error;

#[derive(Debug, Deserialize)]
pub enum Backend {
    Astromart{ pages: Vec<String>, headers: Option<HashMap<String, String>>},
    CloudyNights{ pages: Vec<String>, headers: Option<HashMap<String, String>>},
}

async fn load(
    pages: Vec<String>,
    headers: HashMap<String, String>,
    get_postings: impl AsyncFn(&str, HashMap<String, String>) -> Result<Vec<Posting>, Box<dyn Error>>,
) -> Result<Vec<Posting>, Box<dyn Error>> {
    Ok(
        join_all(
            pages
                .iter()
                .map(|page| get_postings(page, headers.clone()))
                .collect::<Vec<_>>()
        )
        .await
        .into_iter()
        .filter_map(|item| item.ok())
        .flatten()
        .collect::<Vec<Posting>>()
    )
}


impl Backend {
    async fn get_postings(&self) -> Result<Vec<Posting>, Box<dyn Error>> {
        match self {
            Backend::Astromart{pages, headers} => {
                load(
                    pages.to_vec(),
                    headers.as_ref().unwrap_or(&HashMap::default()).clone(),
                    astromart::get_postings,
                ).await
            },
            Backend::CloudyNights{pages, headers} => {
                load(
                    pages.to_vec(),
                    headers.as_ref().unwrap_or(&HashMap::default()).clone(),
                    cloudynights::get_postings,
                ).await
            },
        }
    }
}
