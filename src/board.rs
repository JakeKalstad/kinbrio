use std::str::FromStr;

use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;
use strum::IntoEnumIterator;

use tide::{http::mime, Request}; 
use crate::home::NotFoundTemplate;
use crate::matrix::post_board_create;
use crate::task::{TaskStatus, Task, get_tasks_by_organization};
use crate::{State, user};
 
// SQL STUFF

pub async fn get_organization_boards(conn: &mut PoolConnection<Postgres>, organization_key: uuid::Uuid) -> Vec<Board> {
    let records = sqlx::query!(
        "select key, owner_key, organization_key, name, description, columns, lanes, filter, created, updated from boards where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut boards = Vec::<Board>::new();
    for board in records {
        let brd = Board {
            key: board.key.expect("key exists"),
            organization_key: board.organization_key.expect("organization_key"), 
            owner_key: board.owner_key.expect("owner_key exists"),
            name: board.name.expect("name exists"),
            description: board.description.expect("description exists"), 
            columns: board.columns.expect("columns"),
            lanes: board.lanes.expect("columns exists"), 
            filter: board.filter.expect("filter"),
            created: board.created.expect("created exists"),
            updated: board.updated.expect("updated exists"),
        };
        boards.push(brd);
    }
    return boards
}

pub async fn get_user_boards(conn: &mut PoolConnection<Postgres>, user_key: uuid::Uuid) -> Vec<Board> {
    let records = sqlx::query!(
        "select  key, owner_key, organization_key, name, description, columns, lanes, filter, created, updated from boards where owner_key = $1",
        user_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut boards = Vec::<Board>::new();
    for board in records {
        let brd: Board = Board {
            key: board.key.expect("key exists"),
            organization_key: board.organization_key.expect("organization_key"), 
            owner_key: board.owner_key.expect("owner_key exists"),
            name: board.name.expect("name exists"),
            description: board.description.expect("description exists"), 
            columns: board.columns.expect("columns"),
            lanes: board.lanes.expect("columns exists"), 
            filter: board.filter.expect("filter"),
            created: board.created.expect("created exists"),
            updated: board.updated.expect("updated exists"),
        };
        boards.push(brd);
    }
    return boards
}

pub async fn get_board(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Board {
    let board = sqlx::query!(
        "select key, owner_key, organization_key, name, description, columns, lanes, filter, created, updated from boards where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select board by key");

    Board {
        key: board.key.expect("key exists"),
        organization_key: board.organization_key.expect("organization_key"), 
        owner_key: board.owner_key.expect("owner_key exists"),
        name: board.name.expect("name exists"),
        description: board.description.expect("description exists"),
        columns: board.columns.expect("columns exists"), 
        lanes: board.lanes.expect("columns exists"), 
        filter: board.filter.expect("filter"),
        created: board.created.expect("created exists"),
        updated: board.updated.expect("updated exists"),
    }
}

async fn delete_board(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid, owner_key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM boards where owner_key=$1 AND key=$2", owner_key, key)
    .execute(conn)
    .await;
}
async fn update_board(conn: &mut PoolConnection<Postgres>, board: &Board) {
    sqlx::query!("UPDATE boards SET name=$1, description=$2, columns=$3, 
    lanes=$4, filter=$5 where key=$6",  
    &board.name,
    &board.description,
    &board.columns, 
    &board.lanes, 
    &board.filter, 
    board.key,
)
.execute(conn)
.await
.expect("Insert Success");
}

async fn insert_board(conn: &mut PoolConnection<Postgres>, new_board: &Board) {
    sqlx::query!("INSERT INTO boards (key, organization_key, owner_key, name, description, columns, lanes, filter, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)", 
        new_board.key,
        new_board.organization_key, 
        new_board.owner_key,
        &new_board.name,
        &new_board.description,
        &new_board.columns, 
        &new_board.lanes, 
        &new_board.filter, 
        new_board.created,
        new_board.updated,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}


pub async fn add(req: Request<State>) -> tide::Result {
    let u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    let mut board= Board::new( u.organization_key, u.key,  "".to_owned(), "".to_owned(), vec![], vec![], "".to_owned());
    board.key = uuid::Uuid::nil();
    Ok(tide::Response::builder(tide::StatusCode::Ok)
    .content_type(mime::HTML)
    .body(
        BoardTemplate::new(
            board,
            u,
            vec![],
            vec![],
        )
        .render_string(),
    )
    .build())
}
// Route Stuff

pub async fn delete(req: Request<State>) -> tide::Result {
    let u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    match req.param("board_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("Board uuid parse");
            match delete_board(&mut conn, s_uuid, u.key).await {
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

    match req.param("board_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("Board uuid parse");
            let board = get_board(&mut conn, s_uuid).await;
            let tasks = get_tasks_by_organization(&mut conn, u.organization_key).await;
            let users = crate::user::get_users_by_organization(&mut conn, u.organization_key).await;

            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    BoardTemplate::new(
                        board,
                        u,
                        tasks,
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
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
    let umd: Result<Board, tide::Error> = req.body_json().await;
    match umd {
        Ok(board) => {
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
            if board.key == uuid::Uuid::nil() {
                let s = Board::new(board.organization_key, board.owner_key, board.name, board.description, board.columns, board.lanes, board.filter);
                insert_board(&mut conn, &s).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                post_board_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id, organization_key, claims.matrix_access_token, &s).await.expect("Posting to matrix");
                let j = serde_json::to_string(&s).expect("To JSON");
                Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())
            } else {
                update_board(&mut conn, &board).await;
                let j = serde_json::to_string(&board).expect("To JSON");
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

// data types 
#[derive(Debug, Deserialize, Serialize)]
pub struct Board {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub owner_key: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub columns: Vec::<String>,
    pub lanes: Vec::<String>,
    pub filter: String,
    pub created: i64,
    pub updated: i64,
}
 
impl Board {
    pub fn new(
        organization_key: uuid::Uuid,
        owner_key: uuid::Uuid,
        name: String,
        description: String,
         columns: Vec::<String>,
         lanes: Vec::<String>,
        filter: String,
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
            columns,
            lanes,
            filter,
            created, 
            updated,
        }
    }
}

#[derive(Template)]
#[template(path = "board.html")]
pub struct BoardTemplate {
    board: Board,
    user: crate::user::User,
    tasks: Vec<Task>,
    users: Vec<crate::user::User>,
}
impl<'a> BoardTemplate {
    pub fn new(
        board: Board,
        user: crate::user::User,
        tasks: Vec<Task>,
        users: Vec<crate::user::User>,
    ) -> Self {
        return Self {
            board,
            user,
            tasks,
            users,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    } 
    
    pub fn lane_name<'aa>(&'aa self, lane: &String) -> String{
        let st = TaskStatus::from_str(lane).expect("Get status");
        st.to_string()
    }

    pub fn get_tasks<'aa>(&'aa self, lane: String, tasks: &'aa Vec<Task>) -> Vec<&Task> {
        let st = TaskStatus::from_str(&lane).expect("Get status");
        let lane_tasks = tasks.iter().filter(|t| t.status == st).collect();
        lane_tasks
    }

    pub fn lane_contained(&self, lane: &TaskStatus, lanes: &Vec<String>) -> bool {
        let s_lane = lane.to_string().replace(" ", "");
        lanes.contains(&s_lane)
    }
}
