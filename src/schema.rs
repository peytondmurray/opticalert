use diesel::{
    RunQueryDsl, SqliteConnection, allow_tables_to_appear_in_same_query, joinable, result::Error,
    sql_query, table,
};

table! {
    postings {
        id -> Integer,
        post_type -> Text,
        title -> Text,
        seller -> Nullable<Text>,
        price -> Float,
        hits -> Integer,
        posted -> Timestamp,
        url -> Text,
        fetch_id -> Integer,
    }
}

table! {
    fetches {
        id -> Integer,
        date -> Timestamp
    }
}

joinable!(postings -> fetches(fetch_id));
allow_tables_to_appear_in_same_query!(postings, fetches);

pub fn bootstrap(connection: &mut SqliteConnection) -> Result<(), Error> {
    sql_query("CREATE TABLE IF NOT EXISTS postings (id INTEGER PRIMARY KEY AUTOINCREMENT, post_type TEXT, title TEXT, seller TEXT, price REAL, hits INTEGER, posted TEXT, url TEXT, fetch_id INTEGER, FOREIGN KEY(fetch_id) REFERENCES fetches(id));")
        .execute(connection)?;
    sql_query(
        "CREATE TABLE IF NOT EXISTS fetches (id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT);",
    )
    .execute(connection)?;

    Ok(())
}
