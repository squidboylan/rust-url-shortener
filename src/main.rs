#[macro_use]
extern crate diesel;

#[macro_use]
extern crate serde_derive;

use actix_web::web::Json;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use std::env;
use tokio_diesel::*;

pub mod models;
pub mod schema;

#[derive(Clone)]
struct ServerConfig {
    url: String,
}

pub fn establish_connection() -> diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create pool");
    return pool;
}

async fn create_shortened_link(config: web::Data<ServerConfig>, db: web::Data<Pool<diesel::r2d2::ConnectionManager<PgConnection>>>, params: Json<models::LinkCreate>) -> impl Responder {
    use schema::links::dsl::*;
    #[derive(Serialize)]
    struct Response {
        url: String,
    };
    let mut url = config.url.clone();
    let result: models::Link = diesel::insert_into(links).values(params.into_inner()).get_result_async(&db).await.expect("failed to insert ");

    url.push_str(&result.id);

    let response = Response {
        url,
    };
    HttpResponse::Ok().body(serde_json::to_string(&response).expect("failed to serialize"))
}

async fn get_all_links(db: web::Data<Pool<diesel::r2d2::ConnectionManager<PgConnection>>>) -> impl Responder {
    use schema::links::dsl::*;
    let results = links.load_async::<models::Link>(&db).await.expect("failed to get posts");
    HttpResponse::Ok().body(serde_json::to_string(&results).expect("failed to serialize"))
}

async fn redirect(db: web::Data<Pool<diesel::r2d2::ConnectionManager<PgConnection>>>, path: web::Path<String>) -> impl Responder {
    use schema::links::dsl::*;
    let path_str = path.into_inner();
    let result: models::Link = links.find(path_str.clone()).first_async(&db).await.unwrap();
    HttpResponse::PermanentRedirect().set_header("Location", result.dest_url).finish()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let pool = establish_connection();

    let url = env::var("SERVER_URL").expect("SERVER_URL must be set");

    let config = ServerConfig {
        url,
    };

    HttpServer::new(move ||
        App::new()
            .data(pool.clone())
            .data(config.clone())
            .route("/", web::post().to(create_shortened_link))
            .route("/", web::get().to(get_all_links))
            .route("/{id}", web::get().to(redirect)))
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
