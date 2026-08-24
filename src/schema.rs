use diesel::{table, joinable, allow_tables_to_appear_in_same_query};
use diesel::sql_types::{
    Integer, Text, Double
};

table! {
    postings {
        id -> Integer,
        post_type -> Text,
        title -> Text,
        seller -> Text,
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
