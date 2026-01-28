use crate::api::response::{ApiResponse, ErrorResponse};
use crate::storage::Storage;
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

use super::{
    commands::{UploadFileCommand, UploadFileError},
    queries::{DownloadFileError, DownloadFileQuery},
};

pub fn files_routes() -> Router<Storage> {
    Router::new().route("/:org/:name/:version/:filename", post(upload_file).get(download_file))
}

#[tracing::instrument(skip(storage, multipart), fields(org = %org, name = %name, version = %version, filename = %filename))]
async fn upload_file(
    State(storage): State<Storage>,
    Path((org, name, version, filename)): Path<(String, String, String, String)>,
    mut multipart: Multipart,
) -> Result<Response, FileApiError> {
    let mut content: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        FileApiError::UploadError(UploadFileError::Storage(anyhow::anyhow!(
            "Failed to read multipart field: {}",
            e
        )))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" {
            content_type = field.content_type().map(|s| s.to_string());
            let data = field.bytes().await.map_err(|e| {
                FileApiError::UploadError(UploadFileError::Storage(anyhow::anyhow!(
                    "Failed to read file bytes: {}",
                    e
                )))
            })?;
            content = Some(data.to_vec());
        }
    }

    let content = content.ok_or_else(|| {
        FileApiError::UploadError(UploadFileError::Storage(anyhow::anyhow!(
            "No file field found in multipart data"
        )))
    })?;

    let command = UploadFileCommand {
        org,
        name,
        version,
        filename,
        content,
        content_type,
    };

    let response = super::commands::upload::handle(storage, command).await?;

    tracing::info!(
        key = %response.key,
        size = response.size,
        checksum = %response.checksum,
        "File uploaded via API"
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))).into_response())
}

#[tracing::instrument(skip(storage), fields(org = %org, name = %name, version = %version, filename = %filename))]
async fn download_file(
    State(storage): State<Storage>,
    Path((org, name, version, filename)): Path<(String, String, String, String)>,
) -> Result<Response, FileApiError> {
    let query = DownloadFileQuery {
        org,
        name,
        version,
        filename,
    };

    let response = super::queries::download::handle(storage, query).await?;

    tracing::debug!(
        presigned_url = %response.presigned_url,
        expires_in = response.expires_in,
        "File download URL generated via API"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))).into_response())
}

#[derive(Debug)]
enum FileApiError {
    UploadError(UploadFileError),
    DownloadError(DownloadFileError),
}

impl From<UploadFileError> for FileApiError {
    fn from(err: UploadFileError) -> Self {
        Self::UploadError(err)
    }
}

impl From<DownloadFileError> for FileApiError {
    fn from(err: DownloadFileError) -> Self {
        Self::DownloadError(err)
    }
}

impl IntoResponse for FileApiError {
    fn into_response(self) -> Response {
        match self {
            FileApiError::UploadError(UploadFileError::OrgRequired)
            | FileApiError::UploadError(UploadFileError::NameRequired)
            | FileApiError::UploadError(UploadFileError::VersionRequired)
            | FileApiError::UploadError(UploadFileError::FilenameRequired)
            | FileApiError::UploadError(UploadFileError::FilenameLength)
            | FileApiError::UploadError(UploadFileError::ContentRequired) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            FileApiError::UploadError(UploadFileError::Storage(_)) => {
                tracing::error!("Storage error during file upload: {}", self);
                let error = ErrorResponse::new("STORAGE_ERROR", "A storage error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },

            FileApiError::DownloadError(DownloadFileError::OrgRequired)
            | FileApiError::DownloadError(DownloadFileError::NameRequired)
            | FileApiError::DownloadError(DownloadFileError::VersionRequired)
            | FileApiError::DownloadError(DownloadFileError::FilenameRequired) => {
                let error = ErrorResponse::new("VALIDATION_ERROR", self.to_string());
                (StatusCode::BAD_REQUEST, Json(error)).into_response()
            },
            FileApiError::DownloadError(DownloadFileError::NotFound) => {
                let error = ErrorResponse::new("NOT_FOUND", "File not found");
                (StatusCode::NOT_FOUND, Json(error)).into_response()
            },
            FileApiError::DownloadError(DownloadFileError::Storage(_)) => {
                tracing::error!("Storage error during file download: {}", self);
                let error = ErrorResponse::new("STORAGE_ERROR", "A storage error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

impl std::fmt::Display for FileApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UploadError(e) => write!(f, "{}", e),
            Self::DownloadError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = FileApiError::UploadError(UploadFileError::FilenameRequired);
        assert!(err.to_string().contains("Filename is required"));
    }

    #[test]
    fn test_routes_structure() {
        let router = files_routes();
        assert!(format!("{:?}", router).contains("Router"));
    }
}
