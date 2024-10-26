use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::provider::CoverProvider;

const MSG_PANIC_DEFAULT_CONFIG: &str =
    "default config should not panic while parsing values of Url";

// configuration struct
#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    // cover database connection
    pub cover_db_uri: Url,
    pub cover_db_path_pass: PathBuf,
    // port on which the cover API will listen for incoming connections
    pub listen_port: u16,
    // path where the cover files will be stored.
    pub path_covers: PathBuf,
    // time to wait before retrying getting the cover in seconds.
    pub wait_seconds_retry_retrieve_cover: u64,
    // product API connection, can be the same as the cover database.
    pub product_api_uri: Url,
    pub product_api_path_pass: PathBuf,
    // to update the cache when data is updated in db
    pub cache_api_uri: Url,
    pub cache_api_path_pass: PathBuf,
    // to return a uri of a task to track instead of waiting the operation.
    pub tasks_api_uri: Url,
    pub tasks_api_pass_path: PathBuf,
    pub providers: Vec<CoverProvider>,
    // domain name used for this instance of cover API. Used for cache API
    pub hostname: String,
}
/// Good example of a config
impl Default for Config {
    fn default() -> Self {
        Self {
            cover_db_uri: Url::parse("mysql://cover@localhost:3306/cover")
                .expect(MSG_PANIC_DEFAULT_CONFIG),
            cover_db_path_pass: PathBuf::from("admin/db/cover"),
            listen_port: 8000,
            path_covers: PathBuf::new(),
            wait_seconds_retry_retrieve_cover: 3600,
            product_api_uri: Url::parse("https://dolibarr.example.net")
                .expect(MSG_PANIC_DEFAULT_CONFIG),
            product_api_path_pass: PathBuf::from("admin/dolibarr/api_key"),
            cache_api_uri: Url::parse("http://cover_worker@localhost:8001")
                .expect(MSG_PANIC_DEFAULT_CONFIG),
            cache_api_path_pass: PathBuf::from("admin/cache_pai/api_key"),
            tasks_api_uri: Url::parse("https://cover_worker@tasks.example.net")
                .expect(MSG_PANIC_DEFAULT_CONFIG),
            tasks_api_pass_path: PathBuf::from("admin/tasks-tracker/token"),
            providers: vec![CoverProvider::OpenLibrary],
            hostname: "covers.example.net".to_string(),
        }
    }
}
