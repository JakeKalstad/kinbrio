
use std::ops::Mul;
use std::str::FromStr;

use base64::Engine;
use base64::engine::GeneralPurposeConfig;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;
use reqwest;
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use serde_this_or_that::as_bool;
use tide::{http::mime, Request};
use askama::Template;
use tokio::join;
use uuid::Uuid;
use crate::common::BufferedBytesStream;
use crate::entity::Entity;
use crate::organization::{update_organization,get_organization};
use crate::service_item::ServiceItem;
use crate::{State, user};
 

async fn update_akaunting_options(conn: &mut PoolConnection<Postgres>, akaunting_options: &AkauntingSyncOption) {
    sqlx::query!("UPDATE akaunting_options 
    SET organization_data=$1, employee_data=$2, client_data=$3, 
    vendor_data=$4, item_data=$5,invoice_data=$6, allow_post=$7, last_sync=$8, user_name=$9, user_pass=$10, akaunting_domain=$11, akaunting_company_id=$12 where key=$13",  
    &akaunting_options.organization_data,
    &akaunting_options.employee_data,
    &akaunting_options.client_data, 
    &akaunting_options.vendor_data, 
    &akaunting_options.item_data, 
    &akaunting_options.invoice_data, 
    &akaunting_options.allow_post, 
    &akaunting_options.last_sync, 
    &akaunting_options.user_name, 
    &akaunting_options.user_pass, 
    &akaunting_options.akaunting_domain, 
    &akaunting_options.akaunting_company_id, 
    akaunting_options.key,
)
.execute(conn)
.await
.expect("Insert Success");
}

pub async fn get_akaunting_options(conn: &mut PoolConnection<Postgres>, organization_key: uuid::Uuid) -> Option<AkauntingSyncOption> {
    match sqlx::query!(
        "select key, organization_key, owner_key,  user_name, user_pass, akaunting_domain, akaunting_company_id, organization_data, employee_data, client_data, vendor_data, item_data, invoice_data, allow_post, last_sync, created, updated from akaunting_options where organization_key = $1",
        organization_key
    )
    .fetch_one(conn)
    .await {
        Ok(akaunting_options) => Some(AkauntingSyncOption {
            key: akaunting_options.key.expect("key exists"),
            organization_key: akaunting_options.organization_key.expect("organization_key"), 
            owner_key: akaunting_options.owner_key.expect("owner_key exists"),
            user_name: akaunting_options.user_name.unwrap_or_default(),
            user_pass: akaunting_options.user_pass.unwrap_or_default(),
            akaunting_domain: akaunting_options.akaunting_domain.unwrap_or_default(),
            akaunting_company_id: akaunting_options.akaunting_company_id.unwrap_or_default(),
            organization_data: akaunting_options.organization_data.expect("organization_data exists"),
            employee_data: akaunting_options.employee_data.expect("employee_data exists"),
            client_data: akaunting_options.client_data.expect("client_data exists"), 
            vendor_data: akaunting_options.vendor_data.expect("vendor_data exists"), 
            item_data: akaunting_options.item_data.expect("item_data"),
            invoice_data: akaunting_options.invoice_data.expect("invoice_data"),
            allow_post: akaunting_options.allow_post.expect("allow_post"),
            last_sync: akaunting_options.last_sync.expect("last_sync"),
            created: akaunting_options.created.expect("created exists"),
            updated: akaunting_options.updated.expect("updated exists"),
        }),
        Err(_) => {
            None
        }
    }

    
}
async fn insert_akaunting_options(conn: &mut PoolConnection<Postgres>, new_akaunting_options: &AkauntingSyncOption) {
    sqlx::query!("INSERT INTO akaunting_options (key, organization_key, owner_key,  user_name, user_pass,  akaunting_domain, akaunting_company_id, organization_data, employee_data, client_data, vendor_data, item_data, invoice_data, allow_post, last_sync, created, updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)", 
        new_akaunting_options.key,
        new_akaunting_options.organization_key, 
        new_akaunting_options.owner_key,
        new_akaunting_options.user_name,
        new_akaunting_options.user_pass,
        new_akaunting_options.akaunting_domain,
        new_akaunting_options.akaunting_company_id,
        &new_akaunting_options.organization_data,
        &new_akaunting_options.employee_data,
        &new_akaunting_options.client_data, 
        &new_akaunting_options.vendor_data, 
        &new_akaunting_options.item_data, 
        &new_akaunting_options.invoice_data, 
        &new_akaunting_options.allow_post, 
        &new_akaunting_options.last_sync, 
        new_akaunting_options.created,
        new_akaunting_options.updated,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}

pub async fn save_akaunting_options(mut req: Request<State>) -> tide::Result {
    let claims: user::UserJwtState = match user::read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        },
    };
    let umd: Result<AkauntingSyncOption, tide::Error> = req.body_json().await;
    match umd {
        Ok(akaunting_options) => {
            let mut conn = req.state().db_pool.acquire().await?;
            if akaunting_options.key == uuid::Uuid::nil() {
                let ao = akaunting_options.clone();
                let s = AkauntingSyncOption::new(akaunting_options.organization_key, akaunting_options.owner_key, akaunting_options.user_name, 
                    akaunting_options.user_pass, akaunting_options.akaunting_domain, akaunting_options.akaunting_company_id, akaunting_options.organization_data,
                     akaunting_options.employee_data, akaunting_options.client_data, akaunting_options.vendor_data, akaunting_options.item_data, akaunting_options.invoice_data, akaunting_options.allow_post,
                    akaunting_options.last_sync);
                insert_akaunting_options(&mut conn, &s).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                let mut org = get_organization(&mut conn, organization_key).await; 
                let akaunting_companys = ao.list_companys().await.data;
                for c in akaunting_companys {
                    if c.id.to_string() == ao.akaunting_company_id {
                        org.name = c.name;
                        break 
                    }
                }
                update_organization(&mut conn, &org).await;
                let j = serde_json::to_string(&s).expect("To JSON");
                Ok(tide::Response::builder(tide::StatusCode::Ok)
                    .content_type(mime::JSON)
                    .body(j)
                    .build())
            } else {
                update_akaunting_options(&mut conn, &akaunting_options).await;
                let organization_key = uuid::Uuid::from_str(claims.organization_key.as_str()).expect("organization key");
                let mut org = get_organization(&mut conn, organization_key).await; 
                let ao = akaunting_options.clone();
                let akaunting_companys = ao.list_companys().await.data;
                for c in akaunting_companys {
                    if c.id.to_string() == ao.akaunting_company_id {
                        org.name = c.name;
                        org.contact_email = c.email;
                        org.external_accounting_url = ao.akaunting_domain;
                        org.external_accounting_id = ao.akaunting_company_id;
                        break 
                    }
                }
                update_organization(&mut conn, &org).await;
                let j = serde_json::to_string(&akaunting_options).expect("To JSON");
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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AkauntingImportByID {
    import_id: String,
}

pub async fn import_item(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };
    let mime = req.content_type().unwrap();
    let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!

    if mime.essence().to_string() == "multipart/form-data" {
        let boundary = mime.param("boundary").unwrap().to_string();
        let mut body = BufferedBytesStream { inner: req };
        let mut multipart = multer::Multipart::new(&mut body, boundary);
        let mut import_id = "".to_string();
        while let Some(field) = multipart.next_field().await.expect("next field") {
            let f_name = field.name().clone().expect("get field name");
            if f_name == "import_id" {
                import_id = field.text().await.expect("import_id multi field");
            }
        } 
        let mut options = match get_akaunting_options(&mut conn, u.organization_key).await {
            Some(options) => options,
            None => {
                let s = AkauntingSyncOption::new(u.organization_key, u.key, "".to_string(),"".to_string(),"".to_string(),"".to_string(),true, true, true, true, true, true, true, 0);
                insert_akaunting_options(&mut conn, &s).await;
                s
            }
        };
        let items = options.list_items();
        if import_id.len() == 0 {
            return Ok(tide::Response::builder(tide::StatusCode::BadRequest)
                .content_type(mime::JSON)
                .body("bad import_id")
                .build());
        }
        let item = options.get_item(import_id); 
        let (items, idata) = join!(items, item);
        let idata = idata.data;
        let svc_type = match idata.type_field.as_str() {
            "Service" => crate::service_item::ServiceItemType::Service,
            "Item"=> crate::service_item::ServiceItemType::Item,
            _ => crate::service_item::ServiceItemType::Item,
        };
        let svc_item = ServiceItem::new(u.organization_key, 
            idata.id.to_string(), 
            u.key, 
            idata.name, 
            idata.description,
            idata.sale_price as i64, 
            "USD".to_string(), 
            svc_type,
            crate::service_item::ServiceValueType::Full, 
            vec![]);
            crate::service_item::insert_service_item(&mut conn, &svc_item).await;
        if options.akaunting_domain.len() == 0 {
            options.akaunting_domain = "https://app.akaunting.com/api".to_string()
        }
        let org = crate::organization::get_organization(&mut conn, u.organization_key);
        let companys = options.list_companys();
        let invoices = options.list_invoices();
        let customers = options.list_customers();
        let users = options.list_users();
        let (org, companys, invoices, customers, users) = join!(org, companys, invoices, customers, users);
        Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    AkauntingSyncOptionTemplate::new(
                        options,
                        u,
                        org,
                        companys.data,
                        items.data,
                        invoices.data,
                        customers.data,
                        users.data,
                    )
                    .render_string(),
                )
                .build())
        } else  {
            Ok(tide::Response::builder(tide::StatusCode::BadRequest)
                .content_type(mime::JSON)
                .body("{'error': 'invalid form body'}")
                .build())
        }
}

pub async fn import_customer(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };
    let mime = req.content_type().unwrap();
    let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!

    if mime.essence().to_string() == "multipart/form-data" {
        let boundary = mime.param("boundary").unwrap().to_string();
        let mut body = BufferedBytesStream { inner: req };
        let mut multipart = multer::Multipart::new(&mut body, boundary);
        let mut import_id = "".to_string();
        while let Some(field) = multipart.next_field().await.expect("next field") {
            let f_name = field.name().clone().expect("get field name");
            if f_name == "import_id" {
                import_id = field.text().await.expect("import_id multi field");
            }
        } 
        let mut options = match get_akaunting_options(&mut conn, u.organization_key).await {
            Some(options) => options,
            None => {
                let s = AkauntingSyncOption::new(u.organization_key, u.key, "".to_string(),"".to_string(),"".to_string(),"".to_string(),true, true, true, true, true, true, true, 0);
                insert_akaunting_options(&mut conn, &s).await;
                s
            }
        };
        if import_id.len() == 0 {
            return Ok(tide::Response::builder(tide::StatusCode::BadRequest)
                .content_type(mime::JSON)
                .body("bad import_id")
                .build());
        }
        let customers = options.list_customers();
        let customer = options.get_customer(import_id); 
        let (customers, customer) = join!(customers, customer);
        let idata = customer.data; 
        let ent = Entity::new(u.organization_key, 
            idata.id.to_string(),
            u.key, 
            idata.name, 
            String::default(), 
            String::default(), 
            idata.website.to_string(), 
            String::default(),
            crate::entity::EntityType::Client, 
            idata.address.to_string(),
            String::default(),
            String::default(),
            String::default(),
            String::default(),
            String::default(),
            );
            crate::entity::insert_entity(&mut conn, &ent).await;
        if options.akaunting_domain.len() == 0 {
            options.akaunting_domain = "https://app.akaunting.com/api".to_string()
        }
        let org = crate::organization::get_organization(&mut conn, u.organization_key);
        let companys = options.list_companys();
        let invoices = options.list_invoices();
        let items = options.list_items();
        let users = options.list_users();
        let (org, companys, invoices, items, users) = join!(org, companys, invoices, items, users);
        Ok(tide::Response::builder(tide::StatusCode::Ok)
                .content_type(mime::HTML)
                .body(
                    AkauntingSyncOptionTemplate::new(
                        options,
                        u,
                        org,
                        companys.data,
                        items.data,
                        invoices.data,
                        customers.data,
                        users.data,
                    )
                    .render_string(),
                )
                .build())
        } else  {
            Ok(tide::Response::builder(tide::StatusCode::BadRequest)
                .content_type(mime::JSON)
                .body("{'error': 'invalid form body'}")
                .build())
        }
}

pub async fn get_akaunting_options_page(req: Request<State>) -> tide::Result {
    let u = match crate::user::read_jwt_cookie_to_user(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(tide::Redirect::new("/login").into());
        }
    };

    let mut conn = req.state().db_pool.acquire().await?; // .await? needs to be a real connection pool error handler!!!!!!!!
    let org: crate::organization::Organization = crate::organization::get_organization(&mut conn, u.organization_key).await;
    let mut options = match get_akaunting_options(&mut conn, u.organization_key).await {
        Some(options) => options,
        None => {
            let s = AkauntingSyncOption::new(u.organization_key, u.key, "".to_string(),"".to_string(),"".to_string(),"".to_string(),true, true, true, true, true, true, true, 0);
            insert_akaunting_options(&mut conn, &s).await;
            s
        }
    };
    if options.akaunting_domain.len() == 0 {
        options.akaunting_domain = "https://app.akaunting.com/api".to_string()
    }
    let companys = options.list_companys().await;
    let items = options.list_items().await;
    let invoices = options.list_invoices().await;
    let customers = options.list_customers().await;
    let users = options.list_users().await;
    Ok(tide::Response::builder(tide::StatusCode::Ok)
        .content_type(mime::HTML)
        .body(
            AkauntingSyncOptionTemplate::new(
                options,
                u,
                org,
                companys.data,
                items.data,
                invoices.data,
                customers.data,
                users.data,
            )
            .render_string(),
        )
        .build())
} 

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct AkauntingError {
    pub message: String,
    pub status_code: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AkauntingSyncOption {
    pub key: uuid::Uuid,
    pub organization_key: uuid::Uuid,
    pub owner_key: uuid::Uuid,
    
    pub user_name: String,
    pub user_pass: String,
    pub akaunting_domain: String,
    pub akaunting_company_id: String,

    #[serde(deserialize_with = "as_bool")]
    pub organization_data: bool,
    #[serde(deserialize_with = "as_bool")]
    pub employee_data: bool,
    #[serde(deserialize_with = "as_bool")]
    pub client_data: bool,
    #[serde(deserialize_with = "as_bool")]
    pub vendor_data: bool,
    #[serde(deserialize_with = "as_bool")]
    pub item_data: bool,
    #[serde(deserialize_with = "as_bool")]
    pub invoice_data: bool,
    #[serde(deserialize_with = "as_bool")]
    pub allow_post: bool,

    pub last_sync: i64,
    pub created: i64,
    pub updated: i64,
}

impl AkauntingSyncOption {
    pub fn new(
        organization_key: uuid::Uuid,
        owner_key: uuid::Uuid,
        user_name: String,
        user_pass: String,
        akaunting_domain: String,
        akaunting_company_id: String,
        organization_data: bool,
        employee_data: bool,
        client_data: bool,
        vendor_data: bool,
        item_data: bool,
        invoice_data: bool,
        allow_post: bool,
        last_sync: i64,
    ) -> Self {
        let key = Uuid::new_v4();
        let created = chrono::Utc::now().timestamp();
        let updated = 0;
        Self {
            key,
            organization_key,
            owner_key,
            user_name,
            user_pass,
            akaunting_domain,
            akaunting_company_id,
            organization_data,
            employee_data,
            client_data,
            vendor_data,
            item_data,
            invoice_data,
            allow_post,
            last_sync,
            created, 
            updated,
        }
    }
}

#[derive(Template)]
#[template(path = "akaunting.html")]
pub struct AkauntingSyncOptionTemplate {
    akaunting_options: AkauntingSyncOption,
    user: crate::user::User,
    organization: crate::organization::Organization,
    companys: Vec<CompanyData>,
    items: Vec<ItemData>,
    invoices: Vec<InvoiceData>,
    customers: Vec<CustomerData>,
    users: Vec<UserData>,
}
impl<'a> AkauntingSyncOptionTemplate {
    pub fn new(
        akaunting_options: AkauntingSyncOption,
        user: crate::user::User,
        organization: crate::organization::Organization,
        companys: Vec<CompanyData>,
        items: Vec<ItemData>,
        invoices: Vec<InvoiceData>,
        customers: Vec<CustomerData>,
        users: Vec<UserData>,
    ) -> Self { 
        return Self {
            akaunting_options,
            user,
            organization,
            companys,
            items,
            invoices,
            customers,
            users,
        };
    }

    pub fn get_str(&self, attr: &Option<String>) -> String {
        let s = attr.clone().unwrap_or_default();
        s.clone()
    } 

    pub fn render_string(&self) -> String {
        return self.render().unwrap();
    } 
}
 
fn akaunting_unmarshal<'a, T: serde::Deserialize<'a>>(body: &'a str) -> Result<T, AkauntingError> {
    match serde_json::from_str(body) {
        Ok(v) => Ok(v),
        Err(e) => {
            let error_response: AkauntingError = serde_json::from_str(body).unwrap_or_default();
            println!("{:?}", e);
            println!("{:?}", error_response);
            Err(error_response)
        }
    }
}

impl AkauntingSyncOption {
    fn get_authorization(&self) -> String { 
        let mut buf = String::new();
        let eng = base64::engine::GeneralPurpose::new(&base64::alphabet::URL_SAFE, GeneralPurposeConfig::new());
        eng.encode_string(format!("{}:{}", self.user_name, self.user_pass), &mut buf);
        let s = format!("Basic {}", buf);
        s
    }
    
    pub fn can_sync(&self) -> bool {
        self.akaunting_domain.len() == 0 || self.user_name.len() == 0 || self.user_pass.len() == 0
    }

    pub async fn list_companys(&self) -> ListCompanys {
        if self.can_sync() {
            return ListCompanys{..Default::default()}
        }
        let client = reqwest::Client::builder().build().expect("Built client");
    
        let request = client.request(
            reqwest::Method::GET,
            format!("{}/companies",self.akaunting_domain)
        ).header("Authorization", self.get_authorization());
        let response = request.send().await.expect("Sent request");
        let body = response.text().await.expect("Receive body");
        return akaunting_unmarshal(body.as_str()).unwrap_or_default();
    }

    pub async fn get_customer(&self, id: String) -> GetContactData {
        if self.can_sync() {
            return GetContactData{..Default::default()}
        }
        let client = reqwest::Client::builder().build().expect("Built client");
    
        let request = client.request(
            reqwest::Method::GET,
            format!("{}/contacts/{}?search=type%3Acustomer",self.akaunting_domain, id)
        )
        .header("Authorization", self.get_authorization())
        .header("X-Company", self.akaunting_company_id.to_string());
    
        let response = request.send().await.expect("Sent request");
        let body: String = response.text().await.expect("Receive body");
        return akaunting_unmarshal(body.as_str()).unwrap_or_default();
    }
    pub async fn get_item(&self, id: String) -> Item {
        if self.can_sync() {
            return Item{..Default::default()}
        }
        let client = reqwest::Client::builder().build().expect("Built client");
    
        let request = client.request(
            reqwest::Method::GET,
            format!("{}/items/{}",self.akaunting_domain, id)
        ).header("Authorization", self.get_authorization());
        let response = request.send().await.expect("Sent request");
        let body: String = response.text().await.expect("Receive body");
        return akaunting_unmarshal(body.as_str()).unwrap_or_default();
    }

    pub async fn list_items(&self) -> ListItems {
        if self.can_sync() {
            return ListItems{..Default::default()}
        }
        let client = reqwest::Client::builder().build().expect("Built client");
    
        let request = client.request(
            reqwest::Method::GET,
            format!("{}/items",self.akaunting_domain)
        ).header("Authorization", self.get_authorization());
        let response = request.send().await.expect("Sent request");
        let body: String = response.text().await.expect("Receive body");
        return akaunting_unmarshal(body.as_str()).unwrap_or_default();
    }
    pub async fn list_invoices(&self) -> ListInvoices {
        if self.can_sync() {
            return ListInvoices{..Default::default()}
        }
        let client = reqwest::Client::builder().build().expect("Built client");
    
        let request = client.request(
            reqwest::Method::GET,
            format!("{}/documents?search=type:invoice&page=1&limit=50",self.akaunting_domain)
        ).header("Authorization", self.get_authorization());
        let response = request.send().await.expect("Sent request");
        let body = response.text().await.expect("Receive body");
        return akaunting_unmarshal(body.as_str()).unwrap_or_default();
    }
    pub async fn list_customers(&self) -> ListCustomers {
        if self.can_sync() {
            return ListCustomers{..Default::default()}
        }
        let client = reqwest::Client::builder().build().expect("Built client");
    
        let request = client.request(
            reqwest::Method::GET,
            format!("{}/contacts?search=type:customer&page=1&limit=25",self.akaunting_domain)
        ).header("Authorization", self.get_authorization());
        let response = request.send().await.expect("Sent request");
        let body = response.text().await.expect("Receive body");
        return akaunting_unmarshal(body.as_str()).unwrap_or_default();
    }
    pub async fn list_users(&self) -> ListUserData {
        if self.can_sync() {
            return ListUserData{..Default::default()}
        }
        let client = reqwest::Client::builder().build().expect("Built client");
    
        let request = client.request(
            reqwest::Method::GET,
            format!("{}/users",self.akaunting_domain)
        ).header("Authorization", self.get_authorization());
        let response = request.send().await.expect("Sent request");
        let body = response.text().await.expect("Receive body");
        return akaunting_unmarshal(body.as_str()).unwrap_or_default();
    }
} 

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub data: GetItemData
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUserData {
    pub data: Vec<UserData>,
    pub links: Links,
    pub meta: Meta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub locale: String,
    #[serde(rename = "landing_page")]
    pub landing_page: String,
    pub enabled: bool,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: Value,
    #[serde(rename = "last_logged_in_at")]
    pub last_logged_in_at: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
    pub companies: Companies,
    pub roles: Roles,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Companies {
    pub data: Vec<Company>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub currency: String,
    pub domain: String,
    pub address: String,
    pub logo: String,
    pub enabled: bool,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: i64,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Roles {
    pub data: Vec<Role>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub code: String,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: Value,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContactData {
    pub data: Contact,
} 

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCustomers {
    pub data: Vec<CustomerData>,
    pub links: Links,
    pub meta: Meta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerData {
    pub id: i64,
    pub kinbrio_id: Option<uuid::Uuid>,
    #[serde(rename = "company_id")]
    pub company_id: i64,
    #[serde(rename = "user_id")]
    pub user_id: Value,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub name: String,
    pub email: Option<String>,
    #[serde(rename = "tax_number")]
    pub tax_number: Value,
    pub phone: Value,
    pub address: Option<String>,
    pub website: Value,
    #[serde(rename = "currency_code")]
    pub currency_code: Option<String>,
    pub enabled: bool,
    pub reference: Value,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: Value,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListInvoices {
    pub data: Vec<InvoiceData>,
    pub links: Links,
    pub meta: Meta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceData {
    pub id: i64,
    pub kinbrio_id: Option<uuid::Uuid>,
    #[serde(rename = "company_id")]
    pub company_id: i64,
    #[serde(rename = "type")]
    pub type_field:  Option<String>,
    #[serde(rename = "document_number")]
    pub document_number:  Option<String>,
    #[serde(rename = "order_number")]
    pub order_number:  Option<String>,
    pub status:  Option<String>,
    #[serde(rename = "issued_at")]
    pub issued_at:  Option<String>,
    #[serde(rename = "due_at")]
    pub due_at:  Option<String>,
    pub amount: f64,
    #[serde(rename = "amount_formatted")]
    pub amount_formatted:  Option<String>,
    #[serde(rename = "category_id")]
    pub category_id: Option<i64>,
    #[serde(rename = "currency_code")]
    pub currency_code:  Option<String>,
    #[serde(rename = "currency_rate")]
    pub currency_rate: Option<i64>,
    #[serde(rename = "contact_id")]
    pub contact_id: Option<i64>,
    #[serde(rename = "contact_name")]
    pub contact_name:  Option<String>,
    #[serde(rename = "contact_email")]
    pub contact_email: Value,
    #[serde(rename = "contact_tax_number")]
    pub contact_tax_number: Value,
    #[serde(rename = "contact_phone")]
    pub contact_phone: Value,
    #[serde(rename = "contact_address")]
    pub contact_address: Option<String>,
    #[serde(rename = "contact_city")]
    pub contact_city: Value,
    #[serde(rename = "contact_zip_code")]
    pub contact_zip_code: Value,
    #[serde(rename = "contact_state")]
    pub contact_state: Value,
    #[serde(rename = "contact_country")]
    pub contact_country: Value,
    pub notes: Value,
    pub attachment: bool,
    #[serde(rename = "created_from")]
    pub created_from:  Option<String>,
    #[serde(rename = "created_by")]
    pub created_by: Option<i64>,
    #[serde(rename = "created_at")]
    pub created_at:  Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at:  Option<String>,
    pub category: Category,
    pub currency: Currency,
    pub contact: Contact,
    pub histories: Histories,
    pub items: ListItems,
    #[serde(rename = "item_taxes")]
    pub item_taxes: ItemTaxes,
    pub totals: Totals,
    pub transactions: Transactions,
}
 
    
 
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: i64,
    pub kinbrio_id: Option<uuid::Uuid>,
    #[serde(rename = "company_id")]
    pub company_id: i64,
    #[serde(rename = "type")]
    pub type_field: String,
    pub name: String,
    pub number: String,
    #[serde(rename = "currency_code")]
    pub currency_code: String,
    #[serde(rename = "opening_balance")]
    pub opening_balance: i64,
    #[serde(rename = "opening_balance_formatted")]
    pub opening_balance_formatted: String,
    #[serde(rename = "current_balance")]
    pub current_balance: f64,
    #[serde(rename = "current_balance_formatted")]
    pub current_balance_formatted: String,
    #[serde(rename = "bank_name")]
    pub bank_name: String,
    #[serde(rename = "bank_phone")]
    pub bank_phone: Value,
    #[serde(rename = "bank_address")]
    pub bank_address: Value,
    pub enabled: bool,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: Value,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}  


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCompanys {
    pub data: Vec<CompanyData>,
    pub links: Links,
    pub meta: Meta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyData {
    pub id: i64,
    pub kinbrio_id: Option<uuid::Uuid>,
    pub name: String,
    pub email: String,
    pub currency: String,
    pub domain: String,
    pub address: String,
    pub logo: String,
    pub enabled: bool,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: i64,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub first: String,
    pub last: String,
    pub prev: Value,
    pub next: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(rename = "current_page")]
    pub current_page: i64,
    pub from: i64,
    #[serde(rename = "last_page")]
    pub last_page: i64,
    pub links: Vec<Link>,
    pub path: String,
    #[serde(rename = "per_page")]
    pub per_page: i64,
    pub to: i64,
    pub total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub url: Option<String>,
    pub label: String,
    pub active: bool,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertInvoiceResult {
    pub data: InvoiceData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: Value,
    #[serde(rename = "company_id")]
    pub company_id: Value,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: Value,
    pub color: Value,
    pub enabled: Value,
    #[serde(rename = "parent_id")]
    pub parent_id: Value,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: Value,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Currency {
    pub id: i64,
    #[serde(rename = "company_id")]
    pub company_id: i64,
    pub name: String,
    pub code: String,
    pub rate: Option<i64>,
    pub enabled: bool,
    pub precision:Option<i64>,
    pub symbol: String,
    #[serde(rename = "symbol_first")]
    pub symbol_first: Option<i64>,
    #[serde(rename = "decimal_mark")]
    pub decimal_mark: String,
    #[serde(rename = "thousands_separator")]
    pub thousands_separator: String,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: Value,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: Value,
    pub kinbrio_id: Option<uuid::Uuid>,
    #[serde(rename = "company_id")]
    pub company_id: Value,
    #[serde(rename = "user_id")]
    pub user_id: Value,
    #[serde(rename = "type")]
    pub type_field: Value,
    pub name: String,
    pub email: Value,
    #[serde(rename = "tax_number")]
    pub tax_number: Value,
    pub phone: Value,
    pub address: Value,
    pub website: Value,
    #[serde(rename = "currency_code")]
    pub currency_code: Value,
    pub enabled: Value,
    pub reference: Value,
    #[serde(rename = "created_from")]
    pub created_from: Value,
    #[serde(rename = "created_by")]
    pub created_by: Value,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Histories {
    pub data: Vec<HistoryData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryData {
    pub id: i64,
    #[serde(rename = "company_id")]
    pub company_id: i64,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    #[serde(rename = "document_id")]
    pub document_id: Option<i64>,
    pub status: Option<String>,
    pub notify: Option<i64>,
    pub description: Option<String>,
    #[serde(rename = "created_from")]
    pub created_from: Option<String>,
    #[serde(rename = "created_by")]
    pub created_by: Option<i64>,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItems {
    pub data: Vec<ItemData>,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetItemData {
pub id: i64,
#[serde(rename = "company_id")]
pub company_id: i64,
#[serde(rename = "type")]
pub type_field: String,
pub name: String,
pub description: String,
#[serde(rename = "sale_price")]
pub sale_price: f64,
#[serde(rename = "sale_price_formatted")]
pub sale_price_formatted: String,
#[serde(rename = "purchase_price")]
pub purchase_price: f64,
#[serde(rename = "purchase_price_formatted")]
pub purchase_price_formatted: String,
#[serde(rename = "category_id")]
pub category_id: i64,
pub picture: bool,
pub enabled: bool,
#[serde(rename = "created_from")]
pub created_from: Value,
#[serde(rename = "created_by")]
pub created_by: Value,
#[serde(rename = "created_at")]
pub created_at: String,
#[serde(rename = "updated_at")]
pub updated_at: String,
pub taxes: Taxes,
pub category: Category,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemData {
    pub id: i64,
    pub kinbrio_id: Option<uuid::Uuid>,
    #[serde(rename = "company_id")]
    pub company_id: i64,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    #[serde(rename = "document_id")]
    pub document_id: Option<i64>,
    #[serde(rename = "item_id")]
    pub item_id: Option<i64>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
    #[serde(rename = "price_formatted")]
    pub price_formatted: Option<String>,
    pub total: Option<f64>,
    #[serde(rename = "total_formatted")]
    pub total_formatted: Option<String>,
    #[serde(rename = "created_from")]
    pub created_from: Option<String>,
    #[serde(rename = "created_by")]
    pub created_by: Option<i64>,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    pub taxes: Taxes,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taxes {
    pub data: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemTaxes {
    pub data: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub data: Vec<TotalData>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalData {
    pub id: i64,
    #[serde(rename = "company_id")]
    pub company_id: i64,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    #[serde(rename = "document_id")]
    pub document_id: Option<i64>,
    pub code: Option<String>,
    pub name: Option<String>,
    pub amount: f64,
    #[serde(rename = "amount_formatted")]
    pub amount_formatted: Option<String>,
    #[serde(rename = "sort_order")]
    pub sort_order:Option<i64>,
    #[serde(rename = "created_from")]
    pub created_from: Option<String>,
    #[serde(rename = "created_by")]
    pub created_by: Option<i64>,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transactions {
    pub data: Vec<Value>,
}
