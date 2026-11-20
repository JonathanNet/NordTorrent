use std::io::SeekFrom;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::{
    auth::AuthUser,
    error::AppError,
    state::AppState,
    torrents::{file_for_stream, parse_torrent_id},
};

#[derive(Deserialize)]
pub struct StreamPath {
    id: String,
    file_id: usize,
}

pub async fn stream_file(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(path): Path<StreamPath>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let id = parse_torrent_id(&path.id)?;
    let (mime, _filename) = file_for_stream(&state.api, id, path.file_id)?;
    let mut stream = state.api.api_stream(id, path.file_id).map_err(|err| {
        tracing::debug!("stream open failed: {err:#}");
        AppError::not_found("Filen kan ikke streames endnu.")
    })?;
    let total = stream.len();
    if total == 0 {
        let mut response = StatusCode::OK.into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        response
            .headers_mut()
            .insert(header::CONTENT_LENGTH, HeaderValue::from_static("0"));
        return Ok(response);
    }

    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let (start, end, partial) = match range {
        None => (0, total - 1, false),
        Some(value) => match parse_range(value, total) {
            RangeResult::Partial(start, end) => (start, end, true),
            RangeResult::Invalid => {
                let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
                if let Ok(value) = HeaderValue::from_str(&format!("bytes */{total}")) {
                    response.headers_mut().insert(header::CONTENT_RANGE, value);
                }
                return Ok(response);
            }
        },
    };

    if start > 0 {
        stream
            .seek(SeekFrom::Start(start))
            .await
            .map_err(|err| AppError::internal("could not seek stream", err))?;
    }
    let length = end.saturating_sub(start).saturating_add(1);
    let limited = stream.take(length);
    let body = Body::from_stream(ReaderStream::with_capacity(limited, 64 * 1024));

    let mut response = if partial {
        StatusCode::PARTIAL_CONTENT.into_response()
    } else {
        StatusCode::OK.into_response()
    };
    let output = response.headers_mut();
    output.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    output.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .map_err(|err| AppError::internal("invalid content length header", err))?,
    );
    if partial {
        output.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{total}"))
                .map_err(|err| AppError::internal("invalid content range header", err))?,
        );
    }
    output.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    output.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    output.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    *response.body_mut() = body;
    Ok(response)
}

enum RangeResult {
    Partial(u64, u64),
    Invalid,
}

fn parse_range(value: &str, total: u64) -> RangeResult {
    let Some(spec) = value.strip_prefix("bytes=") else {
        return RangeResult::Invalid;
    };
    if spec.contains(',') {
        return RangeResult::Invalid;
    }
    let Some((start_raw, end_raw)) = spec.split_once('-') else {
        return RangeResult::Invalid;
    };
    let start_raw = start_raw.trim();
    let end_raw = end_raw.trim();

    if start_raw.is_empty() {
        let Ok(suffix) = end_raw.parse::<u64>() else {
            return RangeResult::Invalid;
        };
        if suffix == 0 {
            return RangeResult::Invalid;
        }
        let start = total.saturating_sub(suffix);
        return RangeResult::Partial(start, total - 1);
    }

    let Ok(start) = start_raw.parse::<u64>() else {
        return RangeResult::Invalid;
    };
    if start >= total {
        return RangeResult::Invalid;
    }
    if end_raw.is_empty() {
        return RangeResult::Partial(start, total - 1);
    }
    let Ok(end) = end_raw.parse::<u64>() else {
        return RangeResult::Invalid;
    };
    if end < start {
        return RangeResult::Invalid;
    }
    RangeResult::Partial(start, end.min(total - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ranges() {
        assert!(matches!(
            parse_range("bytes=0-99", 1000),
            RangeResult::Partial(0, 99)
        ));
        assert!(matches!(
            parse_range("bytes=500-", 1000),
            RangeResult::Partial(500, 999)
        ));
        assert!(matches!(
            parse_range("bytes=-50", 1000),
            RangeResult::Partial(950, 999)
        ));
        assert!(matches!(
            parse_range("bytes=0-5000", 1000),
            RangeResult::Partial(0, 999)
        ));
        assert!(matches!(
            parse_range("bytes=1000-", 1000),
            RangeResult::Invalid
        ));
    }
}
