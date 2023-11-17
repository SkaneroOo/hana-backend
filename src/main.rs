use actix_files::{
    Files, 
    NamedFile
};
use actix_web::{
    cookie::{Cookie, time::Duration},
    head,
    http::header::ContentType,
    HttpRequest,
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
use libsql_client::{
    // args,
    Client
};
use reqwest;
use serde::{
    Deserialize,
    Serialize
};
use serde_json::Value;
use shuttle_actix_web::ShuttleActixWeb;
use shuttle_secrets::SecretStore;
use std::collections::HashMap;

// temp discord oauth url https://discord.com/api/oauth2/authorize?client_id=1170384464476639272&redirect_uri=https%3A%2F%2Ffoxhound-sincere-rarely.ngrok-free.app%2Foauth&response_type=code&scope=identify%20email

#[derive(Serialize, Deserialize)]
struct LoginData {
    username: String
}


#[head("/")]
async fn uptime(req: HttpRequest) -> HttpResponse {
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

#[derive(Deserialize, Debug)]
struct Oauth2Data {
    code: String,
    _state: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: String,
    scope: String
}

#[get("/oauth")]
async fn oauth2_redirect(data: web::Query<Oauth2Data>, secrets: web::Data<SecretStore>) -> HttpResponse {

    println!("oauth2_redirect: {data:?}");
    let client = reqwest::Client::new();
    let mut form_data = HashMap::new();
    form_data.insert("grant_type", "authorization_code");
    form_data.insert("code", &data.code);
    form_data.insert("redirect_uri", "https://c6b1-46-204-108-193.ngrok-free.app/oauth");
    let recieved = client.post("https://discord.com/api/v10/oauth2/token")
          .form(&form_data)
          .basic_auth(secrets.get("DISCORD_CLIENT_ID").expect("missing DISCORD_CLIENT_ID"), secrets.get("DISCORD_APP_SECRET"))
          .send().await.expect("failed to send request").json::<AccessTokenResponse>().await.expect("failed to parse response json");
    println!("recieved: {recieved:?}");

    let token_cookie = Cookie::build("access_token", recieved.access_token)
                                   .max_age(Duration::seconds(recieved.expires_in)).finish();
    let refresh_cookie = Cookie::build("refresh_token", recieved.refresh_token).permanent().finish();
    

    HttpResponse::Ok()
        .cookie(token_cookie)
        .cookie(refresh_cookie)
        .finish()
}

// #[derive(Clone)]
// struct AppState {
//     database: Arc<Mutex<Client>>,
// }

async fn setup(database: &Client) {
    let tx = database.transaction().await.expect("Cannot create transaction");

    tx.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT, email TEXT, discord_id TEXT)").await.unwrap();

    tx.commit().await.expect("Cannot commit transaction");
}

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn actix_web(
    #[shuttle_secrets::Secrets] secrets: SecretStore,
    #[shuttle_turso::Turso(
        addr="{secrets.DB_TURSO_URL}",
        token="{secrets.DB_TURSO_TOKEN}")] client: Client,
) -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    // let app_state = AppState {
    //     database: Arc::new(Mutex::new(client))
    // };
    
    setup(&client).await;

    let data = web::Data::new(client);
    let secrets_data = web::Data::new(secrets);
    let config = move |cfg: &mut ServiceConfig| {

        cfg.app_data(data);
        cfg.app_data(secrets_data);
        cfg.service(uptime);
        cfg.service(login);
        cfg.service(oauth2_redirect);
        cfg.service(Files::new("/", "static")
            .show_files_listing()
            .index_file("index.html")
            .use_last_modified(true),
        );
    };

    

    Ok(config.into())
}
