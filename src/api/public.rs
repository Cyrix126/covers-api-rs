use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{error::AppError, AppState};

pub async fn get_cover(
    Path(id): Path<u32>,
    Path(size): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let mut path_cover = state.config.path_covers.to_owned();
    path_cover.push(format!("{id}-{size}.webp"));
    Ok(tokio::fs::read(path_cover).await?)
}

pub async fn get_default_cover(
    Path(size): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let mut path_cover = state.config.path_covers.to_owned();
    path_cover.push(format!("cover-default-{size}.webp"));
    Ok(tokio::fs::read(path_cover).await?)
}
