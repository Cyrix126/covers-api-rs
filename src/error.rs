use std::fmt::Display;

use axum::http::StatusCode;
use axum_thiserror::ErrorStatus;
use thiserror::Error;
use tracing::{error, warn};

/// thiserror Error struct, responding with different StatusCode depending on the error type
/// Those variants are meant for client facing, not for admin debugging.
/// todo, how to associate tracing messages for server side debugging ?
#[derive(Error, Debug, ErrorStatus)]
pub enum AppError {
    #[error("File does not exist")]
    #[status(axum::http::StatusCode::NOT_FOUND)]
    FileNotFound(#[from] std::io::Error),
    #[error("invalid value for HOST header")]
    #[status(StatusCode::BAD_REQUEST)]
    Host,
    #[error("Misconfigured cover API on server side")]
    #[status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)]
    Conf,
    #[error("Backend required for cover API failed")]
    #[status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)]
    Backend,
    #[error("Database connection issue")]
    #[status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)]
    Db(#[from] deadpool_diesel::PoolError),
}

impl AppError {
    // method to output verbose message about the errors intended for administrator.
    pub fn transmit_error(self, error_msg: &impl Display) -> Self {
        error!("{error_msg}");
        match self {
            Self::Conf => warn!("check configuration file"),
            Self::Backend => warn!("a backend service seems to be unjoinable"),
            Self::FileNotFound(_) => warn!("a file requested does not exist"),
            Self::Db(_) => warn!("a database was unjoinable"),
            Self::Host => warn!("invalid value from HOST header for task tracker API"),
        }
        self
    }
}
