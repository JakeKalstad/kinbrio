use std::str::FromStr;
use std::time::SystemTime;

use askama::Template;
use chrono::DateTime;
use chrono::Utc;
use jsonwebtoken::TokenData;
use matrix_sdk::ruma::device_id;
use matrix_sdk::ruma::OwnedUserId;
use matrix_sdk::Session;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use uuid::Uuid;

use tide::http::cookies::Cookie;
use tide::Response;
use tide::StatusCode;
use tide::{http::mime, Request};

use crate::home::NotFoundTemplate;
use crate::matrix;
use crate::matrix::Choice;
use crate::organization;
use crate::organization::Organization;
use crate::State;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

#[derive(Serialize, Deserialize, Clone)]
pub struct UserJwtState {
    pub key: String,
    pub organization_key: String,
    pub email: String,
    pub created: i64,

    pub matrix_user_id: String,
    pub matrix_access_token: String,
    pub matrix_device_id: String,
    pub matrix_refresh_token: String,
    pub matrix_home_server: String,

    pub updated: i64,
    exp: i64,
}

pub async fn get_users_by_organization(
    conn: &mut PoolConnection<Postgres>,
    organization_key: uuid::Uuid,
) -> Vec<User> {
    let user_records = sqlx::query!(
        "select key, organization_key, email,  matrix_user_id, matrix_home_server, created, updated from users where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select user by email");
    let mut users = vec![];
    for user in user_records {
        users.push(User {
            key: user.key.expect("key exists"),
            organization_key: user.organization_key.expect("key exists"),
            email: user.email.expect("email exists"),
            matrix_user_id: user.matrix_user_id.expect("matrix_user_id exists"),
            matrix_home_server: user.matrix_home_server.expect("matrix_home_server exists"),
            created: user.created.expect("created exists"),
            updated: user.updated.expect("updated exists"),
        })
    }
    users
}

pub async fn get_user(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Option<User> {
    match sqlx::query!(
        "select key, organization_key, email, matrix_user_id, matrix_home_server, created, updated from users where key = $1",
        key
    )
    .fetch_one(conn)
    .await {
        Ok(user) => Some(User {
            key: user.key.expect("key exists"),
            organization_key: user.organization_key.expect("key exists"),
            email: user.email.expect("email exists"),
            matrix_user_id: user.matrix_user_id.expect("matrix_user_id exists"),
            matrix_home_server: user.matrix_home_server.expect("matrix_home_server exists"),
            created: user.created.expect("created exists"),
            updated: user.updated.expect("updated exists"),
        }),
        Err(_) => { None }
    }
}

pub async fn get_user_by_matrix_user_id(
    conn: &mut PoolConnection<Postgres>,
    matrix_user_id: String,
) -> Option<User> {
    match sqlx::query!(
        "select key, organization_key, email, matrix_user_id, matrix_home_server, created, updated from users where matrix_user_id = $1",
        &matrix_user_id
    )
    .fetch_one(conn)
    .await {
        Ok(user) => {
            Some(User {
                key: user.key.expect("key exists"),
                organization_key: user.organization_key.expect("key exists"),
                email: user.email.expect("email exists"),
                matrix_user_id: user.matrix_user_id.expect("matrix_user_id exists"),
                matrix_home_server: user.matrix_home_server.expect("matrix_home_server exists"),
                created: user.created.expect("created exists"),
                updated: user.updated.expect("updated exists"),
            })
        }, Err(_) => {
            None
        }
    }
}

async fn delete_user(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) {
    sqlx::query!("DELETE FROM users where key=$1", key)
        .execute(conn)
        .await
        .expect("Insert Success");
}

async fn insert_user(conn: &mut PoolConnection<Postgres>, new_user: &User) {
    sqlx::query!("INSERT INTO users (key, organization_key, email, matrix_user_id, matrix_home_server, created, updated) values($1, $2, $3, $4, $5, $6, $7)", new_user.key, new_user.organization_key, &new_user.email, &new_user.matrix_user_id, &new_user.matrix_home_server, new_user.created, new_user.updated)
    .execute(conn)
    .await
    .expect("Insert Success");
}
async fn update_user(conn: &mut PoolConnection<Postgres>, user: &User) {
    sqlx::query!("UPDATE users SET organization_key=$2, email=$3, matrix_user_id=$4, matrix_home_server=$5, updated=extract(epoch from now()) where key = $1", user.key, user.organization_key, &user.email, &user.matrix_user_id, &user.matrix_home_server)
    .execute(conn)
    .await
    .expect("Insert Success");
}

// JWT AUTHENTICATION STUFF
pub fn jwt_valid(jwt: String) -> Result<TokenData<UserJwtState>, jsonwebtoken::errors::Error> {
    let token_part = jwt.strip_prefix("token=").expect("token part removed");
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("Missing `JWT_SECRET` env variable, needed for running the server");
    return decode::<UserJwtState>(
        &token_part,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS512),
    )
    .map_err(|e| {
        println!("{}", e);
        return e;
    });
}

pub fn create_jwt(
    u: &User,
    matrix_access_token: String,
    matrix_device_id: String,
    matrix_refresh_token: String,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(2))
        .expect("valid timestamp")
        .timestamp();
    let jwt_state = UserJwtState {
        key: u.key.to_string(),
        organization_key: u.organization_key.to_string(),
        email: u.email.to_string(),
        matrix_user_id: u.matrix_user_id.clone(),
        matrix_home_server: u.matrix_home_server.clone(),
        matrix_access_token: matrix_access_token,
        matrix_device_id: matrix_device_id,
        matrix_refresh_token: matrix_refresh_token,
        created: u.created,
        updated: u.updated,
        exp: expiration,
    };
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("Missing `JWT_SECRET` env variable, needed for running the server");
    let header = Header::new(Algorithm::HS512);
    encode(
        &header,
        &jwt_state,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| e)
}

pub fn read_jwt_cookie(cookie: Option<Cookie<'static>>) -> Option<UserJwtState> {
    let cookie_val = match cookie {
        Some(c) => c,
        None => return None,
    };

    let valid = match jwt_valid(cookie_val.to_string()) {
        Ok(t) => t,
        Err(_e) => return None,
    };
    return Some(valid.claims);
}

pub fn read_jwt_cookie_to_user(cookie: Option<Cookie<'static>>) -> Option<User> {
    let cookie_val = match cookie {
        Some(c) => c,
        None => return None,
    };

    let valid = match jwt_valid(cookie_val.to_string()) {
        Ok(t) => t,
        Err(_e) => return None,
    };
    let u = User::new(
        uuid::Uuid::from_str(valid.claims.key.as_str()).expect("parse cookie"),
        uuid::Uuid::from_str(&valid.claims.organization_key)
            .expect("parse organization key from jwt"),
        valid.claims.email,
        valid.claims.matrix_user_id,
        valid.claims.matrix_home_server,
    );

    return Some(u);
}
// Route Stuff
pub async fn delete(req: Request<State>) -> tide::Result {
    if let Some(value) = jwt_invalid(&req) {
        return value;
    }

    match req.param("user_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let u_uuid = uuid::Uuid::from_str(key).expect("User uuid parse");
            delete_user(&mut conn, u_uuid).await;
            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .build())
        }
        Err(e) => {
            println!("{:?}", e);
            Ok(tide::Response::builder(tide::StatusCode::NotFound)
                .content_type(mime::HTML)
                .body(NotFoundTemplate::new().render_string())
                .build())
        }
    }
}

pub fn user_jwt_state_invalid(claims: UserJwtState) -> Option<Result<Response, tide::Error>> {
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Some(Ok(tide::Response::builder(tide::StatusCode::Unauthorized)
            .content_type(mime::PLAIN)
            .body("EXPIRED")
            .build()));
    }
    None
}

pub fn jwt_invalid(req: &Request<State>) -> Option<Result<Response, tide::Error>> {
    let claims = match read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Some(Ok(tide::Response::builder(tide::StatusCode::Unauthorized)
                .content_type(mime::PLAIN)
                .body("UNAUTHORIZED")
                .build()))
        }
    };
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Some(Ok(tide::Response::builder(tide::StatusCode::Unauthorized)
            .content_type(mime::PLAIN)
            .body("EXPIRED")
            .build()));
    }
    None
}

pub fn user_or_error(req: &Request<State>) -> Result<User, Result<tide::Response, tide::Error>> {
    let claims = match read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Err(Ok(tide::Response::builder(tide::StatusCode::Unauthorized)
                .content_type(mime::PLAIN)
                .body("UNAUTHORIZED")
                .build()))
        }
    };
    if let Some(value) = user_jwt_state_invalid(claims.clone()) {
        return Err(value);
    }
    let u = User {
        key: uuid::Uuid::from_str(claims.key.as_str()).expect("key parses"),
        organization_key: uuid::Uuid::from_str(claims.organization_key.as_str())
            .expect("key parses"),
        email: claims.email,
        matrix_user_id: claims.matrix_user_id,
        matrix_home_server: claims.matrix_home_server,
        created: claims.created,
        updated: claims.updated,
    };
    Ok(u)
}

pub async fn get(req: Request<State>) -> tide::Result {
    if let Some(value) = jwt_invalid(&req) {
        return value;
    }

    match req.param("user_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let u_uuid = uuid::Uuid::from_str(key).expect("User uuid parse");
            let user = get_user(&mut conn, u_uuid).await.expect("user exists");

            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(UserTemplate::new(&user, user.email.as_str()).render_string())
                .build())
        }
        Err(e) => {
            println!("{:?}", e);
            Ok(tide::Response::builder(tide::StatusCode::NotFound)
                .content_type(mime::HTML)
                .body(NotFoundTemplate::new().render_string())
                .build())
        }
    }
}

pub async fn register(_req: Request<State>) -> tide::Result {
    let auth = "matrix";
    if auth == "matrix" {
        let homeserver_url = "https://matrix-client.matrix.org";
        let choices = matrix::get_login_urls(
            homeserver_url.to_string(),
            "http://localhost:8080/register_matrix".to_string(),
        )
        .await
        .expect("Get login choices");
        let register = RegisterTemplate::new(choices);
        return Ok(tide::Response::builder(tide::StatusCode::Ok)
            .content_type(mime::HTML)
            .body(register.render_string())
            .build());
    }
    let register = RegisterTemplate::new(vec![]);
    Ok(tide::Response::builder(tide::StatusCode::Ok)
        .content_type(mime::HTML)
        .body(register.render_string())
        .build())
}

pub async fn update(mut req: Request<State>) -> tide::Result {
    let _claims: UserJwtState = match read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };
    let umd: Result<User, tide::Error> = req.body_json().await;
    match umd {
        Ok(u) => {
            let mut conn = match req.state().db_pool.acquire().await {
                Ok(c) => c,
                Err(e) => {
                    return Ok(
                        tide::Response::builder(tide::StatusCode::InternalServerError)
                            .content_type(mime::PLAIN)
                            .body(e.to_string())
                            .build(),
                    )
                }
            };

            update_user(&mut conn, &u).await;
            let j = serde_json::to_string(&u).expect("To JSON");
            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::JSON)
                .body(j)
                .build())
        }
        Err(e) => {
            println!("{:?}", e);
            Ok(tide::Response::builder(tide::StatusCode::BadRequest)
                .content_type(mime::JSON)
                .body("{'error': 'invalid json body'}")
                .build())
        }
    }
}

pub async fn register_post(req: Request<State>) -> tide::Result {
    let vals = req.url().query().expect("Get query values");
    let login_token = vals.split("loginToken=").last().expect("Token found");
    let homeserver_url = "https://matrix-client.matrix.org".to_string();
    let matrix_login_response =
        crate::matrix::login_with_token(homeserver_url.clone(), login_token.to_string()).await;

    match matrix_login_response {
        Ok(u) => {
            let mut conn = match req.state().db_pool.acquire().await {
                Ok(c) => c,
                Err(e) => {
                    return Ok(
                        tide::Response::builder(tide::StatusCode::InternalServerError)
                            .content_type(mime::PLAIN)
                            .body(e.to_string())
                            .build(),
                    )
                }
            };
            let (email, avatar, display_name)  = crate::matrix::account(homeserver_url.clone(), login_token.to_string(), u.user_id.clone()).await;
            let organization_key = Uuid::new_v4();
            let key = Uuid::new_v4();
            let new_user = User::new(
                key,
                organization_key,
                email,
                u.user_id.to_string(),
                u.home_server.expect("Get home server").to_string(),
            );

            organization::insert_organization(
                &mut conn,
                &Organization::new(
                    organization_key,
                    "".to_string(),
                    "".to_string(),
                    new_user.key,
                    "Welcome Inc.".to_string(),
                    "".to_string(),
                    homeserver_url,
                    "".to_string(),
                    "".to_string(),
                    "".to_string(),
                    "".to_string(),
                ),
            )
            .await;

            let mut conn = match req.state().db_pool.acquire().await {
                Ok(c) => c,
                Err(e) => {
                    return Ok(
                        tide::Response::builder(tide::StatusCode::InternalServerError)
                            .content_type(mime::PLAIN)
                            .body(e.to_string())
                            .build(),
                    )
                }
            };

            insert_user(&mut conn, &new_user).await;
            let mut res = Response::new(StatusCode::TemporaryRedirect);
            let now = SystemTime::now();
            let cadd = now
                .checked_add(u.expires_in.unwrap_or_default())
                .expect("added");
            let datetime: DateTime<Utc> = cadd.into();

            let jwt = create_jwt(&new_user, "".to_string(), "".to_string(), "".to_string())
                .expect("JWT Created");

            let cookie_str = format!("token={}; Expires={}", jwt, datetime.to_string());
            // let expire_time = "Wed, 21 Oct 2017 07:28:00 GMT";
            let c = Cookie::parse(cookie_str).unwrap();
            res.insert_cookie(c);
            res.insert_header("Location", "/dashboard");
            Ok(res)
        }
        Err(e) => {
            println!("{:?}", e);
            Ok(tide::Response::builder(tide::StatusCode::BadRequest)
                .content_type(mime::JSON)
                .body("{'error': 'invalid json body'}")
                .build())
        }
    }
}

pub async fn logout(_req: Request<State>) -> tide::Result {
    let mut res = Response::new(StatusCode::TemporaryRedirect);
    let cookie_str = format!("token={};", "");
    // let expire_time = "Wed, 21 Oct 2017 07:28:00 GMT";
    let c = Cookie::parse(cookie_str).unwrap();
    res.remove_cookie(c);
    res.insert_header("Location", "/");
    Ok(res)
}

pub async fn login_by_username(req: Request<State>) -> tide::Result {
    let homeserver_url = "https://matrix-client.matrix.org";
    let claims: crate::user::UserJwtState = match crate::user::read_jwt_cookie(req.cookie("token"))
    {
        Some(c) => c,
        None => {
            // JWT probably expired
            let login = LoginUsernameTemplate::new();
            return Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(login.render_string())
                .build());
        }
    };
    // lets refresh and go
    // fresh login
    let login = LoginUsernameTemplate::new();
    return Ok(tide::Response::builder(tide::StatusCode::Ok)
        .content_type(mime::HTML)
        .body(login.render_string())
        .build());
}

pub async fn login(req: Request<State>) -> tide::Result {
    let homeserver_url = "https://matrix-client.matrix.org";
    let claims: crate::user::UserJwtState = match crate::user::read_jwt_cookie(req.cookie("token"))
    {
        Some(c) => c,
        None => {
            // JWT probably expired
            let choices = matrix::get_login_urls(
                homeserver_url.to_string(),
                "http://localhost:8080/login_matrix".to_string(),
            )
            .await
            .expect("Get login choices");
            let login = LoginTemplate::new(choices);
            return Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(login.render_string())
                .build());
        }
    };
    // lets refresh and go
    if claims.matrix_refresh_token.len() > 0 {
        let user_id =
            <OwnedUserId>::try_from(claims.matrix_user_id.as_str()).expect("parse user id");
        let session = Session {
            access_token: claims.matrix_access_token,
            refresh_token: Some(claims.matrix_refresh_token),
            user_id,
            device_id: device_id!("kinbrio").to_owned(),
        };
        crate::matrix::restore_from_session(claims.matrix_home_server, session)
            .await
            .expect("Restoring from session");
        return Ok(tide::Redirect::new("/").into());
    } else {
        // fresh login
        let choices = matrix::get_login_urls(
            homeserver_url.to_string(),
            "http://localhost:8080/login_matrix".to_string(),
        )
        .await
        .expect("Get login choices");
        let login = LoginTemplate::new(choices);
        return Ok(tide::Response::builder(tide::StatusCode::Ok)
            .content_type(mime::HTML)
            .body(login.render_string())
            .build());
    }
}

pub async fn login_matrix(req: Request<State>) -> tide::Result {
    let vals = req.url().query().expect("Get query values");
    let login_token = vals.split("loginToken=").last().expect("Token found");
    let matrix_login_response = match crate::matrix::login_with_token(
        "https://matrix-client.matrix.org".to_string(),
        login_token.to_string(),
    )
    .await
    {
        Ok(mls) => mls,
        Err(_e) => {
            return Ok(tide::Redirect::new("/register").into());
        }
    };
    let mut conn = match req.state().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            return Ok(
                tide::Response::builder(tide::StatusCode::InternalServerError)
                    .content_type(mime::PLAIN)
                    .body(e.to_string())
                    .build(),
            )
        }
    };
    let matrix_user_id = matrix_login_response.user_id.to_string();
    let matrix_access_token = matrix_login_response.access_token;
    let matrix_device_id = matrix_login_response.device_id.to_string();
    let matrix_refresh_token = matrix_login_response.refresh_token.unwrap_or_default();
    let matrix_expires_in = matrix_login_response.expires_in.unwrap_or_default();
    let matrix_home_server = matrix_login_response
        .home_server
        .as_ref()
        .expect("Get home serve");

    // TODO: add access token/refresh/etc to redis with the matrix_expires_in
    let user = match get_user_by_matrix_user_id(&mut conn, matrix_user_id.clone()).await {
        Some(u) => u,
        None => {
            return Ok(tide::Redirect::new("/register").into());
        }
    };

    let mut res = Response::new(StatusCode::TemporaryRedirect);

    let user_t = User {
        key: user.key,
        organization_key: user.organization_key,
        email: user.email,

        matrix_user_id: matrix_user_id.clone(),
        matrix_home_server: matrix_home_server.to_string().clone(),

        created: user.created,
        updated: user.updated,
    };
    let jwt = create_jwt(
        &user_t,
        matrix_access_token,
        matrix_device_id,
        matrix_refresh_token.to_string(),
    )
    .expect("JWT Created");
    let now = SystemTime::now();
    let cadd = now.checked_add(matrix_expires_in).expect("added");
    let datetime: DateTime<Utc> = cadd.into();

    let cookie_str = format!("token={}; Expires={}", jwt, datetime.to_string());
    let c = Cookie::parse(cookie_str).unwrap();
    res.insert_cookie(c);
    res.insert_header("Location", "/dashboard");
    Ok(res)
}

pub async fn login_post(mut req: Request<State>) -> tide::Result {
    let u: LoginPostRequest = match req.body_json().await {
        Ok(pr) => pr,
        Err(e) => {
            return Ok(tide::Response::builder(tide::StatusCode::BadRequest)
                .content_type(mime::PLAIN)
                .body(e.to_string())
                .build())
        }
    };

    let mut conn = match req.state().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            return Ok(
                tide::Response::builder(tide::StatusCode::InternalServerError)
                    .content_type(mime::PLAIN)
                    .body(e.to_string())
                    .build(),
            )
        }
    };
    let homeserver = u.homeserver.clone();
    let matrix_login_response = match crate::matrix::login_with_password(
        homeserver.clone(),
        u.uid.to_string(),
        u.secret.to_string(),
    )
    .await
    {
        Ok(v) => v,
        Err(_) => {
            return Ok(tide::Redirect::new("/login_by_username").into());
        }
    };
    if matrix_login_response.access_token.len() == 0 {
        return Ok(
            tide::Response::builder(tide::StatusCode::InternalServerError)
                .content_type(mime::PLAIN)
                .body("Unexpected errors from matrix loggin - no valid access token returned")
                .build(),
        );
    }
    let (email, avatar, display_name)  = crate::matrix::account(homeserver.clone(), matrix_login_response.access_token.clone(), matrix_login_response.user_id.clone()).await;
    
    let user = match get_user_by_matrix_user_id(&mut conn, matrix_login_response.user_id.to_string())
    .await {
        Some(u) => u,
        None => {
           
            let organization_key = Uuid::new_v4();
            let key = Uuid::new_v4();
            let new_user: User = User::new(
                key,
                organization_key,
                email,
                matrix_login_response.user_id.to_string(),
                homeserver.clone(),
            );
            
            organization::insert_organization(
                &mut conn,
                &Organization::new(
                    organization_key,
                    "".to_string(),
                    "".to_string(),
                    new_user.key,
                    "Welcome Inc.".to_string(),
                    "".to_string(),
                    homeserver.clone(),
                    "".to_string(),
                    "".to_string(),
                    "".to_string(),
                    "".to_string(),
                ),
            )
            .await;

            let mut conn = match req.state().db_pool.acquire().await {
                Ok(c) => c,
                Err(e) => {
                    return Ok(
                        tide::Response::builder(tide::StatusCode::InternalServerError)
                            .content_type(mime::PLAIN)
                            .body(e.to_string())
                            .build(),
                    )
                }
            };

            insert_user(&mut conn, &new_user).await;
            new_user
        }
    };
    let mut res = Response::new(StatusCode::Ok);
     
    let jwt =
        create_jwt(&user, "".to_string(), "".to_string(), "".to_string()).expect("JWT Created");

    let now = SystemTime::now();
    let cadd = now;
    let datetime: DateTime<Utc> = cadd.into();
    let cookie_str = format!("token={}; Expires={}", jwt, datetime.to_string());
    let c = Cookie::parse(cookie_str).unwrap();
    res.insert_cookie(c);
    Ok(res)
}

#[derive(Template)]
#[template(path = "user.html")]
pub struct UserTemplate<'a> {
    user: &'a User,
    email: &'a str,
}

impl<'a> UserTemplate<'a> {
    pub fn new(user: &'a User, email: &'a str) -> Self {
        return Self { user, email };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Deserialize)]
pub struct LoginPostRequest {
    pub uid: String,
    pub secret: String,
    pub homeserver: String,
}

#[derive(Template)]
#[template(path = "login_username.html")]

pub struct LoginUsernameTemplate {
    user: User,
}

impl<'a> LoginUsernameTemplate {
    pub fn new() -> Self {
        return Self {
            user: User {
                key: uuid::Uuid::nil(),
                organization_key: uuid::Uuid::nil(),
                email: "".to_string(),
                matrix_home_server: "".to_string(),
                matrix_user_id: "".to_string(),
                created: 0,
                updated: 0,
            },
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    user: User,
    choices: Vec<Choice>,
}

impl<'a> LoginTemplate {
    pub fn new(choices: Vec<Choice>) -> Self {
        return Self {
            choices: choices,
            user: User {
                key: uuid::Uuid::nil(),
                organization_key: uuid::Uuid::nil(),
                email: "".to_string(),
                matrix_home_server: "".to_string(),
                matrix_user_id: "".to_string(),
                created: 0,
                updated: 0,
            },
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {
    user: User,
    choices: Vec<Choice>,
}

impl<'a> RegisterTemplate {
    pub fn new(choices: Vec<Choice>) -> Self {
        return Self {
            choices: choices,
            user: User {
                key: uuid::Uuid::nil(),
                organization_key: uuid::Uuid::nil(),
                email: "".to_string(),
                matrix_home_server: "".to_string(),
                matrix_user_id: "".to_string(),
                created: 0,
                updated: 0,
            },
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterUser {
    pub email: String,
    pub organization: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginData {
    email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserPreferences {
    pub key: uuid::Uuid,
    pub user_key: uuid::Uuid,
    pub email: String,

    pub matrix_user_id: String,
    pub matrix_home_server: String,
    pub created: i64,
    pub updated: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub email: String,
    pub matrix_user_id: String,
    pub matrix_home_server: String,
    pub created: i64,
    pub updated: i64,
}

impl ToString for User {
    fn to_string(&self) -> String {
        format!("{}", self.email)
    }
}

impl User {
    pub fn new(
        key: uuid::Uuid,
        organization_key: uuid::Uuid,
        email: String,

        matrix_user_id: String,
        matrix_home_server: String,
    ) -> Self {
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            organization_key,
            email,
            matrix_user_id,
            matrix_home_server,
            created,
            updated,
        }
    }
}
