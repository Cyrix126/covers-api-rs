use anyhow::Result;
use get_pass::get_password;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{error::AppError, provider::CoverProvider};

// configuration struct
#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    // cover database connection
    pub cover_db_uri: ConfigConnection,
    // port on which the cover API will listen for incoming connections
    pub listen_port: u16,
    // path where the cover files will be stored.
    pub path_covers: PathBuf,
    // time to wait before retrying getting the cover in seconds.
    pub wait_seconds_retry_retrieve_cover: u64,
    // product API connection, can be the same as the cover database.
    pub product_api_uri: ConfigConnection,
    pub product_api_type: ProductApiType,
    // to update the cache when data is updated in db
    pub cache_api_uri: ConfigConnection,
    pub cache_api_type: CacheApiType,
    // to return a uri of a task to track instead of waiting the operation.
    pub tasks_api_uri: ConfigConnection,
    pub tasks_api_type: TaskTrackerApiType,
    pub providers: Vec<CoverProvider>,
    // domain name used for this instance of cover API. Used for cache API
    pub hostname: String,
}
/// Good example of a config
impl Default for Config {
    fn default() -> Self {
        Self {
            cover_db_uri: ConfigConnection {
                base_uri: "mysql".to_string(),
                user: Some("cover".to_string()),
                password: Some(PathBuf::from("admin/db/cover")),
                address: "localhost".to_string(),
                port: 3306,
                path: "cover".to_string(),
            },
            listen_port: 8000,
            path_covers: PathBuf::from(""),
            wait_seconds_retry_retrieve_cover: 3600,
            product_api_uri: ConfigConnection {
                base_uri: "https".to_string(),
                user: None,
                password: None,
                address: "dolibarr.example.net".to_string(),
                port: 443,
                path: "".to_string(),
            },
            product_api_type: ProductApiType::Dolibarr(PathBuf::from("admin/dolibarr/api_key")),
            cache_api_uri: ConfigConnection {
                base_uri: "http".to_string(),
                user: Some("cover_worker".to_string()),
                password: Some(PathBuf::from("admin/ldap/cover_worker")),
                address: "localhost".to_string(),
                port: 8001,
                path: "".to_string(),
            },
            cache_api_type: CacheApiType::Mnemosyne,
            tasks_api_uri: ConfigConnection {
                base_uri: "https".to_string(),
                user: Some("cover_worker".to_string()),
                password: Some(PathBuf::from("admin/ldap/cover_worker")),
                address: "tasks.example.net".to_string(),
                port: 443,
                path: "".to_string(),
            },
            providers: vec![CoverProvider::OpenLibrary],
            tasks_api_type: TaskTrackerApiType::TaskTrackerRs(PathBuf::from(
                "admin/tasks-trackers/token",
            )),
            hostname: "covers.example.net".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigConnection {
    pub base_uri: String,
    pub user: Option<String>,
    // the password is a path to a pass file.
    pub password: Option<PathBuf>,
    pub address: String,
    pub port: u16,
    // path is the name of the database or the http path.
    pub path: String,
}

impl ConfigConnection {
    pub fn to_url(&self) -> Result<Url, AppError> {
        let identification = if let Some(user) = &self.user {
            if let Some(path_pass) = &self.password {
                [
                    user,
                    ":",
                    get_password(path_pass)
                        .map_err(|e| AppError::Conf.transmit_error(&e))?
                        .as_str(),
                    "@",
                ]
                .concat()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        Url::parse(
            &[
                &self.base_uri,
                "://",
                &identification,
                &self.address,
                ":",
                &self.port.to_string(),
                "/",
                &self.path,
            ]
            .concat(),
        )
        .map_err(|e| AppError::Conf.transmit_error(&e))
    }
}
#[derive(Clone, Deserialize, Serialize)]
pub enum ProductApiType {
    // the path is the pass file for the token
    Dolibarr(PathBuf),
}
impl Default for ProductApiType {
    fn default() -> Self {
        Self::Dolibarr(PathBuf::new())
    }
}
#[derive(Clone, Default, Deserialize, Serialize)]
pub enum CacheApiType {
    #[default]
    Mnemosyne,
}
#[derive(Clone, Deserialize, Serialize)]
pub enum TaskTrackerApiType {
    // the path is the pass file for the token
    TaskTrackerRs(PathBuf),
}
impl Default for TaskTrackerApiType {
    fn default() -> Self {
        Self::TaskTrackerRs(PathBuf::new())
    }
}
