use std::str::FromStr;
use std::{error::Error, path::Path};

use diesel::dsl::{insert_into, now};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection};
use futures::{StreamExt, stream};
use reqwest::Url;
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;

use crate::configuration::{ConfigFile, ensure_config_exists, get_config_path};
use crate::posting::{Posting, PostingRow, Status};
use crate::schema::{bootstrap, fetches, postings};

mod astromart;
mod backend;
mod cloudynights;
mod configuration;
mod posting;
mod schema;

fn get_sqlite_db(db_path: &Path) -> Result<SqliteConnection, Box<dyn Error>> {
    SqliteConnection::establish(&db_path.to_string_lossy()).map_err(|_| "what".into())
}

fn sync_db<'a>(
    db: &mut SqliteConnection,
    fetched_postings: &'a [Posting],
) -> Result<Vec<&'a Posting>, Box<dyn Error>> {
    let id = insert_into(fetches::table)
        .values((fetches::date.eq(&now),))
        .get_result::<(i32, String)>(db)?
        .0;

    Ok(fetched_postings
        .iter()
        .filter(|posting| {
            if postings::table
                .filter(postings::url.eq(posting.url.to_owned()))
                .first::<PostingRow>(db)
                .is_err()
            {
                if let Err(err) = insert_into(postings::table)
                    .values((
                        postings::post_type.eq(&posting.post_type),
                        postings::title.eq(&posting.title),
                        postings::seller.eq(&posting.seller),
                        postings::price.eq(posting.price),
                        postings::hits.eq(posting.hits),
                        postings::posted.eq(posting.posted),
                        postings::url.eq(&posting.url),
                        postings::fetch_id.eq(id),
                    ))
                    .execute(db)
                {
                    warn!(
                        "Failed to insert posting into database: {}: {}",
                        &posting.url, err
                    );
                } else {
                    info!("Loading {} into database", &posting.url);
                }
                true
            } else {
                false
            }
        })
        .collect())
}

/// curl "https://push.example.de/message"
///     -H "X-Gotify-Key: <apptoken>"
///     -F "title=my title"
///     -F "message=my message"
///     -F "priority=5"
///
/// https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest/51047786#51047786
///
/// * `postings`:
async fn send_gotify(
    server_url: Url,
    key: &str,
    postings: &[&Posting],
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = server_url.join("message")?;
    let responses = stream::iter(postings)
        .map(|posting| async {
            if posting.status == Status::Sold {
                return Ok(None);
            }

            Some(
                client
                    .post(url.clone())
                    .form(&[
                        ("title", &posting.title),
                        ("message", &posting.url),
                        ("priority", &"10".to_string()),
                    ])
                    .header("X-Gotify-Key", key)
                    .send()
                    .await?
                    .text()
                    .await,
            )
            .transpose()
        })
        .buffer_unordered(8);

    responses
        .for_each(|r| async {
            match r {
                Ok(None) => info!("No notification sent, posting is sold"),
                Ok(Some(res)) => info!("Gotify acknowledgement received: {:?}", res),
                Err(e) => error!("Encountered an error issuing notification: {}", e),
            }
        })
        .await;
    Ok(())
}

/// https://codingpackets.com/blog/rust-load-a-toml-file/
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let path = get_config_path()?;
    ensure_config_exists(&path)?;

    let config = ConfigFile::from_path(&path)?;
    // let mut db = get_sqlite_db(Path::new("./opticalert.db"))?;

    println!("{:#?}", config);

    // bootstrap(&mut db)?;

    // let mut posts: Vec<Posting> = Vec::new();
    //
    // let backends = [&config.astromart, &config.cloudynights];
    //
    // for be in backends.into_iter().flatten() {
    //     posts.append(&mut be.get_postings().await.ok().unwrap_or(vec![]));
    // }
    //
    // let new_posts = sync_db(&mut db, &posts)?;
    //
    // send_gotify(
    //     Url::from_str(&config.config.gotify_server)?,
    //     &config.config.gotify_key,
    //     &new_posts,
    // )
    // .await?;

    Ok(())
}
