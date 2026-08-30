use std::path::PathBuf;
use std::str::FromStr;
use std::{error::Error, fs, path::Path};

use diesel::{
    Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection, dsl::insert_into,
};
use directories::ProjectDirs;
use futures::{StreamExt, stream};
use reqwest::Url;
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;

use crate::astromart::AstromartBackend;
use crate::backend::Backend;
use crate::config::ConfigFile;
use crate::posting::{Posting, PostingRow, Status};
use crate::schema::postings;

mod astromart;
mod backend;
mod config;
mod posting;
mod schema;

fn get_sqlite_db(db_path: &Path) -> Result<SqliteConnection, Box<dyn Error>> {
    SqliteConnection::establish(&db_path.to_string_lossy()).map_err(|_| "what".into())
}

fn sync_db(mut db: SqliteConnection, fetched_postings: &[Posting]) -> Vec<&Posting> {
    fetched_postings
        .iter()
        .filter(|posting| {
            if postings::table
                .filter(postings::url.eq(posting.url.to_owned()))
                .first::<PostingRow>(&mut db)
                .is_err()
            {
                if insert_into(postings::table)
                    .values((
                        postings::post_type.eq(&posting.post_type),
                        postings::title.eq(&posting.title),
                        postings::seller.eq(&posting.seller),
                        postings::price.eq(posting.price),
                        postings::hits.eq(posting.hits),
                        postings::posted.eq(posting.posted),
                        postings::url.eq(&posting.url),
                    ))
                    .execute(&mut db)
                    .is_ok()
                {
                    info!("Loading {} into database", &posting.url);
                } else {
                    warn!("Failed to insert posting into database: {}", &posting.url);
                }
                true
            } else {
                false
            }
        })
        .collect()
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
    let responses = stream::iter(postings)
        .map(|posting| {
            let url = server_url.clone();
            let subclient = client.clone();

            async move {
                if posting.status == Status::Sold {
                    return Ok(None);
                }

                Some(
                    subclient
                        .post(url)
                        .form(&[
                            ("title", &posting.title),
                            ("message", &posting.url),
                            ("priority", &"10".to_string()),
                        ])
                        .header("X-Gotify-Key", key)
                        .send()
                        .await?
                        .bytes()
                        .await,
                )
                .transpose()
            }
        })
        .buffer_unordered(8);

    responses
        .for_each(|r| async {
            match r {
                Ok(None) => info!("No notification sent, posting is sold"),
                Ok(_) => info!("Gotify acknowledgement received"),
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

fn get_config(config_dir: &Path) -> Result<ConfigFile, Box<dyn Error>> {
    let config_path = config_dir.join("config.toml");
    let config_file: ConfigFile =
        toml::from_str(&fs::read_to_string(&config_path).map_err(|_| {
            format!(
                "Couldn't read the config file at {}",
                config_path.to_string_lossy()
            )
        })?)
        .map_err(|_| {
            format!(
                "Can't parse the config file at {} as toml",
                config_path.to_string_lossy()
            )
        })?;
    Ok(config_file)
}

/// https://codingpackets.com/blog/rust-load-a-toml-file/
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let config_dir = get_or_create_config_dir()?;
    let config = get_config(&config_dir)?;

    let db = get_sqlite_db(Path::new("./opticalert.db"))?;

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
        stream::iter(backends)
            .map(|b| async move { b.get_postings().await.ok() })
            .buffer_unordered(2)
            // Can't use a .filter() here because we need whatever comes out to be an awaitable;
            // can't use a filter_map above because the thing that comes out is an awaitable (???)
            // even though this is part of the futures package (why??)
            .filter_map(|obj| async { obj })
            .collect::<Vec<Vec<Posting>>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<Posting>>()
    } else {
        return Ok(());
    };

    let new_posts = sync_db(db, &posts);

    println!("{:?}", new_posts);

    send_gotify(
        Url::from_str(&config.config.gotify_server)?,
        &config.config.gotify_key,
        &new_posts,
    )
    .await?;

    Ok(())
}
