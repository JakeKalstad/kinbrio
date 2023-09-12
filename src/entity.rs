use std::str::FromStr;

use askama::Template;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use tide::{http::mime, Request};

use crate::akaunting::InvoiceData;
use crate::home::NotFoundTemplate;
use crate::matrix::{post_entity_create, post_contact_create};
use crate::{State, file, note, user};

pub async fn get_organization_entitys(conn: &mut PoolConnection<Postgres>, organization_key: uuid::Uuid) -> Vec<Entity> {
    let records = sqlx::query!(
        "select 
        key,
        organization_key,
        external_accounting_id,
        owner_key,
        name,
        description, 
        matrix_room_url, 
        web_url, 
        avatar_url, 
        entity_type, 
        address_primary, 
        address_unit, 
        city, 
        state, 
        zip_code, 
        country,
        created, 
        updated from entitys where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select entitys by organization key");
    let mut entitys = Vec::<Entity>::new();
    for entity in records {
        let e_type: EntityType = entity.entity_type.expect("entity_type exists").into();
        let brd = Entity {
            key: entity.key.expect("key exists"),
            organization_key: entity.organization_key.expect("organization_key"), 
            external_accounting_id: entity.external_accounting_id.expect("external_accounting_id"), 
            owner_key: entity.owner_key.expect("owner_key exists"),
            name: entity.name.expect("name exists"),
            description: entity.description.expect("description exists"),
            matrix_room_url: entity.matrix_room_url.expect("matrix_room_url exists"),
            web_url: entity.web_url.expect("web_url exists"),
            avatar_url: entity.avatar_url.expect("avatar_url exists"), 
            entity_type: e_type, 
            address_primary: entity.address_primary.expect("address_primary exists"), 
            address_unit: entity.address_unit.expect("address_unit exists"), 
            city: entity.city.expect("city exists"), 
            state: entity.state.expect("state exists"), 
            zip_code: entity.zip_code.expect("zip_code exists"), 
            country: entity.country.expect("country exists"),
            created: entity.created.expect("created exists"),
            updated: entity.updated.expect("updated exists"),
        };
        entitys.push(brd);
    }
    return entitys
}

pub async fn get_user_entitys(conn: &mut PoolConnection<Postgres>, user_key: uuid::Uuid) -> Vec<Entity> {
    let records = sqlx::query!(
        "select 
        key,
        organization_key,
        external_accounting_id,
        owner_key,
        name,
        description, 
        matrix_room_url, 
        web_url, 
        avatar_url, 
        entity_type, 
        address_primary, 
        address_unit, 
        city, 
        state, 
        zip_code, 
        country,
        created, 
        updated from entitys where owner_key = $1",
        user_key
    )
    .fetch_all(conn)
    .await
    .expect("Select project by key");
    let mut entitys = Vec::<Entity>::new();
    for entity in records {
        let e_type: EntityType = entity.entity_type.expect("entity_type exists").into();
        let brd: Entity = Entity {
            key: entity.key.expect("key exists"),
            external_accounting_id: entity.external_accounting_id.expect("external_accounting_id"), 
            organization_key: entity.organization_key.expect("organization_key"), 
            owner_key: entity.owner_key.expect("owner_key exists"),
            name: entity.name.expect("name exists"),
            description: entity.description.expect("description exists"),
            matrix_room_url: entity.matrix_room_url.expect("matrix_room_url exists"),
            web_url: entity.web_url.expect("web_url exists"),
            avatar_url: entity.avatar_url.expect("avatar_url exists"), 
            entity_type: e_type, 
            address_primary: entity.address_primary.expect("address_primary exists"), 
            address_unit: entity.address_unit.expect("address_unit exists"), 
            city: entity.city.expect("city exists"), 
            state: entity.state.expect("state exists"), 
            zip_code: entity.zip_code.expect("zip_code exists"), 
            country: entity.country.expect("country exists"),
            created: entity.created.expect("created exists"),
            updated: entity.updated.expect("updated exists"),
        };
        entitys.push(brd);
    }
    return entitys
}

pub async fn get_entity(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Entity {
    let entity = sqlx::query!(
        "select 
        key,
        organization_key,
        external_accounting_id,
        owner_key,
        name,
        description, 
        matrix_room_url, 
        web_url, 
        avatar_url, 
        entity_type, 
        address_primary, 
        address_unit, 
        city, 
        state, 
        zip_code, 
        country,
        created, 
        updated from entitys where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select entity by key");

    let e_type: EntityType = entity.entity_type.expect("entity_type exists").into();
    Entity {
        key: entity.key.expect("key exists"),
        organization_key: entity.organization_key.expect("organization_key"), 
        external_accounting_id: entity.external_accounting_id.expect("external_accounting_id"), 
        owner_key: entity.owner_key.expect("owner_key exists"),
        name: entity.name.expect("name exists"),
        description: entity.description.expect("description exists"),
        matrix_room_url: entity.matrix_room_url.expect("matrix_room_url exists"),
        web_url: entity.web_url.expect("web_url exists"),
        avatar_url: entity.avatar_url.expect("avatar_url exists"), 
        entity_type: e_type, 
        address_primary: entity.address_primary.expect("address_primary exists"), 
        address_unit: entity.address_unit.expect("address_unit exists"), 
        city: entity.city.expect("city exists"), 
        state: entity.state.expect("state exists"), 
        zip_code: entity.zip_code.expect("zip_code exists"), 
        country: entity.country.expect("country exists"),
        created: entity.created.expect("created exists"),
        updated: entity.updated.expect("updated exists"),
    }
}

async fn delete_entity(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid, owner_key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM entitys where owner_key=$1 AND key=$2", owner_key, key)
    .execute(conn)
    .await;
}

async fn update_entity(conn: &mut PoolConnection<Postgres>, entity: &Entity) {
    sqlx::query!("UPDATE entitys SET name=$1, description=$2, matrix_room_url=$3, 
    web_url=$4, avatar_url=$5, entity_type=$6, address_primary=$7, address_unit=$8, 
    city=$9, state=$10, zip_code=$11, country=$12, external_accounting_id=$13 where key=$14",
    &entity.name,
    &entity.description,
    &entity.matrix_room_url,
    &entity.web_url,
    &entity.avatar_url,
    entity.entity_type as i16,
    &entity.address_primary,
    &entity.address_unit,
    &entity.city,
    &entity.state,
    &entity.zip_code,
    &entity.country,
    &entity.external_accounting_id,
    entity.key,
)
.execute(conn)
.await
.expect("Insert Success");
}

pub async fn insert_entity(conn: &mut PoolConnection<Postgres>, new_entity: &Entity) {
    sqlx::query!("INSERT INTO entitys (
        key,
        organization_key,
        external_accounting_id,
        owner_key,
        name,
        description, 
        matrix_room_url, 
        web_url, 
        avatar_url, 
        entity_type, 
        address_primary, 
        address_unit, 
        city, 
        state, 
        zip_code, 
        country,
        created, 
        updated
    ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)", 
        new_entity.key,
        new_entity.organization_key, 
        new_entity.external_accounting_id, 
        new_entity.owner_key,
        &new_entity.name,
        &new_entity.description,
        &new_entity.matrix_room_url,
        &new_entity.web_url,
        &new_entity.avatar_url,
        new_entity.entity_type as i16,
        &new_entity.address_primary,
        &new_entity.address_unit,
        &new_entity.city,
        &new_entity.state,
        &new_entity.zip_code,
        &new_entity.country,
        new_entity.created,
        new_entity.updated,
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
    let mut entity= Entity::new( u.organization_key, "".to_owned(), u.key,  "".to_owned(), "".to_owned(), "".to_owned(),"".to_owned(),"".to_owned(), EntityType::Client,"".to_owned(),"".to_owned(),"".to_owned(),"".to_owned(),"".to_owned(),"".to_owned());
    entity.key = uuid::Uuid::nil();
    Ok(tide::Response::builder(tide::StatusCode::Ok)
    .content_type(mime::HTML)
    .body(
        EntityTemplate::new(
            entity,
            vec![],
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
    match req.param("entity_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("Entity uuid parse");
            match delete_entity(&mut conn, s_uuid, u.key).await {
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


pub async fn get_invoices(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };

    match req.param("entity_id") {
        Ok(key) => {

            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("Entity uuid parse");
            let entity = get_entity(&mut conn, s_uuid).await;
            
            let options =  crate::akaunting::get_akaunting_options(&mut conn, u.organization_key).await.expect("get options");
            let invoices = options.list_invoices().await;
            let accounting_id = match req.param("external_id") {
                Ok(id)=> id,
                Err(e) => {
                    println!("{:?}", e);
                    return Ok(tide::Response::builder(tide::StatusCode::NotFound)
                        .content_type(mime::HTML)
                        .body(NotFoundTemplate::new().render_string())
                        .build())
                }
            };
            let mut invoice_list = vec![];
            for i in invoices.data.clone() { 
                let act_id: i64 = accounting_id.parse().expect("String is i64");
                if i.contact_id.expect("contact id exists") == act_id {
                    invoice_list.push(i);
                }
            }
            Ok(tide::Response::builder(tide::StatusCode::Ok)
            .content_type(mime::HTML)
            .body(
                InvoiceListTemplate::new(
                    entity,
                    invoice_list,
                    u,
                )
                .render_string(),
            )
            .build())
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

    match req.param("entity_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("Entity uuid parse");
            let entity = get_entity(&mut conn, s_uuid).await;
            let contacts = get_contacts_by_entity(&mut conn, s_uuid).await;

            let notes = note::get_associated_notes(&mut conn, crate::file::AssociationType::Entity, s_uuid).await;
            let files = file::get_associated_files(&mut conn, crate::file::AssociationType::Entity, s_uuid).await;

            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    EntityTemplate::new(
                        entity,
                        contacts,
                        u,
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
    let umd: Result<Entity, tide::Error> = req.body_json().await;
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
    match umd {
        Ok(entity) => {
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

            if entity.key == uuid::Uuid::nil() {
                let s = Entity::new(entity.organization_key, "".to_owned(), entity.owner_key, entity.name, entity.description, entity.matrix_room_url, entity.web_url, entity.avatar_url, entity.entity_type, entity.address_primary, entity.address_unit, entity.city, entity.state, entity.zip_code, entity.country);
                insert_entity(&mut conn, &s).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                post_entity_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id,  organization_key, claims.matrix_access_token, &s).await.expect("Posting to matrix");
                let j = serde_json::to_string(&s).expect("To JSON");
                Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())
            } else {
                update_entity(&mut conn, &entity).await;
                let j = serde_json::to_string(&entity).expect("To JSON");
                Ok(tide::Response::builder(tide::StatusCode::Ok)
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


pub async fn add_contact(req: Request<State>) -> tide::Result {
    let u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    match req.param("entity_id") {
        Ok(k) => {
            let entity_uuid = uuid::Uuid::from_str(k).expect("entity key supplied");
            let mut contact=       Contact::new(
                entity_uuid,
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                vec![],
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),
                "".to_owned(),"".to_owned(),
                "".to_owned(),
               );
            contact.key = uuid::Uuid::nil();
           Ok(tide::Response::builder(tide::StatusCode::Ok)
           .content_type(mime::HTML)
           .body(
               ContactTemplate::new(
                   contact,
                   u,
                   vec!(),
                   vec!(),
               )
               .render_string(),
           )
           .build())
        },
        Err(_) => todo!(),
    } 
}

pub async fn delete_contact(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM contacts where key=$1",  key)
    .execute(conn)
    .await;
} 

pub async fn get_contacts_by_entity(conn: &mut PoolConnection<Postgres>, entity_key: uuid::Uuid) -> Vec::<Contact> {
    let contact_records = sqlx::query!(
        "select 
        c.key,
        c.external_accounting_id,
        c.entity_key,
        c.first_name,
        c.middle_initial,
        c.last_name,
        c.description, 
        c.position, 
        c.email, 
        c.phone, 
        c.secondary_email, 
        c.secondary_phone, 
        c.matrix_user_id,
        c.web_url, 
        c.avatar_url, 
        c.social_urls, 
        c.address_primary, 
        c.address_unit, 
        c.city, 
        c.state, 
        c.zip_code, 
        c.country,
        c.created, 
        c.updated from contacts c
        where c.entity_key = $1",
        entity_key
    )
    .fetch_all(conn)
    .await
    .expect("Select contacts by organization_key");
    let mut contacts = vec![];
    for contact in contact_records {
        contacts.push(Contact {
            key: contact.key.expect("key exists"),
            external_accounting_id: contact.external_accounting_id.expect("external_accounting_id"), 
            entity_key: contact.entity_key.expect("entity_key"), 
            first_name: contact.first_name.expect("first_name exists"),
            middle_initial: contact.middle_initial.expect("middle_initial exists"),
            last_name: contact.last_name.expect("last_name exists"),
            position: contact.position.expect("position exists"),
            description: contact.description.expect("description exists"),
            email: contact.email.expect("email exists"),
            phone: contact.phone.expect("phone exists"),
            secondary_email: contact.secondary_email.expect("secondary_email exists"),
            secondary_phone: contact.secondary_phone.expect("secondary_phone exists"),
            web_url: contact.web_url.expect("web_url exists"),
            avatar_url: contact.avatar_url.expect("avatar_url exists"), 
            social_urls: contact.social_urls.expect("social_urls exists"), 
            matrix_user_id: contact.matrix_user_id.expect("matrix_user_id exists"), 
            
            address_primary: contact.address_primary.expect("address_primary exists"), 
            address_unit: contact.address_unit.expect("address_unit exists"), 
            city: contact.city.expect("city exists"), 
            state: contact.state.expect("state exists"), 
            zip_code: contact.zip_code.expect("zip_code exists"), 
            country: contact.country.expect("country exists"),
            created: contact.created.expect("created exists"),
            updated: contact.updated.expect("updated exists"),
        });
    }
    contacts
}

pub async fn get_contacts(conn: &mut PoolConnection<Postgres>, organization_key: uuid::Uuid) -> Vec::<Contact> {
    let contact_records = sqlx::query!(
        "select 
        c.key,
        c.external_accounting_id,
        c.entity_key,
        c.first_name,
        c.middle_initial,
        c.last_name,
        c.description, 
        c.position, 
        c.email, 
        c.phone, 
        c.secondary_email, 
        c.secondary_phone, 
        c.matrix_user_id,
        c.web_url, 
        c.avatar_url, 
        c.social_urls, 
        c.address_primary, 
        c.address_unit, 
        c.city, 
        c.state, 
        c.zip_code, 
        c.country,
        c.created, 
        c.updated from contacts c
        inner join entitys e on e.key = c.entity_key 
        where e.organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select contacts by organization_key");
    let mut contacts = vec![];
    for contact in contact_records {
        contacts.push(Contact {
            key: contact.key.expect("key exists"),
            external_accounting_id: contact.external_accounting_id.expect("external_accounting_id"), 
            entity_key: contact.entity_key.expect("entity_key"), 
            first_name: contact.first_name.expect("first_name exists"),
            middle_initial: contact.middle_initial.expect("middle_initial exists"),
            last_name: contact.last_name.expect("last_name exists"),
            position: contact.position.expect("position exists"),
            description: contact.description.expect("description exists"),
            email: contact.email.expect("email exists"),
            phone: contact.phone.expect("phone exists"),
            secondary_email: contact.secondary_email.expect("secondary_email exists"),
            secondary_phone: contact.secondary_phone.expect("secondary_phone exists"),
            web_url: contact.web_url.expect("web_url exists"),
            avatar_url: contact.avatar_url.expect("avatar_url exists"), 
            social_urls: contact.social_urls.expect("social_urls exists"), 
            matrix_user_id: contact.matrix_user_id.expect("matrix_user_id exists"), 
            
            address_primary: contact.address_primary.expect("address_primary exists"), 
            address_unit: contact.address_unit.expect("address_unit exists"), 
            city: contact.city.expect("city exists"), 
            state: contact.state.expect("state exists"), 
            zip_code: contact.zip_code.expect("zip_code exists"), 
            country: contact.country.expect("country exists"),
            created: contact.created.expect("created exists"),
            updated: contact.updated.expect("updated exists"),
        });
    }
    contacts
}

pub async fn get_contact(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Contact {
    let contact = sqlx::query!(
        "select 
        key,
        external_accounting_id,
        entity_key,
        first_name,
        middle_initial,
        last_name,
        description, 
        position, 
        email, 
        phone, 
        secondary_email, 
        secondary_phone, 
        matrix_user_id,
        web_url, 
        avatar_url, 
        social_urls, 
        address_primary, 
        address_unit, 
        city, 
        state, 
        zip_code, 
        country,
        created, 
        updated from contacts where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select contact by key");

    Contact {
        key: contact.key.expect("key exists"),
        external_accounting_id: contact.external_accounting_id.expect("external_accounting_id"), 
        entity_key: contact.entity_key.expect("entity_key"), 
        first_name: contact.first_name.expect("first_name exists"),
        middle_initial: contact.middle_initial.expect("middle_initial exists"),
        last_name: contact.last_name.expect("last_name exists"),
        position: contact.position.expect("position exists"),
        description: contact.description.expect("description exists"),
        email: contact.email.expect("email exists"),
        phone: contact.phone.expect("phone exists"),
        secondary_email: contact.secondary_email.expect("secondary_email exists"),
        secondary_phone: contact.secondary_phone.expect("secondary_phone exists"),
        web_url: contact.web_url.expect("web_url exists"),
        avatar_url: contact.avatar_url.expect("avatar_url exists"), 
        social_urls: contact.social_urls.expect("social_urls exists"), 
        matrix_user_id: contact.matrix_user_id.expect("matrix_user_id exists"), 
        
        address_primary: contact.address_primary.expect("address_primary exists"), 
        address_unit: contact.address_unit.expect("address_unit exists"), 
        city: contact.city.expect("city exists"), 
        state: contact.state.expect("state exists"), 
        zip_code: contact.zip_code.expect("zip_code exists"), 
        country: contact.country.expect("country exists"),
        created: contact.created.expect("created exists"),
        updated: contact.updated.expect("updated exists"),
    }
}

async fn update_contact(conn: &mut PoolConnection<Postgres>, contact: &Contact) {
    let social_urls = contact.social_urls.as_slice();
    sqlx::query!("UPDATE contacts SET first_name=$1, middle_initial=$2, last_name=$3, 
    description=$4, position=$5, email=$6, phone=$7, secondary_email=$8, 
    secondary_phone=$9, matrix_user_id=$10, web_url=$11, avatar_url=$12, social_urls=$13,
    address_primary=$14, address_unit=$15, city=$16, state=$17, zip_code=$18, country=$19, 
    external_accounting_id=$20 where key=$21",
    contact.first_name,
    &contact.middle_initial,
    &contact.last_name, 
    &contact.description,
    &contact.position, 
    &contact.email, 
    &contact.phone, 
    &contact.secondary_email, 
    &contact.secondary_phone, 
    &contact.matrix_user_id, 
    &contact.web_url, 
    &contact.avatar_url, 
    social_urls,
    &contact.address_primary, 
    &contact.address_unit, 
    &contact.city, 
    &contact.state, 
    &contact.zip_code, 
    &contact.country, 
    &contact.external_accounting_id,
    contact.key,
)
.execute(conn)
.await
.expect("Insert Success");
}

pub async fn insert_contact(conn: &mut PoolConnection<Postgres>, new_contact: &Contact) {
    let social_urls = new_contact.social_urls.as_slice();
    sqlx::query!("INSERT INTO contacts (
        key,
        external_accounting_id,
        entity_key,
        first_name,
        middle_initial,
        last_name,
        description, 
        position, 
        email, 
        phone, 
        secondary_email, 
        secondary_phone, 
        matrix_user_id,
        web_url, 
        avatar_url, 
        social_urls, 
        address_primary, 
        address_unit, 
        city, 
        state, 
        zip_code, 
        country,
        created, 
        updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)", 
        new_contact.key,
        new_contact.external_accounting_id,
        new_contact.entity_key, 
        new_contact.first_name,
        &new_contact.middle_initial,
        &new_contact.last_name, 
        &new_contact.description,
        &new_contact.position, 
        &new_contact.email, 
        &new_contact.phone, 
        &new_contact.secondary_email, 
        &new_contact.secondary_phone, 
        &new_contact.matrix_user_id, 
        &new_contact.web_url, 
        &new_contact.avatar_url, 
        social_urls,
        &new_contact.address_primary, 
        &new_contact.address_unit, 
        &new_contact.city, 
        &new_contact.state, 
        &new_contact.zip_code, 
        &new_contact.country, 
        new_contact.created,
        new_contact.updated,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}


pub async fn delete_contact_route(req: Request<State>) -> tide::Result {
    let _u = match crate::user::user_or_error(&req) {
        Ok(value) => value,
        Err(_) => return Ok(tide::Redirect::new("/login").into()),
    };
    match req.param("contact_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?;
            let s_uuid = uuid::Uuid::from_str(key).expect("contact uuid parse");
            match delete_contact(&mut conn, s_uuid).await {
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

pub async fn get_contact_route(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };

    match req.param("contact_id") {
        Ok(key) => {
            let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
            let s_uuid = uuid::Uuid::from_str(key).expect("Entity uuid parse");
            let contact = get_contact(&mut conn, s_uuid).await;

            let notes = note::get_associated_notes(&mut conn, crate::file::AssociationType::Contact, s_uuid).await;
            let files = file::get_associated_files(&mut conn, crate::file::AssociationType::Contact, s_uuid).await;
            Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    ContactTemplate::new(
                        contact,
                        u,
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

pub async fn insert_contact_route(mut req: Request<State>) -> tide::Result {
    let umd: Result<Contact, tide::Error> = req.body_json().await;
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
    
    match umd {
        Ok(contact) => {
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
            
            if contact.key == uuid::Uuid::nil() {
                let s = Contact::new(
                    contact.entity_key,
                    contact.external_accounting_id,
                    contact.first_name, 
                    contact.middle_initial, 
                    contact.last_name, 
                    contact.description, 
                    contact.position, 
                    contact.email, 
                    contact.phone, 
                    contact.secondary_email, 
                    contact.secondary_phone, 
                    contact.matrix_user_id, 
                    contact.web_url, 
                    contact.avatar_url, 
                    contact.social_urls, 
                    contact.address_primary, 
                    contact.address_unit, 
                    contact.city, 
                    contact.state, 
                    contact.zip_code, 
                    contact.country,
                    );
                insert_contact(&mut conn, &s).await;
                post_contact_create(&mut conn, claims.matrix_home_server, claims.matrix_user_id, uuid::Uuid::from_str(claims.organization_key.as_str()).expect("org key"), claims.matrix_access_token,  &s).await.expect("Posting to matrix");
                let j = serde_json::to_string(&s).expect("To JSON");
                return Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())   
            }
            update_contact(&mut conn, &contact).await;
            let j = serde_json::to_string(&contact).expect("To JSON");
            return Ok(tide::Response::builder(tide::StatusCode::Ok)
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

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type)]
pub enum EntityType {
    Client,
    Supplier,
}

impl Into<EntityType> for i16 {
    fn into(self) -> EntityType {
        match self {
            0 => EntityType::Client,
            1 => EntityType::Supplier,
            _ => EntityType::Client
        }
    }
}

impl From<EntityType> for i16 {
    fn from(t: EntityType) -> Self {
        match t {
            EntityType::Client => 0,
            EntityType::Supplier => 1 
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entity {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub external_accounting_id: String,
    pub owner_key: uuid::Uuid,
    pub matrix_room_url: String,
    pub web_url: String,
    pub avatar_url: String,
    pub entity_type: EntityType, 
    pub name: String,
    pub description: String, 

    pub address_primary:String,
    pub address_unit: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub country: String,

    pub created: i64,
    pub updated: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contact {
    pub key: uuid::Uuid,
    pub external_accounting_id: String,
    pub entity_key: uuid::Uuid,
    pub first_name: String,
    pub middle_initial: String,
    pub last_name: String,
    pub description: String,
    pub position: String, 
    pub email: String,
    pub phone: String,
    pub secondary_email: String,
    pub secondary_phone: String,
    pub matrix_user_id: String,
    pub web_url: String,
    pub avatar_url: String,
    pub social_urls: Vec<String>,
    pub address_primary:String,
    pub address_unit: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub country: String,
    pub created: i64,
    pub updated: i64,
}

impl Contact {
    pub fn new(
            entity_key:Uuid,
            external_accounting_id: String,
            first_name:String,
            middle_initial:String,
            last_name:String,
            description:String,
            position:String,
            email:String,
            phone:String,
            secondary_email:String,
            secondary_phone:String,
            matrix_user_id:String,
            web_url:String,
            avatar_url:String,
            social_urls:Vec<String>,
            address_primary:String,
            address_unit:String,
            city:String,
            state:String,
            zip_code:String,
            country:String,
    ) -> Contact {
        let key = Uuid::new_v4();
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            entity_key,
            external_accounting_id,
            first_name,
            middle_initial,
            last_name,
            description,
            position,
            email,
            phone,
            secondary_email,
            secondary_phone,
            matrix_user_id,
            web_url,
            avatar_url,
            social_urls,
            address_primary,
            address_unit,
            city,
            state,
            zip_code,
            country,
            created,
            updated,
        }
    }
}

impl Entity {
    pub fn new(
        organization_key: uuid::Uuid,
        external_accounting_id: String,
        owner_key: uuid::Uuid,
        name: String,
        description: String, 
        matrix_room_url: String, 
        web_url: String, 
        avatar_url: String, 
        entity_type: EntityType, 
        address_primary: String, 
        address_unit: String, 
        city: String, 
        state: String, 
        zip_code: String, 
        country: String,
    ) -> Entity {
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
            created, 
            updated,
            matrix_room_url, 
            web_url, 
            avatar_url, 
            entity_type, 
            address_primary, 
            address_unit, 
            city, 
            state, 
            zip_code, 
            country, 
        }
    }
}

#[derive(Template)]
#[template(path = "invoice_list.html")]
pub struct InvoiceListTemplate {
    entity: Entity,
    invoices: Vec<InvoiceData>,
    user: crate::user::User,
}

impl<'a> InvoiceListTemplate {
    pub fn new(
        entity: Entity,
        invoices: Vec<InvoiceData>,
        user: crate::user::User,
    ) -> Self {
        return Self {
            entity,
            invoices,
            user,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Template)]
#[template(path = "entity.html")]
pub struct EntityTemplate {
    entity: Entity,
    contacts: Vec::<Contact>,
    user: crate::user::User,
    notes: Vec<note::Note>,
    files: Vec<file::File>,
}
impl<'a> EntityTemplate {
    pub fn new(
        entity: Entity,
        contacts: Vec::<Contact>,
        user: crate::user::User,
        notes: Vec<note::Note>,
        files: Vec<file::File>,
    ) -> Self {
        return Self {
            entity,
            contacts,
            user,
            notes,
            files,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}

#[derive(Template)]
#[template(path = "contact.html")]
pub struct ContactTemplate {
    contact: Contact,
    user: crate::user::User,
    notes: Vec<note::Note>,
    files: Vec<file::File>,
}

impl<'a> ContactTemplate {
    pub fn new(
        contact: Contact,
        user: crate::user::User,
        notes: Vec<note::Note>,
        files: Vec<file::File>,
    ) -> Self {
        return Self {
            contact,
            user,
            notes,
            files,
        };
    }

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    }
}