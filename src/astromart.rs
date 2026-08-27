use reqwest::Url;
use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use crate::backend::Backend;
use crate::posting::{Status, Posting};
use std::{error::Error};
use chrono::{NaiveDateTime};

pub struct AstromartBackend {
    pub page_url: String
}

#[async_trait]
impl Backend for AstromartBackend {
    fn new_posting(&self, value: ElementRef) -> Result<Posting, Box<dyn Error>> {
        let post_type = value
            .select(&Selector::parse(".flex-table-col--type")?)
            .next()
            .ok_or("Cannot locate type")?
            .inner_html();

        let status = if value
            .select(&Selector::parse(".flex-table-col--title > span")?)
            .next()
            .is_some_and(|el| el.inner_html().contains("SOLD")) {
                Status::Sold
            } else {
                Status::None
            };

        let post_title_el = value
            .select(&Selector::parse(".flex-table-col--title > a")?)
            .next()
            .ok_or("Cannot locate title")?;

        let title = post_title_el.inner_html();
        let url = "www.astromart.com".to_owned()
            + (post_title_el.attr("href").ok_or("Can't find post link")?);

        let seller = value
            .select(&Selector::parse(".flex-table-col--seller > a")?)
            .next()
            .ok_or("Cannot locate seller")?
            .inner_html();

        let price = value
            .select(&Selector::parse(".flex-table-col--price")?)
            .next()
            .ok_or("Cannot locate price")?
            .inner_html()
            .strip_prefix("$")
            .ok_or("Cannot trim the $ from the price")?
            .parse::<f32>()?;

        let hits = value
            .select(&Selector::parse(".flex-table-col--hits")?)
            .next()
            .ok_or("Cannot locate hits")?
            .inner_html()
            .parse::<i32>()?;

        let posted = NaiveDateTime::parse_from_str(
            &value
                .select(&Selector::parse(".flex-table-col--date")?)
                .next()
                .ok_or("Cannot locate hits")?
                .inner_html(),
            "%m/%d/%Y %I:%M%p"
        )?;

        Ok(Posting {post_type, title, seller, price, hits, posted, url, status})

    }

    async fn get_postings(&self) -> Result<Vec<Posting>, Box<dyn std::error::Error>> {
        let response = reqwest::get(Url::parse(&self.page_url)?).await?.error_for_status()?;
        let html = Html::parse_document(&response.text().await?);

        let selector = Selector::parse(".classifieds")?;
        html.select(&selector);

        Ok(
            html
                .select(&selector)
                .filter_map(|el| { self.new_posting(el).ok() })
                .collect()
        )
    }
}
