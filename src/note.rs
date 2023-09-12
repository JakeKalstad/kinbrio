use std::str::FromStr;

use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use tide::{http::mime, Request};

use crate::file::AssociationType;
use crate::home::NotFoundTemplate;
use crate::matrix::post_note_create;
use crate::{State, user};
 
// SQL STUFF
pub async fn get_associated_notes(conn: &mut PoolConnection<Postgres>, association_type: AssociationType, associated_key: uuid::Uuid) -> Vec<Note> {
    let records = sqlx::query!(
        "select key, organization_key, owner_key, association_type, association_key, title, content, url, created, updated from notes 
        where association_type= $1 AND association_key = $2",
        association_type as i16, associated_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut notes = Vec::<Note>::new();
    for note in records {
        let association_type: AssociationType = note.association_type.expect("association_type exists").into();
        let n= Note {
            key: note.key.expect("key exists"),
            organization_key: note.organization_key.expect("organization_key"), 
            owner_key: note.owner_key.expect("owner_key exists"),
            association_type, 
            association_key: note.association_key.expect("association_key exists"), 
            title: note.title.expect("title exists"),
            content: note.content.expect("content exists"),
            url: note.url.expect("url exists"),
            created: note.created.expect("created exists"),
            updated: note.updated.expect("updated exists"),
        };
        notes.push(n);
    }
    return notes
}

pub async fn get_organization_notes(conn: &mut PoolConnection<Postgres>, organization_key: uuid::Uuid) -> Vec<Note> {
    let records = sqlx::query!(
        "select key, organization_key, owner_key, association_type, association_key, title, content, url, created, updated from notes where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut notes = Vec::<Note>::new();
    for note in records {
        let association_type: AssociationType = note.association_type.expect("association_type exists").into();
        let n = Note {
            key: note.key.expect("key exists"),
            organization_key: note.organization_key.expect("organization_key"), 
            owner_key: note.owner_key.expect("owner_key exists"),
            association_type, 
            association_key: note.association_key.expect("association_key exists"), 
            title: note.title.expect("title exists"),
            content: note.content.expect("content exists"),
            url: note.url.expect("url exists"),
            created: note.created.expect("created exists"),
            updated: note.updated.expect("updated exists"),
        };
        notes.push(n);
    }
    return notes
}

pub async fn get_user_notes(conn: &mut PoolConnection<Postgres>, user_key: uuid::Uuid) -> Vec<Note> {
    let records = sqlx::query!(
        "select key, organization_key, owner_key, association_type, association_key, title, content, url, created, updated from notes where owner_key = $1",
        user_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut notes = Vec::<Note>::new();
    for note in records {
        let association_type: AssociationType = note.association_type.expect("association_type exists").into();
        let n: Note = Note {
            key: note.key.expect("key exists"),
            organization_key: note.organization_key.expect("organization_key"), 
            owner_key: note.owner_key.expect("owner_key exists"),
            association_type, 
            association_key: note.association_key.expect("association_key exists"), 
            title: note.title.expect("title exists"),
            content: note.content.expect("content exists"),
            url: note.url.expect("url exists"),
            created: note.created.expect("created exists"),
            updated: note.updated.expect("updated exists"),
        };
        notes.push(n);
    }
    return notes
}

pub async fn get_note(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Note {
    let note = sqlx::query!(
        "select key, organization_key, owner_key, association_type, association_key, title, content, url, created, updated from notes where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select note by key");
    let association_type: AssociationType = note.association_type.expect("association_type exists").into();
    Note {
        key: note.key.expect("key exists"),
        organization_key: note.organization_key.expect("organization_key"), 
        owner_key: note.owner_key.expect("owner_key exists"),
        association_type, 
        association_key: note.association_key.expect("association_key exists"), 
        title: note.title.expect("title exists"),
        content: note.content.expect("content exists"),
        url: note.url.expect("url exists"),
        created: note.created.expect("created exists"),
        updated: note.updated.expect("updated exists"),
    }
}

async fn delete_note(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid, owner_key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM notes where owner_key=$1 AND key=$2", owner_key, key)
    .execute(conn)
    .await;
}
async fn update_note(conn: &mut PoolConnection<Postgres>, note: &Note) {
    sqlx::query!("UPDATE notes SET title=$1, content=$2, association_type=$3, 
    association_key=$4, url=$5 where key=$6",  
    &note.title,
    &note.content,
    note.association_type as i16, 
    &note.association_key, 
    &note.url, 
    note.key,
)
.execute(conn)
.await
.expect("Insert Success");
}

async fn insert_note(conn: &mut PoolConnection<Postgres>, new_note: &Note) {
    sqlx::query!("INSERT INTO notes (key, organization_key, owner_key, association_type, association_key, title, content, url, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)", 
        new_note.key,
        new_note.organization_key, 
        new_note.owner_key,
        new_note.association_type as i16,
        new_note.association_key,
        &new_note.title,
        &new_note.content,
        &new_note.url, 
        new_note.created,
        new_note.updated,
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

    let ass_id = match req.param("association_id") {
        Ok(v) => v,
        Err(_) => {
            return Ok(tide::Response::builder(tide::StatusCode::InternalServerError)
            .content_type(mime::HTML)
            .body(NotFoundTemplate::new().render_string())
            .build())
        },
    };

    let ass_type = match req.param("association_type") {
        Ok(v) => v,
        Err(_) => {
            return Ok(tide::Response::builder(tide::StatusCode::InternalServerError)
            .content_type(mime::HTML)
            .body(NotFoundTemplate::new().render_string())
            .build())
        },
    };
    let associated_uuid = uuid::Uuid::parse_str(ass_id).expect("associated uuid valid");
    let association_type: AssociationType = AssociationType::from_str(ass_type).expect("Parse type");
    let mut note= Note::new( u.organization_key, u.key, association_type, associated_uuid, "".to_owned(), "".to_owned(), "".to_owned());
    note.key = uuid::Uuid::nil();
    Ok(tide::Response::builder(tide::StatusCode::Ok)
    .content_type(mime::HTML)
    .body(
        NoteTemplate::new(
            note,
            u,
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
    match req.param("note_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("Note uuid parse");
            match delete_note(&mut conn, s_uuid, u.key).await {
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

    match req.param("note_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("Note uuid parse");
            let note = get_note(&mut conn, s_uuid).await;

            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    NoteTemplate::new(
                        note,
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
    let umd: Result<Note, tide::Error> = req.body_json().await;
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
    match umd {
        Ok(note) => {
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
            if note.key == uuid::Uuid::nil() {
                let s = Note::new(note.organization_key, note.owner_key, note.association_type, note.association_key, note.url, note.title, note.content);
                insert_note(&mut conn, &s).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                post_note_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id, organization_key, claims.matrix_access_token, &s).await.expect("Posting to matrix");
                let j = serde_json::to_string(&s).expect("To JSON");
                Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())
            } else {
                update_note(&mut conn, &note).await;
                let j = serde_json::to_string(&note).expect("To JSON");
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
pub struct Note {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid, 
    pub owner_key: uuid::Uuid,
    pub association_type: AssociationType,
    pub association_key: uuid::Uuid,
    pub title: String,
    pub content: String,
    pub url: String,
    pub created: i64,
    pub updated: i64,
}
 
impl Note {
    pub fn new(
        organization_key: uuid::Uuid,
        owner_key: uuid::Uuid,
        association_type: AssociationType,
        association_key: uuid::Uuid,
        url: String,
        title: String,
        content: String,
    ) -> Self {
        let key = Uuid::new_v4();
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            organization_key,
            association_type,
            association_key,
            url,
            owner_key,
            title,
            content,
            created, 
            updated,
        }
    }
}

#[derive(Template)]
#[template(path = "note.html")]
pub struct NoteTemplate {
    note: Note,
    user: crate::user::User,
}

impl<'a> NoteTemplate {
    pub fn new(
        note: Note,
        user: crate::user::User,
    ) -> Self {
        return Self {
            note,
            user,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
