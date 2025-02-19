#![forbid(unsafe_code)]
#![allow(dead_code)]

#[macro_use]
extern crate log;

mod base64;
mod infrastructure;
mod seed;
mod traits;
mod use_case;

use anyhow::Result;
use infrastructure::database::PostgresDatabase;

#[actix_web::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let pg_pool = PostgresDatabase::new().await?;

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new().app_data(actix_web::web::Data::new(pg_pool.clone()))
    })
    .bind(("127.0.0.1", 8080))?
    .run();

    server.await?;

    Ok(())
}
