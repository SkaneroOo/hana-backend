use actix_files::{
    Files, 
    NamedFile
};
use actix_web::{
    head,
    http::header::ContentType,
    HttpResponse,
    get,
    post,
    Responder,
    web::{
        self,
        ServiceConfig
    }
};
use futures::StreamExt;
use libsql_client::Client;
use serde::{
    Deserialize,
    Serialize
};
use shuttle_actix_web::ShuttleActixWeb;
use shuttle_secrets::SecretStore;



#[derive(Serialize, Deserialize)]
struct LoginData {
    username: String
}

#[get("/index")]
async fn index() -> impl Responder {
    NamedFile::open_async("static/index.html").await
}

#[head("/")]
async fn uptime() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[post("/login")]
async fn login(mut body: web::Payload) -> HttpResponse {

    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.unwrap());
    }

    let data: LoginData = serde_json::from_slice(&bytes).unwrap();

    if data.username == "hoge" {
        return HttpResponse::Unauthorized().finish();
    }

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .append_header(("Access-Control-Allow-Origin", "*"))
        .body(format!(r#"{{"username": "{}"}}"#, data.username))
}


// #[derive(Clone)]
// struct AppState {
//     database: Arc<Mutex<Client>>,
// }

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn actix_web(
    #[shuttle_secrets::Secrets] _secrets: SecretStore,
    #[shuttle_turso::Turso(
        addr="libsql://wanted-dragon-man-skanerooo.turso.io",
        token="{secrets.DB_TURSO_TOKEN}")] client: Client,
) -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    // let app_state = AppState {
    //     database: Arc::new(Mutex::new(client))
    // };

    let data = web::Data::new(client);
    let config = move |cfg: &mut ServiceConfig| {

        cfg.app_data(data);
        cfg.service(index);
<<<<<<< HEAD
        cfg.service(uptime);
=======
        cfg.service(login);
>>>>>>> 19e5c779ca5c0d7c687af9be484c8d38bb5d9948
        cfg.service(Files::new("/", "static")
            .show_files_listing()
            .index_file("index.html")
            .use_last_modified(true),
        );
    };

    

    Ok(config.into())
}
