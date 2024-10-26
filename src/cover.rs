use crate::schema::covers::{dsl::covers, id, provider};
use std::path::Path;
use std::time::Duration;

use crate::db::Cover;
use crate::error::AppError;
use crate::provider::{try_get_cover, CoverProvider};
use crate::schema::{self};
use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDateTime, Utc};
use deadpool_diesel::mysql::{Object, Pool};
use derive_more::Display;
use diesel::{
    dsl::{exists, select},
    prelude::*,
};
use reqwest::Client;
/// size of covers
use strum_macros::EnumIter;
use tokio::sync::mpsc::Sender;
#[derive(Display, EnumIter)]
pub enum CoverSize {
    #[display = "L"]
    Large,
    #[display = "M"]
    Medium,
    #[display = "S"]
    Small,
}
/// retrieve a cover for a product if conditions are met.
/// product must exist on product API,
/// product do not already have a cover
/// last try is past enough
/// Send update on the progress
/// Use the Product API given
/// Interact with write access to the Cover DB.
/// Right now, retrieve cover can retrieve only cover by barcode.
pub async fn retrieve_cover(
    product_id: u32,
    pool: &Pool,
    client: &doli_client_api_rs::Client,
    path_cover: &Path,
    wait_retry: u64,
    // using channel to be task tracker agnostic.
    sender_task_progress: Sender<u8>,
) -> Result<()> {
    let conn = pool.get().await?;
    get_status_must_get_image(&conn, product_id, wait_retry).await?;
    let barcode = get_barcode(client, product_id).await?;
    // progress update, conditions to get cover are met
    sender_task_progress.send(50).await?;
    // get it
    let client_provider = Client::new();
    let providers = vec![
        #[cfg(feature = "openlibrary")]
        CoverProvider::OpenLibrary,
    ];
    try_get_cover(
        &conn,
        &client_provider,
        path_cover,
        providers,
        &barcode,
        product_id,
    )
    .await?;
    Ok(())
}
async fn get_barcode(client: &doli_client_api_rs::Client, product_id: u32) -> Result<String> {
    client.get_barcode_from_id(product_id).await?.context("this product does not have barcode. Only products with barcode are supported in this version.")
}
/// verify with the cover API DB if conditions are met to retrieve the cover.
/// In case the id exist in the table, it will check if the id already has an image or if the delay for retrying is expired.
/// Return an error in case the product should not get retrieved.
async fn get_status_must_get_image(conn: &Object, product_id: u32, wait_retry: u64) -> Result<()> {
    // does the Cover DB posses a row with this id ?
    if id_exist(conn, product_id).await? {
        if cover_exist(conn, product_id).await? {
            // if yes, error cover already exist
            return Err(anyhow!("cover already exist for this product"));
        }
        // if no, did the last try expired
        if last_try(conn, product_id).await? + Duration::from_secs(wait_retry)
            <= Utc::now().naive_utc()
        {
            return Err(anyhow!(
                "wait time before trying to get the cover for this product did not expired"
            ));
        }
    }
    Ok(())
}

async fn id_exist(conn: &Object, product_id: u32) -> Result<bool> {
    Ok(conn
        .interact(move |conn| select(exists(covers.filter(id.eq(product_id)))).get_result(conn))
        .await
        .map_err(|e| anyhow!(e.to_string()))??)
}
async fn cover_exist(conn: &Object, product_id: u32) -> Result<bool> {
    Ok(conn
        .interact(move |conn| {
            select(exists(
                covers.filter(id.eq(product_id).and(provider.is_not_null())),
            ))
            .get_result(conn)
        })
        .await
        .map_err(|e| anyhow!(e.to_string()))??)
}
pub async fn all_id(conn: &Object) -> Result<Vec<u32>> {
    Ok(conn
        .interact(move |conn| covers.select(id).load(conn))
        .await
        .map_err(|e| anyhow!(e.to_string()))??)
}
pub async fn all_id_missing_retrievable(conn: &Object, wait_try: u64) -> Result<Vec<u32>> {
    let mut covers_id: Vec<Cover> = conn
        .interact(move |conn| covers.filter(provider.is_null()).load::<Cover>(conn))
        .await
        .map_err(|e| anyhow!(e.to_string()))??;
    covers_id.retain(|c| c.last_try + Duration::from_secs(wait_try) <= Utc::now().naive_utc());
    Ok(covers_id.iter().map(|c| c.id).collect())
}
async fn last_try(conn: &Object, product_id: u32) -> Result<NaiveDateTime> {
    Ok(conn
        .interact(move |conn| {
            covers
                .select(schema::covers::last_try)
                .find(product_id)
                .first(conn)
        })
        .await
        .map_err(|e| anyhow!(e.to_string()))??)
}

// does the Cover DB posses a row with this id ?
// if yes, does the cover already exist ?
// if yes, error cover already exist
// if no, did the last try expired
// if yes, ok
// if no, error
// if not, it is ok to retrieve the cover.
pub async fn update_table_image(
    product_id: u32,
    conn: &Object,
    name_cp: Option<CoverProvider>,
) -> Result<(), AppError> {
    use crate::schema::covers::dsl::*;
    use diesel::prelude::*;
    let now = Utc::now().naive_utc();
    let provider_code = name_cp.map(|p| p as u8);
    let record = Cover {
        id: product_id,
        last_try: now,
        provider: provider_code,
    };
    conn.interact(move |conn| diesel::replace_into(covers).values(&record).execute(conn))
        .await
        .map_err(|_| AppError::Backend)?
        .map_err(|_| AppError::Backend)?;
    Ok(())
}
