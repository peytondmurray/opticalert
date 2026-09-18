use config::Config;
use futures::future::join_all;
use std::path::PathBuf;
use std::str::FromStr;
use std::{error::Error, fs, path::Path};

use diesel::dsl::{insert_into, now};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection};
use directories::ProjectDirs;
use futures::{StreamExt, stream};
use reqwest::Url;
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;

use crate::astromart::AstromartBackend;
use crate::backend::Backend;
use crate::cloudynights::CloudyNightsBackend;
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

fn get_or_create_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let config_dir = ProjectDirs::from("", "", "opticalert")
        .ok_or("Can't find project directory")?
        .config_dir()
        .to_path_buf();

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

fn get_config(config_dir: &Path) -> Result<configuration::ConfigFile, Box<dyn Error>> {
    let config_path = config_dir.join("config.toml");

    let settings = Config::builder()
        .add_source(config::File::with_name(&config_path.to_string_lossy()))
        .add_source(config::Environment::with_prefix("OPTICALERT"))
        .build()?;

    Ok(settings.try_deserialize::<configuration::ConfigFile>()?)
}

/// https://codingpackets.com/blog/rust-load-a-toml-file/
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let config = get_config(&get_or_create_config_dir()?)?;
    let mut db = get_sqlite_db(Path::new("./opticalert.db"))?;

    // Read the config file and instantiate the necessary backends
    let maybe_backends: Option<Vec<Box<dyn Backend>>> = config.astromart.map(|obj| {
        obj.pages
            .iter()
            .map(|page| {
                Box::new(AstromartBackend {
                    page_url: page.to_string(),
                }) as _
            })
            .collect::<Vec<Box<dyn Backend>>>()
    });

    info!("Found backends: {:?}", maybe_backends);
    let posts = if let Some(backends) = maybe_backends {
        join_all(backends.iter().map(|b| b.get_postings()))
            .await
            .iter()
            .filter_map(|o| o.clone().ok())
            .flatten()
            .collect::<Vec<Posting>>()
    } else {
        return Ok(());
    };

    bootstrap(&mut db)?;
    let new_posts = sync_db(&mut db, &posts)?;

    send_gotify(
        Url::from_str(&config.config.gotify_server)?,
        &config.config.gotify_key,
        &new_posts,
    )
    .await?;

    Ok(())
}
