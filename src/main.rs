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
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_diesel::*;

pub mod models;
pub mod schema;

use models::*;

const CACHE_SIZE: usize = 1024;

#[derive(Clone)]
struct ServerConfig {
    url: String,
}

#[derive(Clone)]
struct Cache {
    data: Vec<Link>,
}

impl Cache {
    pub fn new() -> Cache {
        let data = vec![Link::default(); CACHE_SIZE];
        Cache { data }
    }

    pub fn get<'a>(&'a self, k: &'a str) -> Option<&'a Link> {
        let mut hasher = DefaultHasher::new();
        k.hash(&mut hasher);
        let vec_k = hasher.finish() as usize % CACHE_SIZE;
        let v = &self.data[vec_k];
        if v.id == k {
            Some(v)
        } else {
            None
        }
    }

    pub fn insert(&mut self, data: Link) {
        let mut hasher = DefaultHasher::new();
        data.id.hash(&mut hasher);
        let vec_k = hasher.finish() as usize % CACHE_SIZE;
        self.data[vec_k] = data;
    }
}

pub fn establish_connection() -> diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create pool");
    return pool;
}

async fn create_shortened_link(
    config: web::Data<ServerConfig>,
    db: web::Data<Pool<diesel::r2d2::ConnectionManager<PgConnection>>>,
    params: Json<LinkCreate>,
) -> impl Responder {
    use schema::links::dsl::*;
    #[derive(Serialize)]
    struct Response {
        url: String,
    };
    let mut url = config.url.clone();
    let result: Link = diesel::insert_into(links)
        .values(params.into_inner())
        .get_result_async(&db)
        .await
        .expect("failed to insert ");

    url.push_str(&result.id);

    let response = Response { url };
    HttpResponse::Ok().body(serde_json::to_string(&response).expect("failed to serialize"))
}

async fn get_all_links(
    db: web::Data<Pool<diesel::r2d2::ConnectionManager<PgConnection>>>,
) -> impl Responder {
    use schema::links::dsl::*;
    let results = links
        .load_async::<Link>(&db)
        .await
        .expect("failed to get posts");
    HttpResponse::Ok().body(serde_json::to_string(&results).expect("failed to serialize"))
}

async fn redirect(
    cache: web::Data<Arc<RwLock<Cache>>>,
    db: web::Data<Pool<diesel::r2d2::ConnectionManager<PgConnection>>>,
    path: web::Path<String>,
) -> impl Responder {
    use schema::links::dsl::*;
    let path_str = path.into_inner();

    {
        let lock = cache.read().await;
        let result = lock.get(&path_str);
        if let Some(val) = result {
            println!("cache hit {:?}", result);
            return HttpResponse::PermanentRedirect()
                .set_header("Location", val.dest_url.clone())
                .finish();
        }
    }

    let result: Link = links.find(path_str.clone()).first_async(&db).await.unwrap();
    {
        println!("cache miss");
        let mut lock = cache.write().await;
        lock.insert(result.clone());
    }
    HttpResponse::PermanentRedirect()
        .set_header("Location", result.dest_url)
        .finish()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let pool = establish_connection();

    let cache = Arc::new(RwLock::new(Cache::new()));

    let url = env::var("SERVER_URL").expect("SERVER_URL must be set");

    let config = ServerConfig { url };

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .data(config.clone())
            .data(cache.clone())
            .route("/", web::post().to(create_shortened_link))
            .route("/", web::get().to(get_all_links))
            .route("/{id}", web::get().to(redirect))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
