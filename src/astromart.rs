use crate::backend::Site;
use crate::posting::{Posting, Status};
use tracing::warn;
use chrono::NaiveDateTime;
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::error::Error;
use std::fs;

#[derive(Debug, Clone)]
struct PartialPosting {
    post_type: String,
    title: String,
    url: String,
    status: Status,
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

fn new_posting(partial: PartialPosting) -> Result<Posting, Box<dyn Error>> {
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

pub struct Astromart;

async fn _get_postings(
    page_url: &str,
    _headers: HashMap<String, String>,
) -> Result<Vec<Posting>, Box<dyn Error>> {
    // We scope the `Html` usage here because it is not Send, and thus cannot be safely held
    // onto across await points. Basically nothing provided by scraper is okay to be sent cross
    // thread (although we are only doing concurrent work here, not multithreaded...?)
    let partials = {
        let response = reqwest::get(Url::parse(page_url)?)
            .await?
            .error_for_status()?
            .text()
            .await?;

        println!("{:#?}", response);

        let html = Html::parse_document(&response);
        let selector = Selector::parse(".classifieds > .flex-table-row.flex-table-row--body")?;

        let h = html.html();
        println!("{h:?}");
        fs::write(
            "/home/pdmurray/dev/sandbox/beef.html",
            h,
        )?;

        html.select(&selector)
            .filter_map(|el| parse_table_row(el).ok())
            .collect::<Vec<PartialPosting>>()
    };
    let res = partials
        .iter()
        .filter_map(|p| new_posting(p.clone()).ok())
        .collect::<Vec<_>>();

    Ok(res)
}

#[async_trait::async_trait]
impl Site for Astromart {
    async fn get_postings(
        &self,
        page_url: &str,
        _headers: HashMap<String, String>,
    ) -> Result<Vec<Posting>, Box<dyn Error>> {
        _get_postings(page_url, _headers).await.inspect_err(|err| {
            warn!("Unable to get page: {page_url}. Reason: {err}");
        })
    }
}
