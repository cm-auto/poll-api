use std::time::Duration;

async fn delete_old_polls(pool: &sqlx::PgPool) {
    let execute_result = sqlx::query!("delete from poll where delete_at <= now()")
        .execute(pool)
        .await;
    match execute_result {
        Ok(query_result) => {
            let rows_affected = query_result.rows_affected();
            if rows_affected > 0 {
                log::info!("deleted {} rows from table poll", rows_affected);
            }
        }
        Err(e) => {
            log::error!("{}", e);
        }
    }
}

pub fn spawn_database_cleaner_task(pool: sqlx::PgPool) {
    actix_rt::spawn(async move {
        // every hour
        let hours = 1;
        let seconds = hours * 60 * 60;
        let mut interval = actix_rt::time::interval(Duration::from_secs(seconds));
        loop {
            interval.tick().await;
            delete_old_polls(&pool).await;
        }
    });
}
