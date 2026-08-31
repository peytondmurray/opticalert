use crate::backend::{Backend, BackendError};
use crate::posting::{Posting, Status};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use futures::future::join_all;
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};
use std::error::Error;
use tracing::info;

#[derive(Debug, Clone)]
struct PartialPosting {
    post_type: String,
    title: String,
    url: String,
    status: Status,
}

#[derive(Debug)]
pub struct AstromartBackend {
    pub page_url: String,
}

fn parse_table_row(element: ElementRef) -> Result<PartialPosting, Box<dyn Error>> {
    let post_type = element
        .select(&Selector::parse(".flex-table-col--type")?)
        .next()
        .ok_or("Cannot locate type")?
        .text()
        .collect::<String>()
        .trim()
        .to_string();

    let status = if element
        .select(&Selector::parse(".flex-table-col--title > span")?)
        .next()
        .is_some_and(|el| el.text().collect::<String>().trim().contains("SOLD"))
    {
        Status::Sold
    } else {
        Status::None
    };

    let post_title_el = element
        .select(&Selector::parse(".flex-table-col--title > a")?)
        .next()
        .ok_or("Cannot locate title")?;

    let title = post_title_el.text().collect::<String>().trim().to_string();
    let url = "www.astromart.com".to_owned()
        + (post_title_el.attr("href").ok_or("Can't find post link")?);

    Ok(PartialPosting {
        post_type,
        status,
        title,
        url,
    })
}

async fn new_posting(partial: PartialPosting) -> Result<Posting, BackendError> {
    let post = Posting {
        post_type: partial.post_type,
        title: partial.title,
        status: partial.status,
        url: partial.url,
        seller: Some("Foo Bar".to_string()),
        posted: NaiveDateTime::parse_from_str("04/05/1989 05:00PM", "%m/%d/%Y %I:%M%p")?,
        price: 35.00,
        hits: 1000,
    };

    Ok(post)
}

#[async_trait]
impl Backend for AstromartBackend {
    async fn get_postings(&self) -> Result<Vec<Posting>, BackendError> {
        let response = reqwest::get(Url::parse(&self.page_url)?)
            .await?
            .error_for_status()?;

        let partials = {
            let html = Html::parse_document(&response.text().await?);
            // You can't send any scraper-defined object across threads, so instead we synchronously
            // parse the classifieds URL; then we'll make async requests later on to fill in the
            // details.
            let selector = Selector::parse(".classifieds > .flex-table-row.flex-table-row--body")?;

            html.select(&selector)
                .filter_map(|el| parse_table_row(el).ok())
                .collect::<Vec<PartialPosting>>()
        };
        let res = join_all(
            partials
                .iter()
                .map(|p| new_posting(p.clone()))
                .collect::<Vec<_>>(),
        )
        .await
        .iter()
        .filter_map(|item| item.clone().ok())
        .collect();

        info!("Postings: {:?}", res);
        Ok(res)
    }
}
