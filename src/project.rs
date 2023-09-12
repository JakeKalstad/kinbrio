use std::str::FromStr;

use askama::Template;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;
use strum::IntoEnumIterator;

use tide::{http::mime, Request};

use crate::home::NotFoundTemplate;
use crate::milestone::{Milestone, get_milestones_by_project};
use crate::{State, user, entity, note, file};
use crate::matrix::post_project_create;
use crate::task::{Task, get_tasks_by_project, TaskStatus};
 
// SQL STUFF
pub async fn get_projects_by_organization(conn: &mut PoolConnection<Postgres>, organization_key: uuid::Uuid) -> Vec<Project> {
    let records = sqlx::query!(
        "select key, owner_key, organization_key, name, description, tags, estimated_quarter_days, start, due, created, updated from projects where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut projects = Vec::<Project>::new();
    for project in records {
        let prj = Project {
            key: project.key.expect("key exists"),
            organization_key: project.organization_key.expect("organization_key"), 
            owner_key: project.owner_key.expect("owner_key exists"),
            name: project.name.expect("name exists"),
            description: project.description.expect("description exists"),
            tags: project.tags.expect("tags exists"),
            estimated_quarter_days: project.estimated_quarter_days.expect("estimated_quarter_days exists"),
            start: project.start.expect("start"),
            due:project.due.expect("due"),
            created: project.created.expect("created exists"),
            updated: project.updated.expect("updated exists"),
        };
        projects.push(prj);
    }
    return projects
}

pub async fn get_project(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Project {
    let project = sqlx::query!(
        "select key, owner_key, organization_key, name, description, tags, estimated_quarter_days, start, due, created, updated from projects where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select project by key");

    Project {
        key: project.key.expect("key exists"),
        organization_key: project.organization_key.expect("organization_key"), 
        owner_key: project.owner_key.expect("owner_key exists"),
        name: project.name.expect("name exists"),
        description: project.description.expect("description exists"),
        tags: project.tags.expect("tags exists"),
        estimated_quarter_days: project.estimated_quarter_days.expect("estimated_quarter_days exists"),
        start: project.start.expect("start"),
        due:project.due.expect("due"),
        created: project.created.expect("created exists"),
        updated: project.updated.expect("updated exists"),
    }
}

async fn delete_project(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid, owner_key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM projects where owner_key=$1 AND key=$2", owner_key, key)
    .execute(conn)
    .await;
}
 
async fn update_project(conn: &mut PoolConnection<Postgres>, project: &Project) {
    sqlx::query!("UPDATE projects SET name=$1, description=$2, tags=$3, estimated_quarter_days=$4, start=$5, due=$6 where key=$7", 
        &project.name,
        &project.description,
        &project.tags,
        project.estimated_quarter_days,
        project.start,
        project.due,
        project.key,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}

async fn insert_project(conn: &mut PoolConnection<Postgres>, new_project: &Project) {
    sqlx::query!("INSERT INTO projects (key, organization_key, owner_key, name, description, tags, estimated_quarter_days, start, due, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)", 
        new_project.key,
        new_project.organization_key, 
        new_project.owner_key,
        &new_project.name,
        &new_project.description,
        &new_project.tags,
        new_project.estimated_quarter_days,
        new_project.start,
        new_project.due,
        new_project.created,
        new_project.updated,
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
    match req.param("project_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("Project uuid parse");
            match delete_project(&mut conn, s_uuid, u.key).await {
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

pub async fn add(req: Request<State>) -> tide::Result {
    let u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    let mut project= Project::new( uuid::Uuid::nil(),  uuid::Uuid::nil(),  "".to_owned(), "".to_owned(), "".to_owned(),  0, 0, 0);
    project.key = uuid::Uuid::nil();
    Ok(tide::Response::builder(tide::StatusCode::Ok)
    .content_type(mime::HTML)
    .body(
        ProjectTemplate::new(
            project,
            vec![],
            vec![],
            u,
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .render_string(),
    )
    .build())
}

pub async fn get(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };
    match req.param("project_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await.expect("Acquiring connection");
            let p_uuid = uuid::Uuid::from_str(key).expect("Project uuid parse");
            let project = get_project(&mut conn, p_uuid).await;
            let tasks = get_tasks_by_project(&mut conn, project.key).await;
            let milestones = get_milestones_by_project(&mut conn, project.key).await;

            let entitys = entity::get_organization_entitys(&mut conn, u.organization_key).await;
            let contacts = entity::get_contacts(&mut conn, u.organization_key).await;
            let notes = note::get_associated_notes(&mut conn, crate::file::AssociationType::Project, p_uuid).await;
            let files = file::get_associated_files(&mut conn, crate::file::AssociationType::Project, p_uuid).await;

            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    ProjectTemplate::new(
                        project,
                        tasks,
                        milestones,
                        u,
                        entitys,
                        contacts,
                        notes, 
                        files,
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
    let umd: Result<Project, tide::Error> = req.body_json().await;

    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
    
    match umd {
        Ok(project) => {
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
            if project.key == uuid::Uuid::nil() {
                let s = Project::new(project.organization_key, project.owner_key, project.name, project.description, project.tags, project.estimated_quarter_days, project.start, project.due);
                insert_project(&mut conn, &s).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                post_project_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id, organization_key, claims.matrix_access_token, &s).await.expect("Posting to matrix");
            
                let j = serde_json::to_string(&s).expect("To JSON");
                return Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())
            }

            update_project(&mut conn, &project).await;
            let j = serde_json::to_string(&project).expect("To JSON");
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

// data types

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub owner_key: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub tags: String,

    pub estimated_quarter_days: i32,
    pub start: i64,
    pub due: i64,
    pub created: i64,
    pub updated: i64,
}
 
impl Project {
    pub fn new(
        organization_key: uuid::Uuid,
        owner_key: uuid::Uuid,
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
#[template(path = "project.html")]
pub struct ProjectTemplate {
    project: Project,
    tasks: Vec<Task>,
    milestones: Vec<Milestone>,
    user: crate::user::User,
    
    entitys: Vec<entity::Entity>,
    contacts: Vec<entity::Contact>,
    notes: Vec<note::Note>,
    files: Vec<file::File>,
}

impl<'a> ProjectTemplate {
    pub fn new(
        project: Project,
        tasks: Vec<Task>,
        milestones: Vec<Milestone>,
        user: crate::user::User,
        entitys: Vec<entity::Entity>,
        contacts: Vec<entity::Contact>,
        notes: Vec<note::Note>,
        files: Vec<file::File>,
    ) -> Self {
        return Self {
            project,
            tasks,
            milestones,
            user,
            entitys,
            contacts,
            notes, 
            files,
        };
    }
    
    pub fn get_task_background_color<'aa>(&'aa self, status: &TaskStatus) -> String { 
        match *status {
            TaskStatus::Wishlist => "#cc66ff85".to_string(),
            TaskStatus::Todo => "#ff6a4a85".to_string(),
            TaskStatus::PlanningAndEstimating => "#ffb26085".to_string(),
            TaskStatus::InQueue => "#f9ff5b85".to_string(),
            TaskStatus::InProgress => "#61d1d085".to_string(),
            TaskStatus::ToReview => "#658cff85".to_string(),
            TaskStatus::InReviewal => "#D2CAFF85".to_string(),
            TaskStatus::Complete => "#64ff6385".to_string(),
        }
    }


    pub fn get_grid_column<'aa>(&'aa self, start: &i64, due: &i64) -> String {
        let start = chrono::NaiveDateTime::from_timestamp_opt(*start,0).expect("Parsed start date");
        let due = chrono::NaiveDateTime::from_timestamp_opt(*due,0).expect("Parsed due date");
        let col = start.date().weekday().num_days_from_monday() + 1;
        let difference = due.date()- start.date();
        format!("{} / span {}", col, difference.num_days())
    }
    
    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
