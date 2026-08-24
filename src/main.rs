use std::{error::Error, path::{Path}};

use chrono::{DateTime, Local, NaiveDateTime};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection, dsl::insert_into, prelude::Insertable};
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};

use crate::schema::postings;

mod schema;

#[derive(Debug)]
struct Posting {
    post_type: String,
    title: String,
    seller: String,
    price: f32,
    hits: i32,
    posted: NaiveDateTime,
    url: String,
}

impl Posting {
    fn new(value: ElementRef) -> Result<Self, Box<dyn Error>> {
        let post_type = value
            .select(&Selector::parse(".flex-table-col--type")?)
            .next()
            .ok_or("Cannot locate type")?
            .inner_html();

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

        Ok(Posting {post_type, title, seller, price, hits, posted, url})

    }
}

fn get_sqlite_db(db_path: &Path) -> Result<SqliteConnection, Box<dyn Error>> {
    SqliteConnection::establish(&db_path.to_string_lossy()).map_err(|_| "what".into())
}

async fn get_postings(target: &str) -> Result<Vec<Posting>, Box<dyn std::error::Error>> {
    let response = reqwest::get(Url::parse(target)?).await?.error_for_status()?;
    let html = Html::parse_document(&response.text().await?);

    let selector = Selector::parse(".classifieds")?;
    html.select(&selector);

    Ok(
        html
            .select(&selector)
            .filter_map(|el| { Posting::new(el).ok() })
            .collect()
    )
}

async fn sync_db(db: SqliteConnection, new_postings: Vec<Posting>) -> Vec<Posting> {

    new_postings
        .into_iter()
        .filter_map(|posting| {
            postings::table.filter(postings::url.eq(&posting.url))
                .first(&mut db)
                .map_or_else(|obj| {
                    None
                }, |obj| {
                    insert_into(postings::table).values(
                        (
                            postings::post_type.eq(posting.post_type),
                            postings::title.eq(posting.title),
                            postings::seller.eq(posting.seller),
                            postings::price.eq(posting.price),
                            postings::hits.eq(posting.hits),
                            postings::posted.eq(posting.posted),
                            postings::url.eq(posting.url),
                        )
                    ).execute(&mut db);
                    Some(posting)
                })
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let db = get_sqlite_db(Path::new("./am.db"))?;
    let new_postings = sync_db(
        db,
        get_postings("https://www.astromart.com/classifieds/search?q=1100&category_id=10").await?,
    );

    Ok(())
}
