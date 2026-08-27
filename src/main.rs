use std::path::PathBuf;
use std::{error::Error, fs, path::Path};

use tracing::{warn, info, error, Level};
use tracing_subscriber::FmtSubscriber;
use chrono::{NaiveDateTime};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection, dsl::insert_into};
use reqwest::{Url};
use futures::{stream, StreamExt};
use directories::{ProjectDirs};

use crate::schema::postings;
use crate::posting::Posting;
use crate::config::ConfigFile;
use crate::backend::Backend;
use crate::astromart::AstromartBackend;

mod backend;
mod schema;
mod astromart;
mod posting;
mod config;

fn get_sqlite_db(db_path: &Path) -> Result<SqliteConnection, Box<dyn Error>> {
    SqliteConnection::establish(&db_path.to_string_lossy()).map_err(|_| "what".into())
}

fn sync_db(mut db: SqliteConnection, fetched_postings: &[Posting]) -> Vec<&Posting> {
    fetched_postings
        .iter()
        .filter(|posting| {
            if postings::table.filter(postings::url.eq(posting.url.to_owned())).first::<(i32, String, String, String, f32, i32, NaiveDateTime, String, i32)>(&mut db).is_err() {
                if insert_into(postings::table).values(
                    (
                        postings::post_type.eq(&posting.post_type),
                        postings::title.eq(&posting.title),
                        postings::seller.eq(&posting.seller),
                        postings::price.eq(posting.price),
                        postings::hits.eq(posting.hits),
                        postings::posted.eq(posting.posted),
                        postings::url.eq(&posting.url),
                    )
                ).execute(&mut db).is_ok() {
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
async fn send_gotify(server_url: Url, key: &str, postings: &[&Posting]) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let responses = stream::iter(postings)
        .map(|posting| {
            let url = server_url.clone();
            let subclient = client.clone();
            async move {
                subclient
                    .post(url)
                    .form(
                        &[
                            ("title", &posting.title),
                            ("message", &posting.url),
                            ("priority", &"10".to_string())
                        ]
                    )
                    .header("X-Gotify-Key", key)
                    .send()
                    .await?
                    .bytes()
                    .await
            }
        })
        .buffer_unordered(8);

    responses.for_each(|r| {
        async {
            match r {
                Ok(_) => info!("Gotify acknowledgement received"),
                Err(e) => error!("Encountered an error issuing notification: {}", e)
            }
        }
    }).await;
    Ok(())
}

fn get_or_create_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let config_dir = ProjectDirs::from("", "", "opticalert")
        .ok_or("Can't find project directory")?
        .config_dir().to_path_buf();

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

fn get_config(config_dir: &Path) -> Result<ConfigFile, Box<dyn Error>> {
    let config_path = config_dir.join("config.toml");
    let config_file: ConfigFile = toml::from_str(
        &fs::read_to_string(&config_path)
            .map_err(|_| {
                format!("Couldn't read the config file at {}", config_path.to_string_lossy())
            })?
    ).map_err(|_| {
        format!("Can't parse the config file at {} as toml", config_path.to_string_lossy())
    })?;
    return Ok(config_file)
}

/// https://codingpackets.com/blog/rust-load-a-toml-file/
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let config_dir = get_or_create_config_dir()?;
    let config = get_config(&config_dir)?;

    let db = get_sqlite_db(Path::new("./opticalert.db"))?;

    // Read the config file and instantiate the necessary backends
    let backends: Option<Vec<Box<dyn Backend>>> = config
        .astromart
        .map(|obj| {
            obj
                .pages
                .iter()
                .map(|page| {
                    Box::new(AstromartBackend { page_url: page.to_string() }) as _
                })
                .collect::<Vec<Box<dyn Backend>>>()
        });


    let postings: Vec<Posting> = if let Some(bes) = backends {
        bes
            .iter()
            .map(async |be| {
                be.get_postings().await?
            })
            .collect()
    } else {
        []
    };

    // let postings = backends
    //     .map(|bes| {
    //         bes.get_postings()
    //     });

    // let postings = get_postings("https://www.astromart.com/classifieds/search?q=1100&category_id=10").await?;
    // let new_postings = sync_db(db, &postings);
    // send_gotify(
    //     Url::from_str(&config_file.config.gotify_server)?,
    //     &config_file.config.gotify_am_key,
    //     &new_postings,
    // ).await?;

    Ok(())
}
