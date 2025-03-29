use actix_web::{get, post, web, HttpResponse, Responder};
use diesel::prelude::*;
use rand::{distributions::Alphanumeric, Rng};
use std::env;

use crate::db::DbPool;
use crate::models::{NewUrl, Url};
use crate::schema::urls::dsl::*;

#[post("/")]
async fn create_url(
    pool: web::Data<DbPool>,
    item: web::Json<NewUrlRequest>,
) -> impl Responder {
    let conn = pool.get().expect("Couldn't get db connection from pool");

    // Validate the URL (very simple validation)
    if item.original_url.trim().is_empty() {
        return HttpResponse::BadRequest().body("Original URL is required");
    }

    // Generate a short code
    let generated_code: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();

    let new_url = NewUrl {
        original_url: item.original_url.clone(),
        short_code: generated_code.clone(),
    };

    // Insert into database
    let inserted_url = web::block(move || {
        use crate::schema::urls;
        diesel::insert_into(urls::table)
            .values(&new_url)
            .execute(&conn)
            .and_then(|_| {
                urls.filter(short_code.eq(&generated_code))
                    .first::<Url>(&conn)
            })
    })
    .await;

    match inserted_url {
        Ok(url_entry) => {
            let base = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
            let short_url = format!("{}/{}", base, url_entry.short_code);
            HttpResponse::Created().json(serde_json::json!({
                "original_url": url_entry.original_url,
                "short_url": short_url,
                "short_code": url_entry.short_code,
                "created_at": url_entry.created_at
            }))
        }
        Err(err) => {
            eprintln!("Error inserting URL: {:?}", err);
            HttpResponse::InternalServerError().body("Error creating short URL")
        }
    }
}

#[get("/")]
async fn list_urls(pool: web::Data<DbPool>) -> impl Responder {
    let conn = pool.get().expect("Couldn't get db connection from pool");

    let urls_data = web::block(move || urls.order(created_at.desc()).load::<Url>(&conn)).await;
    match urls_data {
        Ok(data) => HttpResponse::Ok().json(data),
        Err(err) => {
            eprintln!("Error loading URLs: {:?}", err);
            HttpResponse::InternalServerError().body("Error loading URLs")
        }
    }
}

#[get("/{code}")]
async fn redirect_url(
    pool: web::Data<DbPool>,
    web::Path(code): web::Path<String>,
) -> impl Responder {
    let conn = pool.get().expect("Couldn't get db connection from pool");

    let result = web::block(move || {
        urls.filter(short_code.eq(code))
            .first::<Url>(&conn)
    })
    .await;

    match result {
        Ok(url_entry) => HttpResponse::Found()
            .append_header(("Location", url_entry.original_url))
            .finish(),
        Err(_) => HttpResponse::NotFound().body("URL not found"),
    }
}

#[derive(serde::Deserialize)]
pub struct NewUrlRequest {
    pub original_url: String,
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_url);
    cfg.service(list_urls);
    cfg.service(redirect_url);
}
