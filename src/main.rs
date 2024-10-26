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
use deadpool_diesel::mysql::Manager;
use deadpool_diesel::mysql::Pool;
use get_pass::url::add_pass_to_url;
use reqwest::Client;
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

#[derive(Clone)]
struct AppState {
    config: Config,
    // connection to the cover api database
    conn_db_cover: Pool,
    // client to request external API
    client_task: tasks_tracker_client::Client,
    client_product: doli_client_api_rs::Client,
    client_cache: Client,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    // config file with database url
    info!("loading config file");
    let config: Config = confy::load("covers-api", "covers-api")?;

    // construct the url of database connection.
    info!("connection to the DB");
    let mut uri_cover_db = config.cover_db_uri.clone();
    add_pass_to_url(&mut uri_cover_db, &config.cover_db_path_pass)?;
    let pool_cover = Pool::builder(Manager::new(
        uri_cover_db.as_str(),
        deadpool_diesel::Runtime::Tokio1,
    ))
    .build()
    .expect("Could not connect to Cover DB");

    // state that Axum will keep

    // construct clients for cache, tasks and product API

    // product API
    let mut uri_product_api = config.product_api_uri.clone();
    add_pass_to_url(&mut uri_product_api, &config.product_api_path_pass)?;
    let client_product = doli_client_api_rs::Client::new(uri_product_api)?;

    // Tasks tracker API
    let mut uri_tasks_tracker_api = config.tasks_api_uri.clone();
    add_pass_to_url(&mut uri_tasks_tracker_api, &config.tasks_api_pass_path)?;
    let client_tasks_tracker = tasks_tracker_client::Client::new(uri_tasks_tracker_api)?;

    // Cache API (simple Client)
    let client_cache = reqwest::Client::new();

    let state = AppState {
        config,
        conn_db_cover: pool_cover,
        client_task: client_tasks_tracker,
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
