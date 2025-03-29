#[macro_use]
extern crate diesel;

mod db;
mod models;
mod routes;
mod schema;

use actix_web::{middleware::Logger, web, App, HttpServer};
use db::establish_connection_pool;
use dotenvy::dotenv;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = establish_connection_pool(&database_url);

    let server_address = "0.0.0.0:8080";

    println!("Starting server at: {}", server_address);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(Logger::default())
            .configure(routes::init_routes)
    })
    .bind(server_address)?
    .run()
    .await
}
