use actix_files::{Files, NamedFile};
use actix_web::{get, web::ServiceConfig, Responder, head, HttpResponse};
use shuttle_actix_web::ShuttleActixWeb;

#[get("/index")]
async fn index() -> impl Responder {
    NamedFile::open_async("static/index.html").await
}

#[head("/")]
async fn uptime() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn actix_web() -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    let config = move |cfg: &mut ServiceConfig| {
        cfg.service(index);
        cfg.service(uptime);
        cfg.service(Files::new("/", "static")
            .show_files_listing()
            .index_file("index.html")
            .use_last_modified(true),
        );
    };

    Ok(config.into())
}
