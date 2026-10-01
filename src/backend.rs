use crate::posting::Posting;
use crate::{astromart, cloudynights};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

type Headers = Option<HashMap<String, String>>;
type Pages = Vec<String>;

#[derive(Debug, Deserialize, Serialize)]
pub struct Backend {
    pages: Pages,
    #[serde(flatten)]
    headers: Headers,
}

#[async_trait::async_trait]
pub trait Site {
    async fn get_postings(
        &self,
        page: &str,
        headers: HashMap<String, String>,
    ) -> Result<Vec<Posting>, Box<dyn Error>>;
}

impl Backend {
    pub async fn load(&self, backend_type: &str) -> Result<Vec<Posting>, Box<dyn Error>> {
        let be: Box<dyn Site> = match backend_type {
            "astromart" => Box::new(astromart::Astromart{}),
            "cloudynights" => Box::new(cloudynights::CloudyNights{}),
            other => Err(format!("No backend available for site {other:?}"))?,
        };

        Ok(
            join_all(
                self.pages
                    .iter()
                    .map(|page| {
                        be.get_postings(
                            page,
                            self.headers.clone().unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .await
            .into_iter()
            .filter_map(|item| item.ok())
            .flatten()
            .collect::<Vec<Posting>>()
        )
    }
}
