use crate::ClientTask;
use anyhow::bail;
use std::{fs::remove_file, path::PathBuf, sync::Arc};
use strum::IntoEnumIterator;
use tasks_tracker_client::abort_task;
use tasks_tracker_client::finish_task;

use axum::{
    body::to_bytes,
    extract::{Path, Request, State},
    http::HeaderValue,
    response::{AppendHeaders, IntoResponse},
    Json,
};
use deadpool_diesel::mysql::Pool;
use enclose::enc;
use get_pass::get_password;
use reqwest::{header::HOST, Client, StatusCode, Url};
use tasks_tracker_client::{create_simple_task, update_task_progress, ResponseNewTask};
use tokio::{
    spawn,
    sync::mpsc::{self, Receiver},
    task::JoinHandle,
};

use crate::{
    config::TaskTrackerApiType,
    cover::{all_id, all_id_missing_retrievable, retrieve_cover, update_table_image, CoverSize},
    error::AppError,
    image::write_cover,
    provider::CoverProvider,
    AppState, ClientCache, ClientProduct,
};

pub async fn retrieve_cover_handle(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // create a task and return the location for it.
    match state.config.tasks_api_type {
        TaskTrackerApiType::TaskTrackerRs(ref p) => {
            let rep = create_simple_task(
                client_task(&state.client_task),
                &state.config.tasks_api_uri.to_url()?,
                String::from("cover api"),
                ["retrieve cover for product", &id.to_string()].concat(),
                &get_password(p).map_err(|_| AppError::Conf)?,
            )
            .await
            .map_err(|_| AppError::Backend)?;
            // return headers from task tracker
            let rep = Arc::new(rep);
            let headers = AppendHeaders([
                ("Content-Location", rep.location.to_string()),
                ("ViewToken", rep.view_token.to_owned()),
            ]);
            // start the job, update task tracker and cache
            spawn(enc!((state, rep)async move {
                wrapper_retrieve_cover(id, state, &rep).await.unwrap();
            }));

            // the task began and token to review it is given back.
            // the token to abort it is not given since the result of the task should be fast.
            Ok((StatusCode::ACCEPTED, headers))
            // if the task api is misconfigured, the server return an internal server error.
        }
    }
    // spawn the task that will update the task.
}

async fn wrapper_retrieve_cover(
    id: u32,
    state: AppState,
    rep: &ResponseNewTask,
) -> Result<(), AppError> {
    // channel to receive progress from task and send it to task tracker.
    let (update_progress, receive_progress) = mpsc::channel(4);
    // start job
    let handle_retrieve = spawn(enc!((state) async move {
        retrieve_cover(
            id,
            &state.conn_db_cover,
            &state.client_product,
            &state.config.path_covers,
            state.config.wait_seconds_retry_retrieve_cover,
            update_progress,
        )
        .await
    }));
    let location = rep.location.to_owned();
    let token_update = rep.update_token.clone();
    // update task tracker
    let handle = spawn(enc!((state) async move {
        manage_tracker_status(
            &state.client_task,
            &location,
            token_update,
            receive_progress,
            handle_retrieve,
        )
        .await
    }));
    // update cache once the job is finished.
    if handle.await.is_ok() {
        update_cache_cover(&state, id)
            .await
            .map_err(|_| AppError::Backend)?;
    }
    Ok(())
}
/// Retrieve missing covers
/// Will search for all product covers not present in cover table and for product cover that are missing and can be retrieved.
pub async fn retrieve_missing_covers(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // create a task and return the location for it.
    match state.config.tasks_api_type {
        TaskTrackerApiType::TaskTrackerRs(ref p) => {
            let rep = create_simple_task(
                client_task(&state.client_task),
                &state
                    .config
                    .tasks_api_uri
                    .to_url()
                    .map_err(|_| AppError::Conf)?,
                String::from("cover api"),
                "retrieve missing covers".to_string(),
                &get_password(p).map_err(|_| AppError::Conf)?,
            )
            .await
            .map_err(|_| AppError::Backend)?;

            // return headers from task tracker
            let rep = Arc::new(rep);
            let headers = AppendHeaders([
                ("Content-Location", rep.location.as_str()),
                ("ViewToken", &rep.view_token),
            ]);
            // start the job, update task tracker and cache
            // errors will be for the task tracker
            spawn(enc!((state, rep)async move {
                wrapper_retrieve_missing_covers(state, &rep).await.unwrap();
            }));

            // the task began and token to review it is given back.
            // the token to abort it is not given since the result of the task should be fast.
            Ok((StatusCode::ACCEPTED, headers).into_response())
        }
    }
}
async fn wrapper_retrieve_missing_covers(
    state: AppState,
    rep: &ResponseNewTask,
) -> anyhow::Result<()> {
    //
    // channel to receive progress from task and send it to task tracker.
    let (update_progress, receive_progress) = mpsc::channel(4);
    // start job
    // get list of id to retrieve
    // if this step fail, cover db or product api is misconfigured, abort task.
    let missing_ids = get_missing_id(
        &state.conn_db_cover,
        &state.client_product,
        state.config.wait_seconds_retry_retrieve_cover,
    )
    .await?;
    let mut count = 0;
    let ids = missing_ids
        .into_iter()
        .inspect(|_| count += 1)
        .filter(|&i| i % 2 == 0)
        .collect::<Vec<_>>();
    // why fail ?
    update_progress.send(1).await?;
    // for each id, retrieve cover, update cache
    let handler = spawn(enc!((state) async move {
        for (nb, id) in ids.into_iter().enumerate() {
            let (update_progress_unit, _receive_progress_unit) = mpsc::channel(4);
            // start job
            if retrieve_cover(
                id,
                &state.conn_db_cover,
                &state.client_product,
                &state.config.path_covers,
                state.config.wait_seconds_retry_retrieve_cover,
                update_progress_unit,
            )
            .await
            .is_ok()
            {
                // if job completed, update progress and update cache
                update_progress.send((nb / count * 100) as u8).await?;
                update_cache_cover(&state, id).await?;
            }
        }
        Ok(())
    }));

    let location = rep.location.to_owned();
    let token_update = rep.update_token.clone();
    // update task tracker
    manage_tracker_status(
        &state.client_task,
        &location,
        token_update,
        receive_progress,
        handler,
    )
    .await?;
    Ok(())
    // cache is updated for each cover
}

async fn get_missing_id(
    conn_db_cover: &Pool,
    client_product: &ClientProduct,
    wait_try: u64,
) -> anyhow::Result<Vec<u32>> {
    match client_product {
        ClientProduct::Dolibarr(c) => {
            // get all products id
            let mut products = doli_client_api_rs::get_all_products(c).await?;
            // get all products id already in cover table
            let conn = conn_db_cover.get().await?;
            let products_in_table_cover = all_id(&conn).await?;
            products.retain(|id| !products_in_table_cover.contains(id));
            // add ids that are present in table but missing covers that can be retrieved.
            let id_covers_missing_retrievable = all_id_missing_retrievable(&conn, wait_try).await?;
            products.extend(id_covers_missing_retrievable);
            Ok(products)
        }
    }
}

/// get the ids of every missing covers.
/// should be protected behind admin authentication
pub async fn get_missing_covers(State(state): State<AppState>) -> impl IntoResponse {
    if let Ok(missing_ids) = get_missing_id(
        &state.conn_db_cover,
        &state.client_product,
        state.config.wait_seconds_retry_retrieve_cover,
    )
    .await
    {
        return (StatusCode::OK, Json(missing_ids)).into_response();
    }
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}
pub async fn add_manual_cover(
    Path(id): Path<u32>,
    State(state): State<AppState>,
    request: Request,
) -> Result<impl IntoResponse, AppError> {
    // check if format compatible
    // task
    // create a task and return the location for it.
    match state.config.tasks_api_type {
        TaskTrackerApiType::TaskTrackerRs(ref p) => {
            let rep = create_simple_task(
                client_task(&state.client_task),
                &state
                    .config
                    .tasks_api_uri
                    .to_url()
                    .map_err(|_| AppError::Conf)?,
                String::from("cover api"),
                ["add manual cover for product", &id.to_string()].concat(),
                &get_password(p).map_err(|_| AppError::Conf)?,
            )
            .await
            .map_err(|_| AppError::Backend)?;

            // return headers from task tracker
            let rep = Arc::new(rep);
            let headers = AppendHeaders([
                ("Content-Location", rep.location.as_str()),
                ("ViewToken", &rep.view_token),
            ]);
            // start the job, update task tracker and cache
            spawn(enc!((state, rep)async move {
                wrapper_add_manual_cover(id, state, &rep, request).await.unwrap();
            }));

            // the task began and token to review it is given back.
            // the token to abort it is not given since the result of the task should be fast.
            Ok((StatusCode::ACCEPTED, headers).into_response())
        }
    }
}

async fn wrapper_add_manual_cover(
    id: u32,
    state: AppState,
    rep: &ResponseNewTask,
    request: Request,
) -> Result<(), AppError> {
    // channel to receive progress from task and send it to task tracker.
    let (update_progress, receive_progress) = mpsc::channel(4);
    // start job
    let body = request.into_body();
    let handle_retrieve = spawn(enc!((state) async move {
        let bytes = to_bytes(body, usize::MAX).await?;
        write_cover(&bytes, id, &state.config.path_covers)?;
        update_progress.send(50).await?;
        let conn = state.conn_db_cover.get().await?;
        update_table_image(id, &conn, Some(CoverProvider::Manual)).await?;
        Ok(())
    }));
    let location = rep.location.to_owned();
    let token_update = rep.update_token.clone();
    // update task tracker
    let handle = spawn(enc!((state) async move {
        manage_tracker_status(
            &state.client_task,
            &location,
            token_update,
            receive_progress,
            handle_retrieve,
        )
        .await
    }));
    // update cache once the job is finished.
    if handle.await.is_ok() {
        update_cache_cover(&state, id).await?;
    }
    Ok(())
}

pub async fn delete_cover(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // delete cover
    delete_cover_files(id, state.config.path_covers.to_owned())?;
    let conn = state.conn_db_cover.get().await?;
    update_table_image(id, &conn, None).await?;
    update_cache_cover(&state, id).await?;
    Ok(())
}

fn delete_cover_files(id: u32, mut path: PathBuf) -> Result<(), std::io::Error> {
    path.push(id.to_string());
    path.push("-");
    for s in CoverSize::iter() {
        let mut path = path.to_path_buf();
        path.push(s.to_string());
        remove_file(path)?;
    }
    Ok(())
}
async fn update_cache_cover(state: &AppState, id: u32) -> Result<(), AppError> {
    match state.config.cache_api_type {
        crate::config::CacheApiType::Mnemosyne => {
            // delete entry per path
            // path API cache
            let uri = state
                .config
                .cache_api_uri
                .to_url()
                .map_err(|_| AppError::Conf)?;
            // path cover API
            for variant in CoverSize::iter() {
                let url = format!("{uri}/api/1/cache/path/{id}/cover-{variant}");
                let host = &state.config.hostname;
                let ClientCache::CachingProxy(ref client) = state.client_cache;
                client
                    .delete(url)
                    .header(
                        HOST,
                        HeaderValue::from_str(host).map_err(|_| AppError::Host)?,
                    )
                    .send()
                    .await
                    .map_err(|_| AppError::Backend)?;
            }
        }
    }
    Ok(())
}
fn client_task(client: &ClientTask) -> &Client {
    match client {
        ClientTask::TasksTracker(c) => c,
    }
}
async fn manage_tracker_status(
    client: &ClientTask,
    task_location: &Url,
    token_update: String,
    mut receiver: Receiver<u8>,
    handler: JoinHandle<anyhow::Result<()>>,
) -> anyhow::Result<()> {
    // check for updates in a loop and update task tracker
    // job should finish when receiver is done
    // if update failed, task tracker backend has failed or is misconfigured.
    // we don't abort the work in this case.
    spawn(enc!((client, task_location, token_update) async move {
        loop {
            if let Some(prog) = receiver.recv().await {
                update_task_progress(client_task(&client), &task_location, &token_update, prog)
                    .await.map_err(|_|AppError::Backend)?;
            } else {
                return Ok::<(), AppError>(());
            }
        }
    }));
    // listen for incoming abort and abort job
    // check if the job is finished. If error, put status aborted on task tracker with error description.
    // if ok, status finish
    if let Err(err) = handler.await {
        abort_task(
            client_task(client),
            task_location,
            &token_update,
            Some(&err.to_string()),
            &[],
        )
        .await?;
        bail!("task had an issue and was aborted");
    }
    finish_task(client_task(client), task_location, &token_update, None, &[]).await?;
    Ok(())
}
