mod seed;

use anyhow::Result;

#[actix_web::main]
async fn main() -> Result<()> {
    use sqlx::postgres::PgPoolOptions;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:password@localhost/dbname")
        .await?;
    
    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new().app_data(actix_web::web::Data::new(pool.clone()))
    })
    .bind(("127.0.0.1", 8080))?
    .run();

    server.await?;

    Ok(())
}

