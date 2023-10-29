use std::sync::OnceLock;

use actix_web::{HttpRequest, HttpResponse};

macro_rules! unwrap_or_log_and_internal_server_error_response {
    ($result:expr, $message:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                log::error!("{}", e);
                // Message is public to the whole crate, so prepending it with "crate::models::"
                // should not be required
                return HttpResponse::InternalServerError().json(Message($message));
            }
        }
    };
}

pub mod option;
pub mod poll;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiIndexResponseData {
    polls: String,
    poll_options: String,
}

static ENDPOINTS: OnceLock<ApiIndexResponseData> = OnceLock::new();

pub async fn get_api_index(request: HttpRequest) -> HttpResponse {
    let endpoints = ENDPOINTS.get_or_init(move || {
        let port = request.app_config().local_addr().port();
        let origin = format!("http://{}:{}/", "127.0.0.1", port);
        ApiIndexResponseData {
            polls: format!("{}polls", origin),
            poll_options: format!("{}poll-options", origin),
        }
    });
    HttpResponse::Ok().json(endpoints)
}
