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

    // first make sure option exists
    let poll_option_id_result =
        sqlx::query!(r#"select id from poll_option where id = $1"#, &id as &i64)
            .fetch_optional(pool)
            .await;
    let poll_option = unwrap_or_log_and_internal_server_error_response!(
        poll_option_id_result,
        "internal server error"
    );
    if poll_option.is_none() {
        return HttpResponse::BadRequest().json(Message("no such poll option"));
    }

    // get all votes for this ip address and the specified poll_option
    let casted_votes_of_ip_result = sqlx::query_as!(
        models::PollVote,
        r#"select id, option_id, ip_address, created_at from poll_vote where ip_address = $1 and option_id = $2"#,
        &ip_address,
        &id,
    )
    .fetch_all(pool)
    .await;

    let casted_votes_of_ip = unwrap_or_log_and_internal_server_error_response!(
        casted_votes_of_ip_result,
        "internal server error"
    );

    // if there are no votes of the ip address for this option
    // then no further option is allowed and we can proceed
    if !casted_votes_of_ip.is_empty() {
        // then check if ip already voted for this option
        if casted_votes_of_ip.iter().any(|item| item.option_id == id) {
            return HttpResponse::BadRequest()
                .json(Message("you have already voted for this option"));
        }

        // now let's check if the poll allows multiple votes
        let poll_result = sqlx::query!(
            r#"select poll_type as "poll_type!: models::PollType"
            from poll inner join poll_option on poll.id = poll_option.poll_id
            where poll_option.id = $1"#,
            &id as &i64
        )
        .fetch_one(pool)
        .await;
        let poll =
            unwrap_or_log_and_internal_server_error_response!(poll_result, "internal server error");
        if poll.poll_type == models::PollType::Single {
            return HttpResponse::BadRequest().json(Message("poll does not allow multiple votes"));
        }
    }

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
}
