use actix_web as aweb;
use aweb::error::{JsonPayloadError, QueryPayloadError};
use document_formats::DocumentFormatError;
use meilisearch_types::error::{Code, ErrorCode, ResponseError};
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum MeilisearchHttpError {
    #[error("A Content-Type header is missing. Accepted values for the Content-Type header are: {}",
            .0.iter().map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", "))]
    MissingContentType(Vec<String>),
    #[error(
        "The Content-Type `{0}` is invalid. Accepted values for the Content-Type header are: {}",
        .1.iter().map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", ")
    )]
    InvalidContentType(String, Vec<String>),
    #[error(transparent)]
    IndexScheduler(#[from] index_scheduler::Error),
    #[error(transparent)]
    Payload(#[from] PayloadError),
    #[error(transparent)]
    FileStore(#[from] file_store::Error),
    #[error(transparent)]
    DocumentFormat(#[from] DocumentFormatError),
    #[error(transparent)]
    Join(#[from] JoinError),
}

impl ErrorCode for MeilisearchHttpError {
    fn error_code(&self) -> Code {
        match self {
            MeilisearchHttpError::MissingContentType(_) => Code::MissingContentType,
            MeilisearchHttpError::InvalidContentType(_, _) => Code::InvalidContentType,
            MeilisearchHttpError::IndexScheduler(e) => e.error_code(),
            MeilisearchHttpError::Payload(e) => e.error_code(),
            MeilisearchHttpError::FileStore(_) => Code::Internal,
            MeilisearchHttpError::DocumentFormat(e) => e.error_code(),
            MeilisearchHttpError::Join(_) => Code::Internal,
        }
    }
}

impl From<MeilisearchHttpError> for aweb::Error {
    fn from(other: MeilisearchHttpError) -> Self {
        aweb::Error::from(ResponseError::from(other))
    }
}

impl From<aweb::error::PayloadError> for MeilisearchHttpError {
    fn from(error: aweb::error::PayloadError) -> Self {
        MeilisearchHttpError::Payload(PayloadError::Payload(error))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PayloadError {
    #[error(transparent)]
    Payload(aweb::error::PayloadError),
    #[error(transparent)]
    Json(JsonPayloadError),
    #[error(transparent)]
    Query(QueryPayloadError),
    #[error("The json payload provided is malformed. `{0}`.")]
    MalformedPayload(serde_json::error::Error),
    #[error("A json payload is missing.")]
    MissingPayload,
}

impl ErrorCode for PayloadError {
    fn error_code(&self) -> Code {
        match self {
            PayloadError::Payload(e) => match e {
                aweb::error::PayloadError::Incomplete(_) => todo!(),
                aweb::error::PayloadError::EncodingCorrupted => todo!(),
                aweb::error::PayloadError::Overflow => todo!(),
                aweb::error::PayloadError::UnknownLength => todo!(),
                aweb::error::PayloadError::Http2Payload(_) => todo!(),
                aweb::error::PayloadError::Io(_) => todo!(),
                _ => todo!(),
            },
            PayloadError::Json(err) => match err {
                JsonPayloadError::Overflow { .. } => Code::PayloadTooLarge,
                JsonPayloadError::ContentType => Code::UnsupportedMediaType,
                JsonPayloadError::Payload(aweb::error::PayloadError::Overflow) => {
                    Code::PayloadTooLarge
                }
                JsonPayloadError::Payload(_) => Code::BadRequest,
                JsonPayloadError::Deserialize(_) => Code::BadRequest,
                JsonPayloadError::Serialize(_) => Code::Internal,
                _ => Code::Internal,
            },
            PayloadError::Query(err) => match err {
                QueryPayloadError::Deserialize(_) => Code::BadRequest,
                _ => Code::Internal,
            },
            PayloadError::MissingPayload => Code::MissingPayload,
            PayloadError::MalformedPayload(_) => Code::MalformedPayload,
        }
    }
}

impl From<JsonPayloadError> for PayloadError {
    fn from(other: JsonPayloadError) -> Self {
        match other {
            JsonPayloadError::Deserialize(e)
                if e.classify() == serde_json::error::Category::Eof
                    && e.line() == 1
                    && e.column() == 0 =>
            {
                Self::MissingPayload
            }
            JsonPayloadError::Deserialize(e)
                if e.classify() != serde_json::error::Category::Data =>
            {
                Self::MalformedPayload(e)
            }
            _ => Self::Json(other),
        }
    }
}

impl From<QueryPayloadError> for PayloadError {
    fn from(other: QueryPayloadError) -> Self {
        Self::Query(other)
    }
}

impl From<PayloadError> for aweb::Error {
    fn from(other: PayloadError) -> Self {
        aweb::Error::from(ResponseError::from(other))
    }
}
