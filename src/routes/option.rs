use actix_web::{
    web::{self, ServiceConfig},
    HttpRequest, HttpResponse, Responder,
};
use sqlx::types::ipnetwork::IpNetwork;

use crate::{
    models::{self, Message},
    AppData,
};

async fn get_option(app_data: web::Data<AppData>, path_id: web::Path<i64>) -> impl Responder {
    let id = path_id.into_inner();
    let pool = &app_data.pool;
    let poll_option_result = sqlx::query_as!(
        models::PollOption,
        r#"select id, poll_id, name from poll_option where id = $1"#,
        &id as &i64
    )
    .fetch_one(pool)
    .await;

    match poll_option_result {
        Ok(poll_option) => HttpResponse::Ok().json(poll_option),
        Err(sqlx::Error::RowNotFound) => {
            HttpResponse::NotFound().json(Message("no such poll option"))
        }
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().json(Message("internal server error"))
        }
    }
}

async fn post_vote(
    app_data: web::Data<AppData>,
    path_id: web::Path<i64>,
    request: HttpRequest,
) -> impl Responder {
    let id = path_id.into_inner();
    let pool = &app_data.pool;
    let ip_address: IpNetwork = match request.peer_addr() {
        Some(addr) => addr.ip().into(),
        None => {
            log::error!("peer_addr is None");
            return HttpResponse::InternalServerError().json(Message("internal server error"));
        }
    };

    // TODO check if poll allows multiple votes and if the ip address has not
    // already voted
    // if only a single vote per poll is allowed check if the current ip address already voted for this poll
    // if multiple votes per poll are allowed check if the current ip address already voted for this option

    let vote_result = sqlx::query_as!(
        models::PollVote,
        r#"insert into poll_vote (option_id, ip_address) values ($1, $2) returning *"#,
        &id as &i64,
        ip_address
    )
    .fetch_one(pool)
    .await;
    match vote_result {
        Ok(vote) => HttpResponse::Ok().json(vote),
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().json(Message("internal server error"))
        }
    }
}

pub fn configure_routes(config: &mut ServiceConfig) {
    config.route("/{id}", web::get().to(get_option));
    // only post for votes
    config.route("/{id}/votes", web::post().to(post_vote));
    // config.route("", web::post().to(post_poll));
}
