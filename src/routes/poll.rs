use actix_web::{
    web::{self, ServiceConfig},
    HttpResponse, Responder,
};
use sqlx::QueryBuilder;

use crate::{
    models::{self, Message},
    AppData,
};

// normally I would split this large file into multiple smaller files,
// however to make it easier to navigate through the code in this file on GitHub
// I put it all in one file

async fn get_polls(app_data: web::Data<AppData>) -> impl Responder {
    let pool = &app_data.pool;
    let polls = sqlx::query_as!(
        models::Poll,
        r#"select id, title, poll_type as "poll_type!: models::PollType", created_at, timeout_at, delete_at from poll"#
    ).fetch_all(pool).await;
    match polls {
        Ok(polls) => HttpResponse::Ok().json(polls),
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().json(Message("internal server error"))
        }
    }
}

async fn get_poll(app_data: web::Data<AppData>, path_id: web::Path<i64>) -> impl Responder {
    let id = path_id.into_inner();
    let pool = &app_data.pool;
    let poll = sqlx::query_as!(
        models::Poll,
        r#"select id, title, poll_type as "poll_type!: models::PollType", created_at, timeout_at, delete_at from poll where id = $1"#,
        &id as &i64
    ).fetch_one(pool).await;
    match poll {
        Ok(poll) => HttpResponse::Ok().json(poll),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(Message("no such poll")),
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().json(Message("internal server error"))
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct PollCount {
    poll_id: i64,
    option_id: i64,
    option_name: String,
    count: i64,
}

const COLORS: &[&RGBColor] = &[&RED, &GREEN, &BLUE, &YELLOW, &CYAN, &MAGENTA];

const fn get_color(index: usize) -> &'static RGBColor {
    COLORS[index % COLORS.len()]
}

use anyhow::Result;
use plotters::prelude::*;

// technically this could also take some kind of Theme Enum
// to allow for dark mode or something like this
fn draw_bar_graph(caption: &str, data: &[PollCount]) -> Result<String> {
    let mut buffer = String::new();

    let data_len = data.len();

    // TODO: is there a way to not set the width and height, but still set the dimensions of the viewBox?
    let svg_backend = SVGBackend::with_string(&mut buffer, (600, 400));
    let root_area = svg_backend.into_drawing_area();
    root_area.fill(&WHITE)?;

    let max_count = data.iter().map(|x| x.count).max().unwrap_or(0);

    let mut context = ChartBuilder::on(&root_area)
        // might need to be changed according to size of biggest label
        .set_label_area_size(LabelAreaPosition::Left, 40)
        // to make sure the most right number on the x axis is not cut off
        .set_label_area_size(LabelAreaPosition::Right, 5)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .caption(caption, ("sans-serif", 40))
        // -1 because the upper bound is included, even though the range is exclusive
        .build_cartesian_2d(0..max_count, (0..data.len() - 1).into_segmented())?;

    context
        .configure_mesh()
        .y_label_formatter(&|x| match x {
            SegmentValue::CenterOf(x) => data[data_len - *x - 1].option_name.clone(),
            // this does not wrap the text
            // SegmentValue::CenterOf(x) => "hey\ntest\ncool".to_string(),
            _ => "".to_string(),
        })
        .draw()?;

    let data_values = data.iter().map(|x| x.count);

    context.draw_series((0..).zip(data_values).map(|(y, x)| {
        let reversed_y = data_len - y - 1;
        let mut bar = Rectangle::new(
            [
                (0, SegmentValue::Exact(reversed_y)),
                (x, SegmentValue::Exact(reversed_y + 1)),
            ],
            get_color(y).filled(),
        );
        bar.set_margin(5, 5, 0, 0);
        bar
    }))?;

    // as along as these are alive, they are still borrowing buffer
    // and it can't be returned
    drop(context);
    drop(root_area);

    Ok(buffer)
}

async fn get_poll_name(poll_id: i64, pool: &sqlx::PgPool) -> Result<String> {
    // we could wrap all of this in Ok and return on error case with ?
    // Ok(
    //     sqlx::query!(r#"select title from poll where id = $1"#, &poll_id as &i64)
    //     .fetch_one(pool)
    //     .await?
    //     .title,
    // )

    // however in my opinion, this is more readable
    let poll_name = sqlx::query!(r#"select title from poll where id = $1"#, &poll_id as &i64)
        .fetch_one(pool)
        .await?;
    Ok(poll_name.title)
}

async fn get_poll_graph(app_data: web::Data<AppData>, path_id: web::Path<i64>) -> impl Responder {
    let id = path_id.into_inner();
    let pool = &app_data.pool;

    let poll_count_result = retrieve_poll_counts(pool, id).await;

    match poll_count_result {
        Ok(poll_count) => {
            if poll_count.is_empty() {
                return HttpResponse::NotFound().json(Message("no such poll"));
            }
            let poll_title = unwrap_or_log_and_internal_server_error_response!(
                get_poll_name(id, pool).await,
                "internal server error"
            );
            let svg_content_result = draw_bar_graph(&poll_title, &poll_count);
            let svg_content = unwrap_or_log_and_internal_server_error_response!(
                svg_content_result,
                "internal server error"
            );
            HttpResponse::Ok()
                .content_type("image/svg+xml")
                .body(svg_content)
        }
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().json(Message("internal server error"))
        }
    }
}

/// if poll_id refers to a non existing poll, an empty vector is returned
async fn retrieve_poll_counts(pool: &sqlx::PgPool, poll_id: i64) -> sqlx::Result<Vec<PollCount>> {
    // this will return an empty vector if the poll_id refers to a non existing poll
    // isn't this the correct behavior though?
    // logically a poll needs to have at least two poll options
    // so an empty vector could indicate that
    // however it is not as explicit
    // TODO: check if poll with given id exists and if not return None
    sqlx::query_as!(
        PollCount,
        r#"select poll.id as poll_id, poll_option.id as option_id, poll_option.name as option_name,
        (select count(poll_vote.id)
            from poll_vote where poll_vote.option_id = poll_option.id) as "count!: i64"
        from poll, poll_option where poll_option.poll_id = poll.id and poll.id = $1"#,
        poll_id
    )
    .fetch_all(pool)
    .await
}

async fn get_poll_votes(app_data: web::Data<AppData>, path_id: web::Path<i64>) -> impl Responder {
    let id = path_id.into_inner();
    let pool = &app_data.pool;
    let poll_counts_result = retrieve_poll_counts(pool, id).await;
    match poll_counts_result {
        Ok(poll_counts) => {
            // to make sure this errors for a non existing poll
            if poll_counts.is_empty() {
                HttpResponse::NotFound().json(Message("no such poll"))
            } else {
                HttpResponse::Ok().json(poll_counts)
            }
        }
        // this error only appears for fetch_optional
        // Err(e) if matches!(e, sqlx::Error::RowNotFound) => {
        //     HttpResponse::NotFound().json(Message("no such poll"))
        // }
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().json(Message("internal server error"))
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollPostRequestData {
    title: String,
    poll_type: models::PollType,
    timeout_at: Option<chrono::DateTime<chrono::Utc>>,
    delete_at: Option<chrono::DateTime<chrono::Utc>>,
    poll_options: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PollPostResponseData {
    #[serde(flatten)]
    poll: models::Poll,
    poll_options: Vec<models::PollOption>,
}

// the names indicates that this returns a boolean
// should it be renamed?
// no, since it kind of acts like a boolean, however you additionally get
// the reason why it is not valid
fn are_poll_options_valid(poll_options: &Vec<String>) -> Result<(), &str> {
    if poll_options.len() < 2 {
        return Err("At least two poll options are required");
    }
    if poll_options.len() > 10 {
        return Err("At most ten poll options are allowed");
    }
    for (i, poll_option) in poll_options.iter().enumerate() {
        if poll_option.is_empty() {
            return Err("poll option is empty");
        }
        // this only checks options after the current one
        // we don't have to check with previous ones, since
        // the previous ones already compared themselves
        // with the current one
        for other_poll_option in poll_options.iter().skip(i + 1) {
            if poll_option == other_poll_option {
                return Err("poll options are not unique");
            }
        }
    }
    Ok(())
}

async fn post_poll(
    app_data: web::Data<AppData>,
    poll: web::Json<PollPostRequestData>,
) -> impl Responder {
    let pool = &app_data.pool;
    let request_data = poll.into_inner();

    // check title length bigger than 0
    if request_data.title.is_empty() {
        return HttpResponse::BadRequest().json(Message("title is empty"));
    }
    // check timeout_at is in the future
    if let Some(timeout_at) = request_data.timeout_at {
        if timeout_at < chrono::Utc::now() {
            return HttpResponse::BadRequest().json(Message("timeout_at is in the past"));
        }
    }
    // check delete_at is higher or equal to timeout_at
    if let Some(delete_at) = request_data.delete_at {
        if delete_at
            < request_data
                .timeout_at
                .unwrap_or(chrono::Utc::now() + chrono::Duration::minutes(30))
        {
            return HttpResponse::BadRequest().json(Message("delete_at is lower than timeout_at"));
        }
    }

    if let Err(e) = are_poll_options_valid(&request_data.poll_options) {
        return HttpResponse::BadRequest().json(e);
    }

    let mut query_builder =
        QueryBuilder::new("insert into poll (title, poll_type, timeout_at, delete_at) values (");
    query_builder.push_bind(&request_data.title);
    query_builder.push(", ");
    query_builder.push_bind(&request_data.poll_type);
    query_builder.push(", ");
    // we can't remap the option, since for the value
    // we have to call push_bind and in case of absence
    // of a value we have to use the default
    if let Some(timeout_at) = request_data.timeout_at {
        query_builder.push_bind(timeout_at.to_rfc3339());
    } else {
        query_builder.push("default");
    }
    query_builder.push(", ");
    if let Some(delete_at) = request_data.delete_at {
        query_builder.push_bind(delete_at.to_rfc3339());
    } else {
        query_builder.push("default");
    }
    query_builder.push(r#") returning id, title, poll_type, created_at, timeout_at, delete_at"#);

    let query = query_builder.build_query_as::<models::Poll>();
    // we need to start a transaction
    // because we also need to insert the options
    let transaction_result = pool.begin().await;
    let mut transaction = match transaction_result {
        Ok(transaction) => transaction,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().json("internal server error");
        }
    };

    let insert_result = query.fetch_one(transaction.as_mut()).await;

    let poll = match insert_result {
        Ok(poll) => poll,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().json("internal server error");
        }
    };

    let mut option_insert_query_builder =
        QueryBuilder::new(r#"insert into poll_option (poll_id, name) "#);

    option_insert_query_builder.push_values(
        request_data.poll_options.into_iter(),
        |mut b, poll_option| {
            b.push_bind(poll.id).push_bind(poll_option);
        },
    );

    option_insert_query_builder.push(r#" returning id, name, poll_id"#);

    let query = option_insert_query_builder.build_query_as::<models::PollOption>();
    let options_insert_result = query.fetch_all(transaction.as_mut()).await;

    // matches like these can be done with a macro
    //
    // let inserted_poll_options = match options_insert_result {
    //     Ok(value) => value,
    //     Err(e) => {
    //         log::error!("{}", e);
    //         return HttpResponse::InternalServerError().json(Message("internal server error"));
    //     }
    // };
    //
    // and thus reduce code repetition
    let inserted_poll_options = unwrap_or_log_and_internal_server_error_response!(
        options_insert_result,
        "internal server error"
    );

    let commit_result = transaction.commit().await;
    // if let Err(error) = commit_result {
    //     log::error!("{}", error);
    //     return HttpResponse::InternalServerError().json(Message("internal server error"));
    // }
    unwrap_or_log_and_internal_server_error_response!(commit_result, "internal server error");

    let response_data = PollPostResponseData {
        poll,
        poll_options: inserted_poll_options,
    };
    HttpResponse::Ok().json(response_data)
}

pub fn configure_routes(config: &mut ServiceConfig) {
    config.route("", web::get().to(get_polls));
    // this could alternatively be done with a header guard that checks for
    // the accept header, if json, send json, otherwise send an image
    config.route("/{id}", web::get().to(get_poll));
    config.route("/{id}/graph", web::get().to(get_poll_graph));
    config.route("/{id}/votes", web::get().to(get_poll_votes));
    config.route("", web::post().to(post_poll));
}
