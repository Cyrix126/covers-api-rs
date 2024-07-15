use std::path::Path;

use crate::cover::update_table_image;
use crate::image::write_cover;
use anyhow::Result;
use deadpool_diesel::mysql::Object;
use reqwest::Client;
use serde::{Deserialize, Serialize};
// #[derive(Clone, Deserialize, Serialize)]
#[derive(Clone, Deserialize, Serialize)]
#[repr(u8)]
pub enum CoverProvider {
    #[cfg(feature = "openlibrary")]
    OpenLibrary,
    Manual,
}

#[cfg(feature = "openlibrary")]
async fn openlibrary_cover(client: &Client, barcode: &str) -> Result<Vec<u8>> {
    let url = [
        "https://covers.openlibrary.org/b/isbn/",
        barcode,
        "-L.jpg?default=false",
    ]
    .concat();
    Ok(client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

pub async fn try_get_cover(
    conn: &Object,
    client: &Client,
    path_cover: &Path,
    providers: Vec<CoverProvider>,
    barcode: &str,
    product_id: u32,
) -> Result<()> {
    let mut provider = None;
    for cp in providers {
        match cp.method(client, barcode).await {
            Ok(cover) => {
                // write cover
                write_cover(&cover, product_id, path_cover)?;
                // success to true
                provider = Some(cp);
                // abort iteration
                break;
            }
            Err(_) => continue,
        }
    }
    update_table_image(product_id, conn, provider).await?;
    // result is ok even if no files has been changed. If using thiserror, result could be made more useful.
    Ok(())
}

impl CoverProvider {
    async fn method(&self, client: &Client, barcode: &str) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "openlibrary")]
            CoverProvider::OpenLibrary => openlibrary_cover(client, barcode).await,
            _ => {
                panic!("Manual should not be present in this method.\n This variant is only useful when using the manual upload of cover")
            }
        }
    }
}
