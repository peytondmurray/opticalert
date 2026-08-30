use crate::posting::Posting;
use async_trait::async_trait;
use std::fmt::Debug;

pub struct BackendError {
    message: String,
}

impl<T> From<T> for BackendError
where
    T: ToString,
{
    fn from(value: T) -> Self {
        BackendError {
            message: value.to_string(),
        }
    }
}

#[async_trait]
pub trait Backend: Debug {
    async fn get_postings(&self) -> Result<Vec<Posting>, BackendError>;
}
