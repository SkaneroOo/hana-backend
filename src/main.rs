#![allow(clippy::future_not_send)]

use actix_files::Files;
use actix_web::{
    App,
    cookie::{
        Cookie, 
        time::Duration
    },
    head,
    HttpServer,
    HttpRequest,
    HttpResponse,
    get,
    web
};
use dotenv::dotenv;
use libsql_client::{
    Client, 
    Config
};
use serde::{
    Deserialize,
    Serialize
};
#[allow(unused_imports)]
use serde_json::{
    to_string,
    Value
};
use std::{
    collections::HashMap,
    env, 
    sync::Mutex
};

// temp discord oauth url https://discord.com/api/oauth2/authorize?client_id=1170384464476639272&redirect_uri=https%3A%2F%2Ffoxhound-sincere-rarely.ngrok-free.app%2Foauth&response_type=code&scope=identify%20email

#[derive(Serialize, Deserialize)]
struct LoginData {
    username: String
}


#[head("/")]
async fn uptime(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[get("/login")]
async fn login(req: HttpRequest) -> HttpResponse {

    HttpResponse::PermanentRedirect()
    .append_header(("Location", format!("https://discord.com/api/oauth2/authorize?client_id=1170384464476639272&redirect_uri=https%3A%2F%2F{}%2Foauth&response_type=code&scope=identify%20email", req.connection_info().host())))
    .finish()
}

#[derive(Deserialize, Debug)]
struct Oauth2Data {
    code: String,
    #[allow(dead_code)]
    state: Option<String>
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
async fn oauth2_redirect(req: HttpRequest, data: web::Query<Oauth2Data>, secrets: web::Data<Secrets>) -> HttpResponse {
    dbg!(req.connection_info().host());
    println!("oauth2_redirect: {data:?}");
    let client = reqwest::Client::new();
    let mut form_data = HashMap::new();
    form_data.insert("grant_type", "authorization_code");
    form_data.insert("code", &data.code);
    let host = format!("https://{}/oauth", req.connection_info().host());
    form_data.insert("redirect_uri", &host);
    let recieved = client.post("https://discord.com/api/v10/oauth2/token")
          .form(&form_data)
          .basic_auth(&secrets.discord_client_id, Some(&secrets.discord_app_secret))
          .send().await.expect("failed to send request").json::<AccessTokenResponse>().await.expect("unable to parse response");

    println!("recieved: {recieved:?}");

    let token_cookie = Cookie::build("access_token", recieved.access_token)
                                   .max_age(Duration::seconds(recieved.expires_in)).finish();
    let refresh_cookie = Cookie::build("refresh_token", recieved.refresh_token).permanent().finish();
    
    // HttpResponse::TemporaryRedirect()

    HttpResponse::PermanentRedirect()
        .append_header(("Location", "/"))
        .cookie(token_cookie)
        .cookie(refresh_cookie)
        .finish()
}

#[derive(Debug, Serialize, Deserialize)]
struct UserData {
    id: String,
    #[serde(rename(deserialize = "global_name"))]
    username: String,
    avatar: String
}

#[derive(Debug, Serialize, Deserialize)]
struct UserDataResponse {
    status: String,
    message: Option<String>,
    user: Option<UserData>
}

#[derive(Debug, Deserialize, Serialize)]
struct AuthorizationInformation {
    // application: Value,
    // scopes: Vec<String>,
    // expires: String,
    user: UserData
}


#[get("/get-user")]
async fn get_user(req: HttpRequest, secrets: web::Data<Secrets>) -> HttpResponse {

    let mut response = HttpResponse::Ok();
    response.append_header(("Access-Control-Allow-Origin", "*"));

    let client = reqwest::Client::new();

    let mut cookies = vec![];

    let user_token = match req.cookie("access_token") {
        Some(cookie) => cookie,
        None => {
            #[allow(clippy::single_match_else)]
            match req.cookie("refresh_token") {
                Some(cookie) => {
                    let mut form_data = HashMap::new();
                    form_data.insert("grant_type", "refresh_token");
                    form_data.insert("refresh_token", cookie.value());
                    
                    let recieved = client.post("https://discord.com/api/v10/oauth2/token")
                        .form(&form_data)
                        .basic_auth(&secrets.discord_client_id, Some(&secrets.discord_app_secret))
                        .send().await.expect("failed to send request").json::<AccessTokenResponse>().await.expect("unable to parse response");
                    
                    let token_cookie = Cookie::build("access_token", recieved.access_token)
                                                    .max_age(Duration::seconds(recieved.expires_in)).finish();
                    let refresh_cookie = Cookie::build("refresh_token", recieved.refresh_token).permanent().finish();

                    cookies.push(token_cookie.clone());
                    cookies.push(refresh_cookie);

                    token_cookie
                },
                None => {
                    let response_body = UserDataResponse {
                        status: "Error".to_string(),
                        message: Some("User not logged in".to_string()),
                        user: None
                    };
                    // return response.body(to_string(&response_body).unwrap_or_else(|_| unreachable!()))
                    return response.json(response_body)
                }
            }
        }
    };

    let data = client.get("https://discord.com/api/v10/oauth2/@me")
        .header("Authorization", format!("Bearer {}", user_token.value()))
        .send().await.expect("cannot send request").json::<AuthorizationInformation>().await.expect("cannot parse recieved data");

    println!("{data:?}");

    for cookie in cookies {
        response.cookie(cookie);
    }

    let resp_data = UserDataResponse {
        status: "Ok".to_string(),
        message: None,
        user: Some(data.user)
    };
    response.json(resp_data)
    // response.body(to_string(&resp_data).unwrap_or_else(|_| unreachable!()))
}

// #[derive(Clone)]
// struct AppState {
//     database: Arc<Mutex<Client>>,
// }

async fn setup(database: &Client) {
    let tx = database.transaction().await.expect("Cannot create transaction");

    tx.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT, discord_id TEXT)").await.expect("Cannot create table");

    tx.commit().await.expect("Cannot commit transaction");
}

struct Secrets {
    db_url: String,
    db_token: String,
    discord_client_id: String,
    discord_app_secret: String
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error>{

    dotenv().ok();

    let secrets = Secrets {
        db_url: env::var("DB_URL").expect("missing DB_URL"),
        db_token: env::var("DB_TOKEN").expect("missing DB_TOKEN"),
        discord_client_id: env::var("DISCORD_CLIENT_ID").expect("missing DISCORD_CLIENT_ID"),
        discord_app_secret: env::var("DISCORD_APP_SECRET").expect("missing DISCORD_APP_SECRET")
    };

    let client = Client::from_config(Config {
        url: url::Url::parse(&secrets.db_url).expect("Cannot parse database url"),
        auth_token: Some(secrets.db_token.to_string())
    }).await.expect("Cannot create database client");
    
    setup(&client).await;

    let data = web::Data::new(Mutex::new(client));
    let secrets_data = web::Data::new(Mutex::new(secrets));


    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::clone(&data))
            .app_data(web::Data::clone(&secrets_data))
            .service(uptime)
            .service(login)
            .service(oauth2_redirect)
            .service(get_user)
            .service(Files::new("/", "static")
                .show_files_listing()
                .index_file("index.html")
                .use_last_modified(true),
            )
            .service(Files::new("/css", "static/css"))
    })
    .bind(("127.0.0.1", env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse::<u16>().unwrap_or(8080)))?
    .run()
    .await
}
