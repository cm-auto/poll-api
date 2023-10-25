use actix_web::HttpResponse;

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

pub async fn get_api_index() -> HttpResponse {
    // These two can't be consts,
    // since the base url and the api prefix would have to be set
    // from the env or program args.
    // TODO: however we should make use of memoization
    // TODO: actually use the values passed to the program
    HttpResponse::Ok().json(ApiIndexResponseData {
        polls: "http://127.0.0.1:1337/polls".to_string(),
        poll_options: "http://127.0.0.1:1337/poll-options".to_string(),
    })
}
