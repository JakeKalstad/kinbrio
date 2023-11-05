use std::io::Cursor;
use std::str::FromStr;

use crate::common::BufferedBytesStream;
use askama::Template;
use minio_rsc::Minio;
use minio_rsc::provider::StaticProvider;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgQueryResult;
use sqlx::Postgres;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use tide::{http::mime, Request};

use crate::home::NotFoundTemplate;
use crate::matrix::post_file_create;
use crate::{user, State};


pub async fn get_associated_files(
    conn: &mut PoolConnection<Postgres>,
    association_type: AssociationType,
    association_key: uuid::Uuid,
) -> Vec<File> {
    let file_records = sqlx::query!(
        "select key, owner_key, organization_key, association_type, association_key, url, hash, name, description, tags, format, size, created, updated 
        from files 
        WHERE association_type = $1 AND association_key = $2",
        association_type as i16, association_key
    )
    .fetch_all(conn)
    .await
    .expect("Select file by association key");
    let mut files = vec![];
    for file in file_records {
        let association_type: AssociationType = file
            .association_type
            .expect("association_type exists")
            .into();
        files.push(File {
            key: file.key.expect("key exists"),
            owner_key: file.owner_key.expect("owner_key exists"),
            organization_key: file.organization_key.expect("organization_key exists"),
            hash: file.hash.expect("hash exists"),
            name: file.name.expect("name exists"),
            description: file.description.expect("description exists"),
            url: file.url.expect("url exists"),
            tags: file.tags.expect("tags exists"),
            format: file.format.expect("format exists"),
            size: file.size.expect("size exists"),
            created: file.created.expect("created exists"),
            updated: file.updated.expect("updated exists"),
            association_type,
            association_key: file.association_key.expect("association_key exists"),
        });
    }
    return files;
}
 

pub async fn get_organization_files(
    conn: &mut PoolConnection<Postgres>,
    org_key: uuid::Uuid,
) -> Vec<File> {
    let file_records = sqlx::query!(
        "select key, owner_key, organization_key, association_type, association_key, url, hash, name, description, tags, format, size, created, updated from files where organization_key = $1",
        org_key
    )
    .fetch_all(conn)
    .await
    .expect("Select file by key");
    let mut files = vec![];
    for file in file_records {
        let association_type: AssociationType = file
            .association_type
            .expect("association_type exists")
            .into();
        files.push(File {
            key: file.key.expect("key exists"),
            owner_key: file.owner_key.expect("owner_key exists"),
            organization_key: file.organization_key.expect("organization_key exists"),
            hash: file.hash.expect("hash exists"),
            name: file.name.expect("name exists"),
            description: file.description.expect("description exists"),
            url: file.url.expect("url exists"),
            tags: file.tags.expect("tags exists"),
            format: file.format.expect("format exists"),
            size: file.size.expect("size exists"),
            created: file.created.expect("created exists"),
            updated: file.updated.expect("updated exists"),
            association_type,
            association_key: file.association_key.expect("association_key exists"),
        });
    }
    return files;
}

pub async fn get_file(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> File {
    let file = sqlx::query!(
        "select key, owner_key, organization_key, association_type, association_key, url, hash, name, description, tags, format, size, created, updated from files where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select file by key");

    let association_type: AssociationType = file
        .association_type
        .expect("association_type exists")
        .into();
    File {
        key: file.key.expect("key exists"),
        owner_key: file.owner_key.expect("owner_key exists"),
        organization_key: file.organization_key.expect("organization_key exists"),
        hash: file.hash.expect("hash exists"),
        name: file.name.expect("name exists"),
        description: file.description.expect("description exists"),
        url: file.url.expect("url exists"),
        tags: file.tags.expect("tags exists"),
        format: file.format.expect("format exists"),
        size: file.size.expect("size exists"),
        created: file.created.expect("created exists"),
        updated: file.updated.expect("updated exists"),
        association_type,
        association_key: file.association_key.expect("association_key exists"),
    }
}

async fn delete_file(
    conn: &mut PoolConnection<Postgres>,
    key: uuid::Uuid,
    owner_key: uuid::Uuid,
) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!(
        "DELETE FROM files where owner_key=$1 AND key=$2",
        owner_key,
        key
    )
    .execute(conn)
    .await;
}

async fn insert_file(conn: &mut PoolConnection<Postgres>, new_file: &File) {
    sqlx::query!("INSERT INTO files (key, owner_key, organization_key, association_type, association_key, url, hash, name, description, tags, format, size, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)", 
        new_file.key,
        new_file.owner_key,
        new_file.organization_key,
        new_file.association_type as i16,
        new_file.association_key,
        &new_file.url,
        &new_file.hash,
        &new_file.name,
        &new_file.description,
        &new_file.tags,
        &new_file.format,
        new_file.size,
        new_file.created,
        new_file.updated,
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
    match req.param("file_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("File uuid parse");
            match delete_file(&mut conn, s_uuid, u.key).await {
                Ok(_) => Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::HTML)
                    .build()),
                Err(_) => Ok(
                    tide::Response::builder(tide::StatusCode::InternalServerError)
                        .content_type(mime::HTML)
                        .body(NotFoundTemplate::new().render_string())
                        .build(),
                ),
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

    match req.param("association_type") {
        Ok(association_type) => match req.param("association_id") {
            Ok(association_id) => {
                let association_id = uuid::Uuid::from_str(association_id).expect("file uuid parse");
                let mut file = File::new(
                    u.key,
                    u.organization_key,
                    AssociationType::from_str(association_type).expect("Valid association type"),
                    association_id,
                    "".to_owned(),
                    "".to_owned(),
                    "".to_owned(),
                    "".to_owned(),
                    "".to_owned(),
                    "".to_owned(),
                    0,
                );
                file.key = uuid::Uuid::nil();
                Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::HTML)
                    .body(FileTemplate::new(file, u).render_string())
                    .build())
            }
            Err(e) => {
                println!("{:?}", e);
                Ok(tide::Response::builder(tide::StatusCode::NotFound)
                    .content_type(mime::HTML)
                    .body(NotFoundTemplate::new().render_string())
                    .build())
            }
        },
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
    match req.param("file_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("File uuid parse");
            let file = get_file(&mut conn, s_uuid).await;

            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(FileTemplate::new(file, u).render_string())
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

pub async fn get_file_fs(association_type: String, association_key: String, name: String) -> Result<reqwest::Response, minio_rsc::error::Error> {
    let bucket_name = get_bucket_name(association_type, association_key);

    let s3_url = std::env::var("S3_URL")
    .expect("Missing `S3_URL` env variable, needed for running the server");
    let static_provider = StaticProvider::new(
        "Y8oJ5TXXk659r0a8Hxlh",
        "J0S5VgsVbI9u77Jtwnihht1ZkWn1OTKAvGEpptBr",
        None,
    );
    let minio_client = Minio::builder()
    .endpoint(s3_url)
    .provider(static_provider)
    .secure(false)
    .build()
    .unwrap();
    minio_client.get_object(bucket_name, name).await
}

pub async fn insert(req: Request<State>) -> tide::Result {
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };
    let mime = req.content_type().unwrap();
    let mut conn = match req.state().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            println!("DB ERROR");
            return Ok(
                tide::Response::builder(tide::StatusCode::InternalServerError)
                    .content_type(mime::PLAIN)
                    .body(e.to_string())
                    .build(),
            )
        }
    };
    
    if mime.essence().to_string() == "multipart/form-data" {
        let boundary = mime.param("boundary").unwrap().to_string();
        let mut body = BufferedBytesStream { inner: req };
        let mut multipart = multer::Multipart::new(&mut body, boundary);
        let mut key = "".to_string();
        let mut name = "".to_string();
        let mut description = "".to_string();
        let mut format = "".to_string();
        let mut tags = "".to_string();
        let mut url = "".to_string();
        let mut organization_key = "".to_string();
        let mut association_type = "".to_string();
        let mut association_key = "".to_string();
        let mut hash = "".to_string();
        let mut size = 0;
        
        let mut buffer = vec![];
        while let Some(mut field) = multipart.next_field().await.expect("next field") {
            let f_name = field.name().clone().expect("get field name");
            println!("{f_name}");
            if f_name == "name" {
                name = field.text().await.expect("name multi field");
            } else if f_name == "key" {
                key = field.text().await.expect("key multi field");
            } else if f_name == "description" {
                description = field.text().await.expect("description multi field");
            } else if f_name == "tags" {
                tags = field.text().await.expect("tags field");
            } else if f_name == "organization_key" {
                organization_key = field.text().await.expect("organization_key multi field");
            } else if f_name == "association_type" {
                association_type = field.text().await.expect("association_type multi field");
            } else if f_name == "association_key" {
                association_key = field.text().await.expect("association_key multi field");
            } else if f_name == "hash" {
                hash = field.text().await.expect("hash multi field");
            } else if f_name == "size" {
                size = field.text().await.expect("size multi field").parse::<i64>().expect("Valid int64");
            } else if f_name == "url" {
                url = field.text().await.expect("url multi field");
            } else if f_name == "format" {
                format = field.text().await.expect("format field");
            } else if f_name == "file" {
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .expect("read in chunk from multipart stream")
                {
                    buffer.write_all(&chunk).await.expect("Write to s3buffer");
                }
            }
        }
        let s3_url = std::env::var("S3_URL")
        .expect("Missing `S3_URL` env variable, needed for running the server");
        let static_provider = StaticProvider::new(
            "Y8oJ5TXXk659r0a8Hxlh",
            "J0S5VgsVbI9u77Jtwnihht1ZkWn1OTKAvGEpptBr",
            None,
        );
        let minio_client = Minio::builder()
        .endpoint(s3_url)
        .provider(static_provider)
        .secure(false)
        .build()
        .unwrap();
      
        let bucket_name = get_bucket_name(association_type.clone(), association_key.clone());
        minio_client.make_bucket(bucket_name.clone(), true).await.unwrap_or_else(|_e| "Already Exists".to_string());
        minio_client.put_object(bucket_name, name.clone(), buffer.into()).await.expect("Put buffer");
        let mut s = File::new(
            uuid::Uuid::from_str(claims.key.as_str()).expect("user key exists"),
            uuid::Uuid::from_str(organization_key.as_str()).expect("organization_key exists"),
            AssociationType::from_str(association_type.as_str()).expect("Valid association type"),
            uuid::Uuid::from_str(association_key.as_str()).expect("association_key exists"),
            url,
            hash,
            name.clone(),
            description,
            tags,
            format,
            size,
        );
        let key = uuid::Uuid::from_str(key.as_str()).expect("user key exists");
        if !key.is_nil() {
            s.key = key;
            // updo0t
        } else {
            insert_file(&mut conn, &s).await;
            let organization_key =
                uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
            post_file_create(
                &mut conn,
                claims.matrix_home_server,
                claims.matrix_user_id,
                organization_key, 
                claims.matrix_access_token,
                &s,
            )
            .await
            .expect("Posting to matrix");
        }
        return Ok(tide::Redirect::new("/").into());
    }
    Ok(tide::Response::builder(tide::StatusCode::BadRequest)
        .content_type(mime::JSON)
        .body("{'error': 'invalid multipart body'}")
        .build())
}

fn get_bucket_name(association_type: String, association_key:String) -> String {
    format!("{}-{}-fs", association_type.to_lowercase(), association_key.to_lowercase())
}
// data types

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type)]
pub enum AssociationType {
    Organization,
    Project,
    Task,
    Entity,
    Contact,
    Milestone,
    User,
}

impl FromStr for AssociationType {
    type Err = ();
    fn from_str(input: &str) -> Result<AssociationType, Self::Err> {
        match input {
            "Organization" => Ok(AssociationType::Organization),
            "Project" => Ok(AssociationType::Project),
            "Task" => Ok(AssociationType::Task),
            "Entity" => Ok(AssociationType::Entity),
            "Contact" => Ok(AssociationType::Contact),
            "Milestone" => Ok(AssociationType::Milestone),
            "User" => Ok(AssociationType::User),
            _ => Ok(AssociationType::Organization),
        }
    }
}

impl ToString for AssociationType {
    fn to_string(&self) -> String {
        match self {
            AssociationType::Organization => "Organization".to_owned(),
            AssociationType::Project => "Project".to_owned(),
            AssociationType::Task => "Task".to_owned(),
            AssociationType::Entity => "Entity".to_owned(),
            AssociationType::Contact => "Contact".to_owned(),
            AssociationType::Milestone => "Milestone".to_owned(),
            AssociationType::User => "User".to_owned(),
        }
    }
}

impl Into<AssociationType> for i16 {
    fn into(self) -> AssociationType {
        match self {
            0 => AssociationType::Organization,
            1 => AssociationType::Project,
            2 => AssociationType::Task,
            3 => AssociationType::Entity,
            4 => AssociationType::Contact,
            5 => AssociationType::Milestone,
            6 => AssociationType::User,
            _ => AssociationType::Organization,
        }
    }
}

impl From<AssociationType> for i16 {
    fn from(t: AssociationType) -> Self {
        match t {
            AssociationType::Organization => 0,
            AssociationType::Project => 1,
            AssociationType::Task => 2,
            AssociationType::Entity => 3,
            AssociationType::Contact => 4,
            AssociationType::Milestone => 5,
            AssociationType::User => 6,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct File {
    pub key: uuid::Uuid,
    pub owner_key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub association_type: AssociationType,
    pub association_key: uuid::Uuid,
    pub url: String,
    pub hash: String,
    pub name: String,
    pub description: String,
    pub tags: String,
    pub format: String,
    pub size: i64,
    pub created: i64,
    pub updated: i64,
}

impl File {
    pub fn new(
        owner_key: uuid::Uuid,
        organization_key: uuid::Uuid,
        association_type: AssociationType,
        association_key: uuid::Uuid,
        url: String,
        hash: String,
        name: String,
        description: String,
        tags: String,
        format: String,
        size: i64,
    ) -> Self {
        let key = Uuid::new_v4();
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            owner_key,
            organization_key,
            association_type,
            association_key,
            url,
            hash,
            name,
            description,
            tags,
            format,
            size,
            created,
            updated,
        }
    }
}

#[derive(Template)]
#[template(path = "file.html")]
pub struct FileTemplate {
    file: File,
    user: crate::user::User,
}

impl<'a> FileTemplate {
    pub fn new(file: File, user: crate::user::User) -> Self {
        return Self { file, user };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
#[derive(Template)]
#[template(path = "xls_editor.html")]
pub struct XLSEditorTemplate {
    file: File,
    user: crate::user::User,
}

impl<'a> XLSEditorTemplate {
    pub fn new(file: File, user: crate::user::User) -> Self {
        return Self { file, user };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}