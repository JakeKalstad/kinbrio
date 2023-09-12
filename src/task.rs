use std::str::FromStr;

use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use sqlx::postgres::PgQueryResult;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use uuid::Uuid;

use tide::{http::mime, Request};

use crate::home::NotFoundTemplate;
use crate::{State, user};
use crate::matrix::post_task_create;
 
// SQL STUFF

pub async fn get_task( conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Task {
    let task = sqlx::query!(
        "select key, owner_key, organization_key, project_key, assignee_key, name, description, tags, status, estimated_quarter_days, start, due, created, updated from tasks where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select task by key");

    let t_status: TaskStatus = task.status.expect("status exists").into();
    Task {
        key: task.key.expect("key exists"),
        organization_key: task.organization_key.expect("organization_key"), 
        project_key: task.project_key.expect("project_key"), 
        owner_key: task.owner_key.expect("owner_key exists"),
        assignee_key: task.assignee_key.expect("assignee_key exists"),
        name: task.name.expect("name exists"),
        description: task.description.expect("description exists"),
        tags: task.tags.expect("tags exists"),
        status: t_status,
        estimated_quarter_days: task.estimated_quarter_days.expect("estimated_quarter_days exists"),
        start: task.start.expect("start"),
        due:task.due.expect("due"),
        created: task.created.expect("created exists"),
        updated: task.updated.expect("updated exists"),
    }
}
pub async fn get_tasks_by_organization(conn: &mut PoolConnection<Postgres>,organization_key: uuid::Uuid) -> Vec<Task> {
    let records = sqlx::query!(
        "select key, owner_key, organization_key, project_key, assignee_key, name, description, tags, status, estimated_quarter_days, start, due, created, updated from tasks where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .unwrap_or_default();

    let mut tasks = Vec::<Task>::new();
    for task in records {
        let t_status: TaskStatus = task.status.expect("status exists").into();
        let tsk = Task {
            key: task.key.expect("key exists"),
            organization_key: task.organization_key.expect("organization_key"), 
            project_key: task.project_key.expect("project_key"), 
            owner_key: task.owner_key.expect("owner_key exists"),
            assignee_key: task.assignee_key.expect("assignee_key exists"),
            name: task.name.expect("name exists"),
            description: task.description.expect("description exists"),
            tags: task.tags.expect("tags exists"),
            status: t_status,
            estimated_quarter_days: task.estimated_quarter_days.expect("estimated_quarter_days exists"),
            start: task.start.expect("start"),
            due:task.due.expect("due"),
            created: task.created.expect("created exists"),
            updated: task.updated.expect("updated exists"),
        };
        tasks.push(tsk);
    }
    return tasks
}
pub async fn get_tasks_by_project(conn: &mut PoolConnection<Postgres>,project_key: uuid::Uuid) -> Vec<Task> {
    let records = sqlx::query!(
        "select key, owner_key, organization_key, project_key, assignee_key, name, description, tags, status, estimated_quarter_days, start, due, created, updated from tasks where project_key = $1",
        project_key
    )
    .fetch_all(conn)
    .await
    .unwrap_or_default();

    let mut tasks = Vec::<Task>::new();
    for task in records {
        let t_status: TaskStatus = task.status.expect("status exists").into();
        let tsk = Task {
            key: task.key.expect("key exists"),
            organization_key: task.organization_key.expect("organization_key"), 
            project_key: task.project_key.expect("project_key"), 
            owner_key: task.owner_key.expect("owner_key exists"),
            assignee_key: task.assignee_key.expect("assignee_key exists"),
            name: task.name.expect("name exists"),
            description: task.description.expect("description exists"),
            tags: task.tags.expect("tags exists"),
            status: t_status,
            estimated_quarter_days: task.estimated_quarter_days.expect("estimated_quarter_days exists"),
            start: task.start.expect("start"),
            due:task.due.expect("due"),
            created: task.created.expect("created exists"),
            updated: task.updated.expect("updated exists"),
        };
        tasks.push(tsk);
    }
    return tasks
}


async fn delete_task( conn: &mut PoolConnection<Postgres>, key: uuid::Uuid, owner_key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM tasks where owner_key=$1 AND key=$2", owner_key, key)
    .execute( conn)
    .await;
}

async fn insert_task( conn: &mut PoolConnection<Postgres>, new_task: &Task) {
    sqlx::query!("INSERT INTO tasks (key, owner_key, organization_key, project_key, assignee_key, name, description, tags, status, estimated_quarter_days, start, due, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)", 
        new_task.key,
        new_task.owner_key,
        new_task.organization_key,
        new_task.project_key, 
        new_task.assignee_key,
        &new_task.name,
        &new_task.description,
        &new_task.tags,
        new_task.status as i16,
        new_task.estimated_quarter_days,
        new_task.start,
        new_task.due,
        new_task.created,
        new_task.updated,
    )
    .execute( conn)
    .await
    .expect("Insert Success");
}

async fn update_task(conn: &mut PoolConnection<Postgres>, task: &Task) {
    sqlx::query!("UPDATE tasks SET 
    organization_key=$2, project_key=$3, owner_key=$4, assignee_key=$5, name=$6, 
    description=$7, tags=$8, status=$9, estimated_quarter_days=$10, start=$11, due=$12, 
    updated=extract(epoch from now()) where key = $1",
     task.key, task.organization_key, task.project_key, task.owner_key, task.assignee_key, task.name,task.description, 
     task.tags, task.status as i16, task.estimated_quarter_days, task.start, task.due)
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
    match req.param("task_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("Task uuid parse");
            match delete_task(&mut conn, s_uuid, u.key).await {
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
    let u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    match req.param("task_id") {
        Ok(key) => {
            let s_uuid = uuid::Uuid::from_str(key).expect("Task uuid parse");
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let task = get_task(&mut conn, s_uuid).await;
            let users = crate::user::get_users_by_organization(&mut conn, u.organization_key).await;
            
            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    TaskTemplate::new(
                        task,
                        u,
                        users,
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
    let umd: Result<Task, tide::Error> = req.body_json().await;

    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
     
    match umd {
        Ok(task) => {
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

            if task.key == uuid::Uuid::nil() {
                let s = Task::new(task.organization_key, task.project_key, task.owner_key, task.assignee_key, task.name, task.description, task.tags,TaskStatus::Todo, task.estimated_quarter_days, task.start, task.due);
                insert_task(&mut conn, &s).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                post_task_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id, organization_key, claims.matrix_access_token, &s).await.expect("Posting to matrix");
                let j = serde_json::to_string(&s).expect("To JSON");
                return Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build());
            }

            update_task(&mut conn, &task).await;
            let j = serde_json::to_string(&task).expect("To JSON");
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
#[derive(PartialEq, Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, EnumIter)]
pub enum TaskStatus {
    Wishlist,
    Todo,
    PlanningAndEstimating,
    InQueue,
    InProgress,
    ToReview,
    InReviewal,
    Complete,
}
impl Into<TaskStatus> for i16 {
    fn into(self) -> TaskStatus {
        match self {
            0 => TaskStatus::Wishlist,
            1 => TaskStatus::Todo, 
            2 => TaskStatus::PlanningAndEstimating, 
            3 => TaskStatus::InQueue, 
            4 => TaskStatus::InProgress, 
            5 => TaskStatus::ToReview, 
            6 => TaskStatus::InReviewal, 
            7 => TaskStatus::Complete, 
            _ => TaskStatus::Wishlist
        }
    }
}

 
impl FromStr for TaskStatus {
    type Err = ();
    fn from_str(input: &str) -> Result<TaskStatus, Self::Err> {
        match input {
            "Wishlist" => Ok(TaskStatus::Wishlist),
            "Todo" => Ok(TaskStatus::Todo),
            "PlanningAndEstimating" => Ok(TaskStatus::PlanningAndEstimating),
            "InQueue" => Ok(TaskStatus::InQueue),
            "InProgress" => Ok(TaskStatus::InProgress),
            "ToReview" => Ok(TaskStatus::ToReview),
            "InReviewal" => Ok(TaskStatus::InReviewal),
            "Complete" => Ok(TaskStatus::Complete),
            _ => Ok(TaskStatus::Wishlist),
        }
    }
}

impl ToString for TaskStatus {
    fn to_string(&self) -> String {
        match self {
            TaskStatus::Wishlist => "Wishlist".to_owned(),
            TaskStatus::Todo => "Todo".to_owned(),
            TaskStatus::PlanningAndEstimating => "Planning And Estimating".to_owned(),
            TaskStatus::InQueue => "In Queue".to_owned(),
            TaskStatus::InProgress => "In Progress".to_owned(),
            TaskStatus::ToReview =>"To Review".to_owned(),
            TaskStatus::InReviewal => "In Reviewal".to_owned(),
            TaskStatus::Complete => "Complete".to_owned(),
        }
    }
}

impl From<TaskStatus> for i16 {
    fn from(t: TaskStatus) -> Self {
        match t {
            TaskStatus::Wishlist => 0,
            TaskStatus::Todo => 1, 
            TaskStatus::PlanningAndEstimating => 2,
            TaskStatus::InQueue => 3,
            TaskStatus::InProgress => 4,
            TaskStatus::ToReview => 5,
            TaskStatus::InReviewal => 6,
            TaskStatus::Complete => 7,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Task {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub project_key: uuid::Uuid,
    pub owner_key: uuid::Uuid,
    pub assignee_key: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub tags: String,
    pub status: TaskStatus,
    pub estimated_quarter_days: i32,
    pub start: i64,
    pub due: i64,
    pub created: i64,
    pub updated: i64,
}
 
impl Task {
    pub fn new(
        organization_key: uuid::Uuid,
        project_key: uuid::Uuid,
        owner_key: uuid::Uuid,
        assignee_key: uuid::Uuid,
        name: String,
        description: String,
        tags: String,
        status: TaskStatus,
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
            assignee_key,
            name,
            description,
            tags,
            status,
            estimated_quarter_days,
            start,
            due,
            created, 
            updated,
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
            let users = crate::user::get_users_by_organization(&mut conn, u.organization_key).await;
            let mut task= Task::new( u.organization_key, project_id,  u.key,  uuid::Uuid::nil(),  "".to_owned(), "".to_owned(), "".to_owned(), TaskStatus::Todo,  0, 0, 0);
            task.key = uuid::Uuid::nil();
            Ok(tide::Response::builder(tide::StatusCode::Ok)
            .content_type(mime::HTML)
            .body(
                TaskTemplate::new(
                    task,
                    u,
                    users,
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

#[derive(Template)]
#[template(path = "task.html")]
pub struct TaskTemplate {
    task: Task,
    user: crate::user::User,
    users: Vec::<crate::user::User>,
}

impl<'a> TaskTemplate {
    pub fn new(
        task: Task,
        user: crate::user::User,
        users: Vec::<crate::user::User>,
    ) -> Self {
        return Self {
            task,
            user,
            users,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
