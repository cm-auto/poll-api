use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer, Scope};

use crate::{background_tasks::spawn_database_cleaner_task, routes::get_api_index};

mod models;
mod routes;

mod background_tasks;

struct AppData {
    pool: sqlx::PgPool,
}

#[actix_web::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");

    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pg_pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    spawn_database_cleaner_task(pg_pool.clone());

    let bind_address = dotenvy::var("BIND_ADDRESS").unwrap_or("0.0.0.0".to_string());
    let port = dotenvy::var("PORT")
        .unwrap_or("1337".to_string())
        .parse::<u16>()
        .expect("Could not parse PORT");

    let api_prefix = "/";

    let app_data = web::Data::new(AppData { pool: pg_pool });

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    println!("Listening on {}:{}", bind_address, port);
    HttpServer::new(move || {
        let polls_scope =
            Scope::new(&format!("{}polls", api_prefix)).configure(routes::poll::configure_routes);
        let options_scope = Scope::new(&format!("{}poll-options", api_prefix))
            .configure(routes::option::configure_routes);

        App::new()
            .app_data(app_data.clone())
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::Trim,
            ))
            .wrap(middleware::Logger::default())
            .wrap(Cors::permissive())
            .route(api_prefix, web::get().to(get_api_index))
            .service(polls_scope)
            .service(options_scope)
    })
    .bind((bind_address, port))
    .unwrap()
    .run()
    .await
    .unwrap()
}
