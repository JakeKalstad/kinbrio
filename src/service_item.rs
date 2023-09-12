use std::str::FromStr;

use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use sqlx::postgres::PgQueryResult;
use strum_macros::EnumIter;
use strum::IntoEnumIterator;
use uuid::Uuid;

use tide::{http::mime, Request};

use crate::home::NotFoundTemplate;
use crate::organization::{get_organization, Organization};
use crate::{State, user};
 
// SQL STUFF

pub async fn get_organization_service_items(conn: &mut PoolConnection<Postgres>, organization_key: uuid::Uuid) -> Vec<ServiceItem> {
    let records = sqlx::query!(
        "select  key, owner_key, organization_key, external_accounting_id, name, description, value, currency, service_item_type, service_value_type, expenses, created, updated from service_items where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut service_items = Vec::<ServiceItem>::new();
    for service_item in records { 
        let service_item_type: ServiceItemType = service_item.service_item_type.expect("service_item_type exists").into();
        let service_value_type: ServiceValueType = service_item.service_value_type.expect("service_value_type exists").into();
        let svc = ServiceItem {
            key: service_item.key.expect("key exists"),
            organization_key: service_item.organization_key.expect("organization_key"), 
            external_accounting_id: service_item.external_accounting_id.expect("external_accounting_id"),
            owner_key: service_item.owner_key.expect("owner_key exists"),
            name: service_item.name.expect("name exists"),
            description: service_item.description.expect("description exists"), 
            value: service_item.value.expect("value exists"),
            currency: service_item.currency.expect("currency exists"), 
            service_item_type,
            service_value_type,
            expenses: service_item.expenses.expect("expenses exists"),
            created: service_item.created.expect("created exists"),
            updated: service_item.updated.expect("updated exists"),
        };
        service_items.push(svc);
    }
    return service_items
}

pub async fn get_service_item(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> ServiceItem {
    let service_item = sqlx::query!(
        "select key, owner_key, organization_key, external_accounting_id, name, description, value, currency, service_item_type, service_value_type, expenses, created, updated from service_items where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select service_item by key"); 
    
    let service_item_type: ServiceItemType = service_item.service_item_type.expect("service_item_type exists").into();
    let service_value_type: ServiceValueType = service_item.service_value_type.expect("service_value_type exists").into();
    ServiceItem {
        key: service_item.key.expect("key exists"),
        organization_key: service_item.organization_key.expect("organization_key"), 
        external_accounting_id: service_item.external_accounting_id.expect("external_accounting_id"), 
        owner_key: service_item.owner_key.expect("owner_key exists"),
        name: service_item.name.expect("name exists"),
        description: service_item.description.expect("description exists"),
        value: service_item.value.expect("value exists"),
        currency: service_item.currency.expect("currency exists"), 
        service_item_type,
        service_value_type,
        expenses: service_item.expenses.expect("expenses exists"),
        created: service_item.created.expect("created exists"),
        updated: service_item.updated.expect("updated exists"),
    }
}

async fn delete_service_item(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid, owner_key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM service_items where owner_key=$1 AND key=$2", owner_key, key)
    .execute(conn)
    .await;
}

async fn update_service_item(conn: &mut PoolConnection<Postgres>, service_item: &ServiceItem) {
    sqlx::query!("UPDATE service_items SET name=$1, description=$2, value=$3, currency=$4, service_item_type=$5, service_value_type=$6, expenses=$7, external_accounting_id=$8 where key=$9", 
        &service_item.name,
        &service_item.description,
        &service_item.value,
        service_item.currency,
        service_item.service_item_type as i16,
        service_item.service_value_type as i16,
        &service_item.expenses,
        &service_item.external_accounting_id,
        service_item.key,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}
pub async fn insert_service_item(conn: &mut PoolConnection<Postgres>, new_service_item: &ServiceItem) {
    sqlx::query!("INSERT INTO service_items (key, organization_key, owner_key, name, description, value, currency, service_item_type, service_value_type, expenses, external_accounting_id, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)", 
        new_service_item.key,
        new_service_item.organization_key, 
        new_service_item.owner_key,
        new_service_item.name,
        new_service_item.description,
        new_service_item.value, 
        new_service_item.currency, 
        new_service_item.service_item_type as i16, 
        new_service_item.service_value_type as i16, 
        &new_service_item.expenses, 
        &new_service_item.external_accounting_id,
        new_service_item.created,
        new_service_item.updated,
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
    let mut service_item= ServiceItem::new( u.organization_key, String::default(), u.key,  "".to_owned(), "".to_owned(), 0, "USD".to_string(),  ServiceItemType::Service, ServiceValueType::Full, vec![]);
    service_item.key = uuid::Uuid::nil();
    let mut conn = req.state().db_pool.acquire().await?;
    let organization = get_organization(&mut conn, u.organization_key).await;
    Ok(tide::Response::builder(tide::StatusCode::Ok)
    .content_type(mime::HTML)
    .body(
        ServiceItemTemplate::new(
            service_item,
            organization,
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
    match req.param("service_item_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("ServiceItem uuid parse");
            match delete_service_item(&mut conn, s_uuid, u.key).await {
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

    match req.param("service_item_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("ServiceItem uuid parse");
            let service_item = get_service_item(&mut conn, s_uuid).await;
            let organization = get_organization(&mut conn, u.organization_key).await;
            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    ServiceItemTemplate::new(
                        service_item,
                        organization,
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
    let umd: Result<ServiceItem, tide::Error> = req.body_json().await;
    let _claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
    match umd {
        Ok(service_item) => {
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
            if service_item.key == uuid::Uuid::nil() { 
                let s = ServiceItem::new(service_item.organization_key,service_item.external_accounting_id, service_item.owner_key, service_item.name, service_item.description,
                     service_item.value, service_item.currency, service_item.service_item_type, service_item.service_value_type, service_item.expenses);
                insert_service_item(&mut conn, &s).await;
                //let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                // post_service_item_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id, organization_key, claims.matrix_access_token, &s).await.expect("Posting to matrix");
                let j = serde_json::to_string(&s).expect("To JSON");
                return Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build());
            }
            
            update_service_item(&mut conn, &service_item).await;
            let j = serde_json::to_string(&service_item).expect("To JSON");
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

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, EnumIter)]
pub enum ServiceValueType {
    Hourly,
    Milestone,
    Full,
}

impl Into<ServiceValueType> for i16 {
    fn into(self) -> ServiceValueType {
        match self {
            0 => ServiceValueType::Hourly,
            1 => ServiceValueType::Milestone,
            2 => ServiceValueType::Full,
            _ => ServiceValueType::Hourly
        }
    }
}

impl FromStr for ServiceValueType {
    type Err = ();
    fn from_str(input: &str) -> Result<ServiceValueType, Self::Err> {
        match input {
            "Hourly Rate" => Ok(ServiceValueType::Hourly),
            "Milestone Completion" => Ok(ServiceValueType::Milestone),
            "Upon Completion" => Ok(ServiceValueType::Full),
            "Full" => Ok(ServiceValueType::Full),
            _ => Ok(ServiceValueType::Hourly),
        }
    }
}

impl ToString for ServiceValueType {
    fn to_string(&self) -> String {
        match self {
            ServiceValueType::Hourly => "Hourly Rate".to_string(),
            ServiceValueType::Milestone => "Milestone Completion".to_string(),
            ServiceValueType::Full => "Upon Completion".to_string(),
        }
    }
}

impl From<ServiceValueType> for i16 {
    fn from(t: ServiceValueType) -> Self {
        match t {
            ServiceValueType::Hourly => 0,
            ServiceValueType::Milestone => 1 ,
            ServiceValueType::Full => 2 
        }
    }
}
#[derive(PartialEq, Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type, EnumIter)]
pub enum ServiceItemType {
    Service,
    Item,
}

impl ToString for ServiceItemType {
    fn to_string(&self) -> String {
        match self {
            ServiceItemType::Service => "Service".to_string(),
            ServiceItemType::Item => "Item".to_string(),
        }
    }
}

impl Into<ServiceItemType> for i16 {
    fn into(self) -> ServiceItemType {
        match self {
            0 => ServiceItemType::Service,
            1 => ServiceItemType::Item, 
            _ => ServiceItemType::Service
        }
    }
}

impl From<ServiceItemType> for i16 {
    fn from(t: ServiceItemType) -> Self {
        match t {
            ServiceItemType::Service => 0,
            ServiceItemType::Item => 1 , 
        }
    }
} 

#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceItem {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub external_accounting_id: String, 
    pub owner_key: uuid::Uuid,
    pub name: String,
    pub description: String, 
    
    pub value: i64,
    pub currency: String,
    
    pub service_item_type: ServiceItemType,
    pub service_value_type: ServiceValueType,
    pub expenses: Vec::<uuid::Uuid>, 
    
    pub created: i64,
    pub updated: i64,
}
 
impl ServiceItem {
    pub fn new(
        organization_key: uuid::Uuid,
        external_accounting_id: String,
        owner_key: uuid::Uuid,
        name: String,
        description: String,
        value: i64,
        currency: String,
        service_item_type: ServiceItemType,
        service_value_type: ServiceValueType,
        expenses: Vec::<uuid::Uuid>, 
    ) -> Self {
        let key = Uuid::new_v4();
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            organization_key,
            external_accounting_id,
            owner_key,
            name,
            description,
            value,
            currency,
            service_item_type,
            service_value_type,
            expenses,
            created, 
            updated,
        }
    }
}

#[derive(Template)]
#[template(path = "service_item.html")]
pub struct ServiceItemTemplate {
    service_item: ServiceItem,
    organization: Organization,
    user: crate::user::User,
}

impl<'a> ServiceItemTemplate {
    pub fn new(
        service_item: ServiceItem,
        organization: Organization,
        user: crate::user::User,
    ) -> Self {
        return Self {
            service_item,
            organization,
            user,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}
