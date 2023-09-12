use std::str::FromStr;

use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use tide::{http::mime, Request};

use crate::home::NotFoundTemplate;
use crate::matrix::post_milestone_create;
use crate::{State, user};
 
// SQL STUFF

pub async fn get_milestones_by_project(conn: &mut PoolConnection<Postgres>, project_key: uuid::Uuid) -> Vec::<Milestone> {
    let milestone_records = sqlx::query!(
        "select key, organization_key, owner_key, project_key, name, description, tags, estimated_quarter_days, start, due, created, updated from mile_stones where project_key = $1",
        project_key
    )
    .fetch_all(conn)
    .await
    .expect("Select milestone by key");
    let mut milestones = vec![];
    for milestone in milestone_records { 
        milestones.push(Milestone {
            key: milestone.key.expect("key exists"),
            organization_key: milestone.organization_key.expect("organization_key"), 
            owner_key: milestone.owner_key.expect("owner_key exists"),
            project_key: milestone.project_key.expect("project_key exists"),
            name: milestone.name.expect("name exists"),
            description: milestone.description.expect("description exists"),
            tags: milestone.tags.expect("tags exists"),
            estimated_quarter_days: milestone.estimated_quarter_days.expect("estimated_quarter_days exists"),
            start: milestone.start.expect("start"),
            due:milestone.due.expect("due"),
            created: milestone.created.expect("created exists"),
            updated: milestone.updated.expect("updated exists"),
        })
    }
    milestones
}

pub async fn get_milestone(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Milestone {
    let milestone = sqlx::query!(
        "select key, organization_key, owner_key, project_key, name, description, tags, estimated_quarter_days, start, due, created, updated from mile_stones where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select milestone by key");

    Milestone {
        key: milestone.key.expect("key exists"),
        organization_key: milestone.organization_key.expect("organization_key"), 
        owner_key: milestone.owner_key.expect("owner_key exists"),
        project_key: milestone.project_key.expect("project_key exists"),
        name: milestone.name.expect("name exists"),
        description: milestone.description.expect("description exists"),
        tags: milestone.tags.expect("tags exists"),
        estimated_quarter_days: milestone.estimated_quarter_days.expect("estimated_quarter_days exists"),
        start: milestone.start.expect("start"),
        due:milestone.due.expect("due"),
        created: milestone.created.expect("created exists"),
        updated: milestone.updated.expect("updated exists"),
    }
}

async fn delete_milestone(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid, owner_key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM mile_stones where owner_key=$1 AND key=$2", owner_key, key)
    .execute(conn)
    .await;
}

async fn update_milestone(conn: &mut PoolConnection<Postgres>, board: &Milestone) {
    sqlx::query!("UPDATE mile_stones SET name=$1, description=$2, tags=$3, 
    estimated_quarter_days=$4, start=$5, due=$6 where key=$7",
    &board.name,
    &board.description,
    &board.tags, 
    &board.estimated_quarter_days, 
    &board.start, 
    &board.due, 
    board.key,
)
.execute(conn)
.await
.expect("Insert Success");
}

async fn insert_milestone(conn: &mut PoolConnection<Postgres>, new_milestone: &Milestone) {
    sqlx::query!("INSERT INTO mile_stones (key, organization_key, project_key, owner_key, name, description, tags, estimated_quarter_days, start, due, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)", 
        new_milestone.key,
        new_milestone.organization_key, 
        new_milestone.project_key, 
        new_milestone.owner_key,
        &new_milestone.name,
        &new_milestone.description,
        &new_milestone.tags,
        new_milestone.estimated_quarter_days,
        new_milestone.start,
        new_milestone.due,
        new_milestone.created,
        new_milestone.updated,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}

// Route Stuff

pub async fn delete(req: Request<State>) -> tide::Result {
    let u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    match req.param("milestone_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("Milestone uuid parse");
            match delete_milestone(&mut conn, s_uuid, u.key).await {
                Ok(_) =>    Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .build()),
                Err(_) =>  Ok(tide::Response::builder(tide::StatusCode::InternalServerError)
                .content_type(mime::HTML)
                .body(NotFoundTemplate::new().render_string())
                .build()),
            }
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

pub async fn get(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };
    match req.param("milestone_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("Milestone uuid parse");
            let milestone = get_milestone(&mut conn, s_uuid).await;

            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    MilestoneTemplate::new(
                        milestone,
                        u,
                    )
                    .render_string(),
                )
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

pub async fn insert(mut req: Request<State>) -> tide::Result {
    let umd: Result<Milestone, tide::Error> = req.body_json().await;
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    }; 
    match umd {
        Ok(milestone) => {
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
            
            if milestone.key == uuid::Uuid::nil() {
                let s = Milestone::new(milestone.organization_key, milestone.owner_key, milestone.project_key, milestone.name, milestone.description, milestone.tags, milestone.estimated_quarter_days, milestone.start, milestone.due);
                insert_milestone(&mut conn, &s).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                post_milestone_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id, organization_key, claims.matrix_access_token, &s).await.expect("Posting to matrix");
                let j = serde_json::to_string(&s).expect("To JSON");
                Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())
            } else {
                update_milestone(&mut conn, &milestone).await;
                let j = serde_json::to_string(&milestone).expect("To JSON");
                return Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())
            }
            
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

pub async fn add(req: Request<State>) -> tide::Result {
    let u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    
    match req.param("project_id") {
        Ok(project_id) => { 
            let project_id = uuid::Uuid::from_str(project_id).expect("Project uuid parse");
            let mut milestone= Milestone::new( u.organization_key, u.key, project_id,  "".to_owned(), "".to_owned(), "".to_owned(),  0, 0, 0);
            milestone.key = uuid::Uuid::nil();
            Ok(tide::Response::builder(tide::StatusCode::Ok)
            .content_type(mime::HTML)
            .body(
                MilestoneTemplate::new(
                    milestone,
                    u,
                )
                .render_string(),
            )
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
// data types

#[derive(Debug, Deserialize, Serialize)]
pub struct Milestone {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub owner_key: uuid::Uuid,
    pub project_key: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub tags: String,

    pub estimated_quarter_days: i32,
    pub start: i64,
    pub due: i64,
    pub created: i64,
    pub updated: i64,
}
 
impl Milestone {
    pub fn new(
        organization_key: uuid::Uuid,
        owner_key: uuid::Uuid,
        project_key: uuid::Uuid,
        name: String,
        description: String,
        tags: String,
        estimated_quarter_days:i32,
        start: i64,
        due: i64,
    ) -> Self {
        let key = Uuid::new_v4();
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            organization_key,
            project_key,
            owner_key,
            name,
            description,
            tags,
            estimated_quarter_days,
            start,
            due,
            created, 
            updated,
        }
    }
}

#[derive(Template)]
#[template(path = "milestone.html")]
pub struct MilestoneTemplate {
    milestone: Milestone,
    user: crate::user::User,
}

impl<'a> MilestoneTemplate {
    pub fn new(
        milestone: Milestone,
        user: crate::user::User,
    ) -> Self {
        return Self {
            milestone,
            user,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
