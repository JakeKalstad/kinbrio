use std::str::FromStr;

use askama::Template;

use crate::{
    board, entity, organization, project,
    user::{self, User},
    State, file, note, service_item,
};
use tide::{http::mime, Request};

pub async fn home(req: Request<State>) -> tide::Result {
    return match user::read_jwt_cookie(req.cookie("token")) {
        Some(_c) => dashboard(req).await,
        None => {
            let home = HomeTemplate::new(user::User::new(
                uuid::Uuid::nil(),
                uuid::Uuid::nil(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ));
            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(home.render_string())
                .build())
        }
    };
}
pub async fn account(req: Request<State>) -> tide::Result {
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
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
    let key = uuid::Uuid::from_str(claims.key.as_str()).expect("uuid parse");
    let user = user::get_user(&mut conn, key).await.expect("user exists");

    let home = user::UserTemplate::new(&user, user.email.as_str());
    Ok(tide::Response::builder(tide::StatusCode::Ok)
        .content_type(mime::HTML)
        .body(home.render_string())
        .build())
}

pub async fn documentation(req: Request<State>) -> tide::Result {
    return Ok(tide::Redirect::new("http://localhost:3005").into());
    let user = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => user::User::new(
            uuid::Uuid::from_str(c.key.as_str()).expect("parsed cookie"),
            uuid::Uuid::from_str(c.organization_key.as_str()).expect("parsed cookie"),
            c.email,
            c.matrix_user_id,
            c.matrix_home_server,
        ),
        None => {
             user::User::new(
                uuid::Uuid::nil(),
                uuid::Uuid::nil(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
             )
        }
    };
    
    let docs = DocumentationTemplate::new(user);
    return Ok(tide::Response::builder(tide::StatusCode::Ok)
        .content_type(mime::HTML)
        .body(docs.render_string())
        .build());
}

pub async fn dashboard(req: Request<State>) -> tide::Result {
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
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
    let key = uuid::Uuid::from_str(claims.key.as_str()).expect("uuid parse");
    let user = match user::get_user(&mut conn, key).await {
        Some(u) => u,
        None => {
            return Ok(tide::Redirect::new("/login").into());
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
    let org_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("Valid UUID");
    
    let projects = project::get_projects_by_organization(&mut conn, org_key).await;
    let organization = organization::get_organization(&mut conn, org_key).await;
    let organization_boards = board::get_organization_boards(&mut conn, org_key).await;
    let user_boards = board::get_user_boards(&mut conn, key).await;
    let entitys = entity::get_organization_entitys(&mut conn, org_key).await;
    let contacts = entity::get_contacts(&mut conn, org_key).await;
    let notes = note::get_organization_notes(&mut conn, org_key).await;
    let files = file::get_organization_files(&mut conn, org_key).await;
    let service_items = service_item::get_organization_service_items(&mut conn, org_key).await;
    let home = DashboardTemplate::new(
        user,
        organization,
        projects,
        organization_boards,
        user_boards,
        service_items,
        entitys,
        contacts,
        notes,
        files,
    );
    Ok(tide::Response::builder(tide::StatusCode::Ok)
        .content_type(mime::HTML)
        .body(home.render_string())
        .build())
}

#[derive(Template)]
#[template(path = "documentation.html")]
pub struct DocumentationTemplate {
    user: User,
}

impl<'a> DocumentationTemplate {
    pub fn new(user: User) -> Self {
        return Self { user };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    user: User,
    organization: organization::Organization,
    projects: Vec<project::Project>,
    organization_boards: Vec<board::Board>,
    user_boards: Vec<board::Board>,
    service_items: Vec<service_item::ServiceItem>,
    entitys: Vec<entity::Entity>,
    contacts: Vec<entity::Contact>,
    notes: Vec<note::Note>,
    files: Vec<file::File>,
}

impl<'a> DashboardTemplate {
    pub fn new(
        user: User,
        organization: organization::Organization,
        projects: Vec<project::Project>,
        organization_boards: Vec<board::Board>,
        user_boards: Vec<board::Board>,
        service_items: Vec<service_item::ServiceItem>,
        entitys: Vec<entity::Entity>,
        contacts: Vec<entity::Contact>,
        notes: Vec<note::Note>,
        files: Vec<file::File>,
    ) -> Self {
        return Self {
            user,
            organization,
            projects,
            organization_boards,
            user_boards,
            service_items,
            entitys,
            contacts,
            notes,
            files,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    user: User,
}

impl HomeTemplate {
    pub fn new(user: User) -> Self {
        return Self { user };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Template)]
#[template(path = "notfound.html")]
pub struct NotFoundTemplate {}

impl<'a> NotFoundTemplate {
    pub fn new() -> Self {
        return Self {};
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
