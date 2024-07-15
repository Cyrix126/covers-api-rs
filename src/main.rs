use api::public::{get_cover, get_default_cover};
use api::worker::{
    add_manual_cover, delete_cover, get_missing_covers, retrieve_cover_handle,
    retrieve_missing_covers,
};
use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use axum::routing::put;
use axum::Router;
use config::CacheApiType;
use config::ProductApiType;
use config::TaskTrackerApiType;
use deadpool_diesel::mysql::Manager;
use deadpool_diesel::mysql::Pool;
use doli_client_api_rs::client_doli;
use doli_client_api_rs::Client as ClientDoli;
use error::AppError;
use get_pass::get_password;
use reqwest::Client;
use reqwest::Url;
use std::error::Error;
use tracing::info;

use anyhow::Result;
use config::Config;
use db::run_migrations;
mod api;
mod config;
/// cover module contains everything related to the task created by the API interacting with the DB and product API
mod cover;
mod db;
/// Error from handler
mod error;
mod image;
/// method to get cover from provider
mod provider;
mod schema;
/// different type of client for each different type of tasks tracker API supported
#[derive(Clone)]
enum ClientTask {
    TasksTracker(Client),
}

/// different type of client for each different type of tasks product API supported
#[derive(Clone)]
enum ClientProduct {
    Dolibarr(ClientDoli),
}
/// different type of client for each different type of tasks product API supported
#[derive(Clone)]
enum ClientCache {
    CachingProxy(Client),
}

fn api_task_type_to_client(api_type_task: &TaskTrackerApiType) -> ClientTask {
    match api_type_task {
        TaskTrackerApiType::TaskTrackerRs(_) => ClientTask::TasksTracker(Client::new()),
    }
}
fn api_cache_type_to_client(api_type_cache: &CacheApiType) -> ClientCache {
    match api_type_cache {
        CacheApiType::Mnemosyne => ClientCache::CachingProxy(Client::new()),
    }
}
fn api_product_type_to_client(
    api_type_product: &ProductApiType,
    url: Url,
) -> Result<ClientProduct> {
    match api_type_product {
        ProductApiType::Dolibarr(path_token_api) => Ok(ClientProduct::Dolibarr(client_doli(
            &get_password(path_token_api).map_err(|_| AppError::Conf)?,
            url,
        ))),
    }
}

#[derive(Clone)]
struct AppState {
    config: Config,
    // connection to the cover api database
    conn_db_cover: Pool,
    // client to request external API
    client_task: ClientTask,
    client_product: ClientProduct,
    client_cache: ClientCache,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    // config file with database url
    info!("loading config file");
    let config: Config = confy::load("covers-api", "covers-api")?;

    // construct the url of database connection.
    info!("connection to the DB");
    let pool_cover = Pool::builder(Manager::new(
        config.cover_db_uri.to_url()?.as_str(),
        deadpool_diesel::Runtime::Tokio1,
    ))
    .build()
    .expect("Could not connect to Cover DB");

    // state that Axum will keep
    let client_product =
        api_product_type_to_client(&config.product_api_type, config.product_api_uri.to_url()?)?;
    let client_cache = api_cache_type_to_client(&config.cache_api_type);
    let client_task = api_task_type_to_client(&config.tasks_api_type);
    let state = AppState {
        config,
        conn_db_cover: pool_cover,
        client_task,
        client_product,
        client_cache,
    };
    info!("checking and constructing tables");
    // create table if needed
    run_migrations(&state.conn_db_cover)
        .await
        .expect("failed to create table or connect to database.");

    // set up the API endpoints
    let adr = format!("127.0.0.1:{}", state.config.listen_port);
    info!("listening on {adr}");
    let listener = tokio::net::TcpListener::bind(&adr).await?;
    axum::serve(listener, routes(state)).await?;

    Ok(())
}

fn routes(state: AppState) -> Router {
    Router::new()
        .route("/:id/cover-:size", get(get_cover))
        .route("/cover-default-:size", get(get_default_cover))
        .route("/:id/retreive-cover", put(retrieve_cover_handle))
        .route("/missing-covers", put(retrieve_missing_covers))
        .route("/missing-covers", get(get_missing_covers))
        .route("/:id", post(add_manual_cover))
        .route("/:id", delete(delete_cover))
        .with_state(state)
}
