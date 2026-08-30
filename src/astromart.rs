use reqwest::Url;
use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use tracing::info;
use crate::backend::Backend;
use crate::posting::{Status, Posting};
use std::{error::Error};
use chrono::{NaiveDateTime};

#[derive(Debug)]
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
            .text()
            .collect::<String>()
            .trim()
            .to_string();

        let status = if value
            .select(&Selector::parse(".flex-table-col--title > span")?)
            .next()
            .is_some_and(|el| el.text().collect::<String>().trim().contains("SOLD")) {
                Status::Sold
            } else {
                Status::None
            };

        let post_title_el = value
            .select(&Selector::parse(".flex-table-col--title > a")?)
            .next()
            .ok_or("Cannot locate title")?;

        let title = post_title_el
            .text()
            .collect::<String>()
            .trim()
            .to_string();
        let url = "www.astromart.com".to_owned()
            + (post_title_el.attr("href").ok_or("Can't find post link")?);

        let sellerv = value
            .select(&Selector::parse(".flex-table-col--seller > a")?).next();

        eprintln!("{:?}", sellerv);
        let seller = value
            .select(&Selector::parse(".flex-table-col--seller")?)
            // .select(&Selector::parse(".flex-table-col--seller > a")?)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or("".to_string());
        eprintln!("{:?}", seller);

        let price = value
            .select(&Selector::parse(".flex-table-col--price")?)
            .next()
            .ok_or("Cannot locate price")?
            .text()
            .collect::<String>()
            .trim()
            .to_string()
            .strip_prefix("$")
            .ok_or("Cannot trim the $ from the price")?
            .parse::<f32>()?;

        let hits = value
            .select(&Selector::parse(".flex-table-col--hits")?)
            .next()
            .ok_or("Cannot locate hits")?
            .text()
            .collect::<String>()
            .trim()
            .to_string()
            .parse::<i32>()?;

        let posted = NaiveDateTime::parse_from_str(
            value
                .select(&Selector::parse(".flex-table-col--date")?)
                .next()
                .ok_or("Cannot locate hits")?
                .text()
                .collect::<String>()
                .trim(),
            "%m/%d/%Y %I:%M%p"
        )?;

        let post = Posting {post_type, title, seller, price, hits, posted, url, status};
        eprintln!("{:?}", post);
        Ok(post)
    }

    async fn get_postings(&self) -> Result<Vec<Posting>, Box<dyn std::error::Error>> {
        let response = reqwest::get(Url::parse(&self.page_url)?).await?.error_for_status()?;
        let html = Html::parse_document(&response.text().await?);

        let selector = Selector::parse(".classifieds > .flex-table-row.flex-table-row--body")?;
        let res = html
                .select(&selector)
                .filter_map(|el| {
                    let post = self.new_posting(el);
                    if let Err(ref err) = post {
                        eprintln!("{:?}", err)
                    }
                    post.ok()
                })
                .collect();

        info!("Postings: {:?}", res);
        Ok(res)
    }
}
