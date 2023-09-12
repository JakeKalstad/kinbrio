use std::str::FromStr;

use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgQueryResult;
use sqlx::Postgres;
use uuid::Uuid;

use tide::{http::mime, Request};

use crate::home::NotFoundTemplate;
use crate::{user, State};

// SQL STUFF

pub async fn get_organization(
    conn: &mut PoolConnection<Postgres>,
    key: uuid::Uuid,
) -> Organization {
    let organization = sqlx::query!(  
        "select key, external_accounting_id, external_accounting_url, owner_key, name, description, matrix_home_server, matrix_live_support_room_url, matrix_general_room_url, domain, contact_email, created, updated from organization where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select organization by key");

    Organization {
        key: organization.key.expect("key exists"),
        external_accounting_id: organization.external_accounting_id.expect("external_accounting_id exists"),
        external_accounting_url: organization.external_accounting_url.expect("external_accounting_url exists"),
        owner_key: organization.owner_key.expect("owner_key exists"),
        name: organization.name.expect("name exists"),
        description: organization.description.expect("description exists"),
        matrix_home_server: organization.matrix_home_server.expect("matrix_home_server exists"),
        matrix_live_support_room_url: organization.matrix_live_support_room_url.expect("matrix_live_support_room_url exists"),
        matrix_general_room_url: organization.matrix_general_room_url.expect("matrix_general_room_url exists"),
        domain: organization.domain.expect("domain exists"),
        contact_email: organization.contact_email.expect("contact_email exists"),
        created: organization.created.expect("created exists"),
        updated: organization.updated.expect("updated exists"),
    }
}

async fn delete_organization(
    conn: &mut PoolConnection<Postgres>,
    key: uuid::Uuid,
    owner_key: uuid::Uuid,
) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!(
        "DELETE FROM organization where owner_key=$1 AND key=$2",
        owner_key,
        key
    )
    .execute(conn)
    .await;
}

pub(crate) async fn insert_organization(
    conn: &mut PoolConnection<Postgres>,
    new_organization: &Organization,
) {
    sqlx::query!("INSERT INTO organization (key, external_accounting_id, external_accounting_url, owner_key, name, description, matrix_home_server, matrix_live_support_room_url, matrix_general_room_url, domain, contact_email, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)", 
        new_organization.key,
        new_organization.external_accounting_id,
        new_organization.external_accounting_url,
        new_organization.owner_key,
        &new_organization.name,
        &new_organization.description,
        new_organization.matrix_home_server,
        new_organization.matrix_live_support_room_url,
        new_organization.matrix_general_room_url,
        new_organization.domain,
        new_organization.contact_email,
        new_organization.created,
        new_organization.updated,
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
    match req.param("organization_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("Organization uuid parse");
            match delete_organization(&mut conn, s_uuid, u.key).await {
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


pub async fn get(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };
    match req.param("organization_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("Organization uuid parse");
            let organization = get_organization(&mut conn, s_uuid).await;
            
            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(OrganizationTemplate::new(organization, u).render_string())
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

pub async fn update_organization(conn: &mut PoolConnection<Postgres>, organization: &Organization) {
    sqlx::query!(
        "UPDATE organization SET name=$1, description=$2, external_accounting_id=$3, external_accounting_url=$4, matrix_home_server=$5, matrix_live_support_room_url=$6, matrix_general_room_url=$7, domain=$8, contact_email=$9 where key=$10",
        &organization.name,
        &organization.description,
        organization.external_accounting_id,
        organization.external_accounting_url,
        organization.matrix_home_server,
        organization.matrix_live_support_room_url,
        organization.matrix_general_room_url,
        organization.domain,
        organization.contact_email,
        organization.key,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}
pub async fn insert(mut req: Request<State>) -> tide::Result {
    let umd: Result<Organization, tide::Error> = req.body_json().await;

    let _claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };

    match umd {
        Ok(organization) => {
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

            if organization.key == uuid::Uuid::nil() {
                let s = Organization::new(
                    Uuid::new_v4(),
                    organization.external_accounting_id,
                    organization.external_accounting_url,
                    organization.owner_key,
                    organization.name,
                    organization.description,
                    organization.matrix_home_server,
                    organization.matrix_live_support_room_url,
                    organization.matrix_general_room_url,
                    organization.domain,
                    organization.contact_email,
                );
                insert_organization(&mut conn, &s).await;
                let j = serde_json::to_string(&s).expect("To JSON");
                return Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build());
            }

            update_organization(&mut conn, &organization).await;
            let j = serde_json::to_string(&organization).expect("To JSON");
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
pub struct Organization {
    pub key: uuid::Uuid,
    pub external_accounting_id: String,
    pub external_accounting_url: String,
    pub owner_key: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub matrix_home_server: String,
    pub matrix_live_support_room_url: String,
    pub matrix_general_room_url: String,
    pub domain: String,
    pub contact_email: String,
    
    pub created: i64,
    pub updated: i64,
}

impl Organization {
    pub fn new(key: uuid::Uuid, 
        external_accounting_id: String,
        external_accounting_url: String,
        owner_key: uuid::Uuid, 
        name: String, 
        description: String,
        matrix_home_server: String,
        matrix_live_support_room_url: String,
        matrix_general_room_url: String,
        domain: String,
        contact_email: String,
    ) -> Self {
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            external_accounting_id,
            external_accounting_url,
            owner_key,
            name,
            description,
            matrix_home_server,
            matrix_live_support_room_url,
            matrix_general_room_url,
            domain,
            contact_email,
            created,
            updated,
        }
    }

    pub fn has_external_accounting(&self) -> bool {
        return self.external_accounting_id.len() > 0;
    }
}

#[derive(Template)]
#[template(path = "organization.html")]
pub struct OrganizationTemplate {
    organization: Organization,
    user: crate::user::User,
}

impl<'a> OrganizationTemplate {
    pub fn new(organization: Organization, user: crate::user::User) -> Self {
        return Self { organization, user };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
