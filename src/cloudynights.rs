use crate::backend::{Backend, BackendError};
use crate::posting::{Posting, Status};
use async_trait::async_trait;
use futures::future::join_all;
use thirtyfour::extensions::query::ElementQueryable;
use thirtyfour::{self, By, DesiredCapabilities, WebDriver, WebElement};

#[derive(Debug)]
pub struct CloudyNightsBackend {
    pub page_url: String,
}

async fn get_posting(el: &WebElement) -> Option<Posting> {
    let a = el
        .query(By::Css("span.ipsContained > a"))
        .desc("post <a>")
        .first()
        .await
        .ok()?;

    let span = el
        .query(By::Css("span.ipsStream_price"))
        .desc("post price")
        .first()
        .await
        .ok()?;

    let seller = el
        .query(By::Css("p.ipsType_reset > a"))
        .desc("seller link")
        .first()
        .await
        .ok()?
        .text()
        .await
        .ok();

    let status = el
        .query(By::Css("span.ipsBadge_intermediary"))
        .desc("sold badge")
        .first()
        .await
        .ok()
        .map_or_else(|| Status::None, |_| Status::Sold);

    let posted = el
        .query(By::Css("time"))
        .desc("post date")
        .first()
        .await
        .ok()?
        .attr("datetime")
        .await
        .ok()??;

    Some(Posting {
        post_type: "wanted".to_string(),
        title: a.text().await.ok()?,
        seller,
        price: span
            .text()
            .await
            .ok()?
            .strip_prefix("$")?
            .parse::<f32>()
            .ok()?,
        hits: 0,
        posted: chrono::DateTime::parse_from_rfc3339(&posted)
            .ok()?
            .naive_local(),
        url: a.attr("href").await.ok()??,
        status,
    })
}

#[async_trait]
impl Backend for CloudyNightsBackend {
    async fn get_postings(&self) -> Result<Vec<Posting>, BackendError> {
        let caps = DesiredCapabilities::chrome();
        let driver = WebDriver::managed(caps).await?;

        driver.goto(&self.page_url).await?;

        // ipsStreamItem ipsStreamItem_contentBlock ipsStreamItem_expanded ipsAreaBackground_reset ipsPad
        let elements = driver
            .query(By::Css("li.ipsStreamItem.ipsStreamItem_contentBlock.ipsStreamItem_expanded.ipsAreaBackground_reset.ipsPad"))
            .desc("posts")
            .any()
            .await?;

        // heading: span.ipsContained
        let res = join_all(elements.iter().map(get_posting).collect::<Vec<_>>())
            .await
            .into_iter()
            .flatten() // Removes None elements because Option implements IntoIterator
            .collect::<Vec<Posting>>();

        Ok(res)
    }
}
