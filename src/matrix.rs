use std::fmt;

use matrix_sdk::{
    self,
    config::SyncSettings,
    ruma::{
        api::client::session::{
            get_login_types::v3::{IdentityProvider, LoginType},
            login,
        },
        device_id,
        events::room::message::RoomMessageEventContent,
        OwnedUserId, RoomId, UserId,
    },
    Client, Session,
};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, postgres::PgQueryResult, Postgres};
use url::Url;

use crate::{
    board::Board, entity::{Entity, Contact}, file::File, milestone::Milestone, project::Project, task::Task, note::Note,
};

// authentication, messaging and server management stuff for matrix.

const INITIAL_DEVICE_DISPLAY_NAME: &str = "Kinbrio-client";

pub struct Choice {
    pub url: String,
    pub display: String,
    pub logo: String,
}
pub async fn get_login_urls(
    homeserver_url: String,
    redirect_url: String,
) -> Result<Vec<Choice>, matrix_sdk::Error> {
    let homeserver_url = Url::parse(&homeserver_url).expect("Url Correct");
    let client = Client::new(homeserver_url)
        .await
        .expect("Matrix Server Connecting");
    let mut choices = Vec::new();
    let login_types = client
        .get_login_types()
        .await
        .expect("Login types found")
        .flows;
    for login_type in login_types {
        match login_type {
            LoginType::Sso(sso) => {
                if sso.identity_providers.is_empty() {
                    choices.push(LoginChoice::Sso)
                } else {
                    choices.extend(sso.identity_providers.into_iter().map(LoginChoice::SsoIdp))
                }
            }
            LoginType::Password(t) => {
                choices.push(LoginChoice::Password)
            }
            LoginType::Token(_) | _ => {}
        }
    }
    let mut urls = vec![];
    for c in &choices {
        let u = c
            .login(&client, redirect_url.clone())
            .await
            .expect("login URL fails");
        urls.push(Choice {
            url: u.clone(),
            display: c.to_string(),
            logo: c.get_icon(),
        });
    }
    return Ok(urls);
}

#[derive(Debug)]
pub enum LoginChoice {
    Password,
    /// Login with SSO.
    Sso,
    /// Login with a specific SSO identity provider.
    SsoIdp(IdentityProvider),
}

impl LoginChoice {
    /// Login with this login choice.
    async fn login(&self, client: &Client, redirect: String) -> anyhow::Result<String> {
        match self {
            LoginChoice::Password => login_with_password_url(client),
            LoginChoice::Sso => login_with_sso_url(client, redirect, None).await,
            LoginChoice::SsoIdp(idp) => login_with_sso_url(client, redirect, Some(idp)).await,
        }
    }

    fn get_icon_mxc(&self) -> String {
        match self {
            LoginChoice::Password =>  "/fs/images/sso/user_password.svg".to_string(),
            LoginChoice::Sso => "hmm".to_string(),
            LoginChoice::SsoIdp(idp) => idp.icon.as_ref().expect("get icon URL").to_string(),
        }
    }

    fn get_icon(&self) -> String {
        match self {
            LoginChoice::Password =>  "/fs/images/sso/user_password.svg".to_string(),
            LoginChoice::Sso => "hmm".to_string(),
            LoginChoice::SsoIdp(idp) => {
                let mxc_uri = idp.icon.as_ref().expect("get icon URL").to_string();
                mxc_uri.replace("mxc://", "https://matrix.org/_matrix/media/r0/download/")
            },
        }
    }
}

impl fmt::Display for LoginChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginChoice::Password => write!(f, "Username and password"),
            LoginChoice::Sso => write!(f, "SSO"),
            LoginChoice::SsoIdp(idp) => write!(f, "{}", idp.name),
        }
    }
}

/// Login with a username and password.
fn login_with_password_url(client: &Client) -> anyhow::Result<String> {
    return Ok("/login_by_username".to_string())
}

/// Login with SSO.
pub(crate) async fn restore_from_session(
    homeserver_url: String,
    session: Session,
) -> anyhow::Result<()> {
    let homeserver_url = Url::parse(&homeserver_url).expect("URL parsing");
    let client = Client::new(homeserver_url)
        .await
        .expect("Matrix server connection");
    client.restore_login(session).await.expect("Restore login");
    Ok(())
}

pub(crate) async fn login_with_password(
    homeserver_url: String,
    uid: String,
    password: String,
) -> anyhow::Result<login::v3::Response> {
    let homeserver_url = Url::parse(&homeserver_url).expect("URL parsing");
    let client = Client::new(homeserver_url)
        .await
        .expect("Matrix server connection");
    let login_builder = client.login_username(uid.as_str(), password.as_str()).send().await.expect("logged in");
           
    Ok(login_builder)
}


pub(crate) async fn account(
    homeserver_url: String,
    token: String,
    user_id:OwnedUserId,
) -> (std::string::String, std::string::String, std::string::String) {
    let homeserver_url = Url::parse(&homeserver_url).expect("URL parsing");
    let client = Client::new(homeserver_url)
        .await
        .expect("Matrix server connection"); 
    client.restore_login(Session {
        access_token: token,
        refresh_token: None,
        user_id: OwnedUserId::from(user_id),
        device_id: device_id!("kinbrio").to_owned(),
    }).await.expect("Restore");
    let avatar = client.account().get_avatar_url().await.expect("Get avatar URL").expect("Unroll").to_string();
    let threepids = client.account().get_3pids().await.expect("Get 3pid").threepids;
    let mut addy = String::default();
    for pid in threepids {
        addy = match pid.medium {
            matrix_sdk::ruma::thirdparty::Medium::Email => pid.address,
            matrix_sdk::ruma::thirdparty::Medium::Msisdn => pid.address,
            _ => String::default(),
        };
        if addy.len() > 0 {
            break;
        }
    };
    let display_name = client.account().get_profile().await.expect("get prorfile").displayname.unwrap_or_default();
    (addy, avatar, display_name)
}

pub(crate) async fn login_with_token(
    homeserver_url: String,
    token: String,
) -> anyhow::Result<login::v3::Response> {
    let homeserver_url = Url::parse(&homeserver_url).expect("URL parsing");
    let client = Client::new(homeserver_url)
        .await
        .expect("Matrix server connection");
    
    let login_builder = client.login_token(&token).send().await?;
    Ok(login_builder)
}

async fn login_with_sso_url(
    client: &Client,
    redirect: String,
    idp: Option<&IdentityProvider>,
) -> anyhow::Result<String> {
    
    let login_builder = client
        .get_sso_login_url(&redirect, Some(&idp.expect("get provider").id))
        .await
        .expect("Get provider");

    Ok(login_builder)
}

pub async fn delete_room(
    conn: &mut PoolConnection<Postgres>,
    key: uuid::Uuid,
) -> Result<PgQueryResult, sqlx::Error> {
    return sqlx::query!("DELETE FROM rooms where key=$1", key)
        .execute(conn)
        .await;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Room {
    pub key: uuid::Uuid,
    pub owner_key: uuid::Uuid,
    pub organization_key: uuid::Uuid,

    pub name: String,
    pub description: String,
    pub matrix_room_url: String,
    pub matrix_room_id: String,
    pub message_types: MessageDataType,
    pub alert_level: i16,

    pub created: i64,
    pub updated: i64,
}

pub async fn get_rooms(
    conn: &mut PoolConnection<Postgres>,
    organization_key: uuid::Uuid,
) -> Vec<Room> {
    let room_records = sqlx::query!(
        "select 
        key, 
        owner_key, 
        organization_key, 
        name, 
        description, 
        matrix_room_url, 
        matrix_room_id, 
        message_types,
        alert_level, 
        created, 
        updated from rooms 
        where organization_key = $1",
        organization_key
    )
    .fetch_all(conn)
    .await
    .expect("Select room by key");
    let mut rooms = vec![];
    for room in room_records {
        let m_type: MessageDataType = room.message_types.expect("message_types exists").into();
        rooms.push(Room {
            key: room.key.expect("key exists"),
            owner_key: room.owner_key.expect("owner_key exists"),
            organization_key: room.organization_key.expect("organization_key exists"),
            name: room.name.expect("name exists"),
            description: room.description.expect("description exists"),
            matrix_room_url: room.matrix_room_url.expect("matrix_room_url exists"),
            matrix_room_id: room.matrix_room_id.expect("matrix_room_id exists"),
            message_types: m_type,
            alert_level: room.alert_level.expect("alert_level exists"),
            created: room.created.expect("created exists"),
            updated: room.updated.expect("updated exists"),
        });
    }
    rooms
}

pub async fn get_room(conn: &mut PoolConnection<Postgres>, key: uuid::Uuid) -> Room {
    let room = sqlx::query!(
        "select 
        key, 
        owner_key, 
        organization_key, 
        name, 
        description, 
        matrix_room_url, 
        matrix_room_id, 
        message_types,
        alert_level, 
        created, 
        updated from rooms 
        where key = $1",
        key
    )
    .fetch_one(conn)
    .await
    .expect("Select room by key");
    Room {
        key: room.key.expect("key exists"),
        owner_key: room.owner_key.expect("owner_key exists"),
        organization_key: room.organization_key.expect("organization_key exists"),
        name: room.name.expect("name exists"),
        description: room.description.expect("description exists"),
        matrix_room_url: room.matrix_room_url.expect("matrix_room_url exists"),
        matrix_room_id: room.matrix_room_id.expect("matrix_room_id exists"),
        message_types: MessageDataType::All,
        alert_level: room.alert_level.expect("alert_level exists"),
        created: room.created.expect("created exists"),
        updated: room.updated.expect("updated exists"),
    }
}

pub async fn insert_room(conn: &mut PoolConnection<Postgres>, new_room: &Room) {
    let t = new_room.message_types as i16;
    sqlx::query!(
        "INSERT INTO rooms (
        key, 
        owner_key, 
        organization_key, 
        name, 
        description, 
        matrix_room_url, 
        matrix_room_id, 
        message_types,
        alert_level, 
        created, 
        updated) values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        new_room.key,
        new_room.owner_key,
        new_room.organization_key,
        &new_room.name,
        &new_room.description,
        &new_room.matrix_room_url,
        &new_room.matrix_room_id,
        &t,
        &new_room.alert_level,
        new_room.created,
        new_room.updated,
    )
    .execute(conn)
    .await
    .expect("Insert Success");
}

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone, Copy, sqlx::Type)]
pub enum MessageDataType {
    All = 0,
    Board = 1,
    Entity = 2,
    File = 3,
    Milestone = 4,
    Oragnization = 5,
    Project = 6,
    Task = 7,
    User = 8,
    Report = 9,
    Room = 10,
}
impl Into<MessageDataType> for i16 {
    fn into(self) -> MessageDataType {
        match self {
            0 => MessageDataType::All,
            1 => MessageDataType::Board,
            2 => MessageDataType::Entity,
            3 => MessageDataType::File,
            4 => MessageDataType::Milestone,
            5 => MessageDataType::Oragnization,
            6 => MessageDataType::Project,
            7 => MessageDataType::Task,
            8 => MessageDataType::User,
            9 => MessageDataType::Report,
            10 => MessageDataType::Room,
            _ => MessageDataType::All
        }
    }
}
impl From<MessageDataType> for i16 {
    fn from(t: MessageDataType) -> Self {
        match t {
            MessageDataType::All => 0,
            MessageDataType::Board => 1,
            MessageDataType::Entity => 2,
            MessageDataType::File => 3,
            MessageDataType::Milestone => 4,
            MessageDataType::Oragnization => 5,
            MessageDataType::Project => 6,
            MessageDataType::Task => 7,
            MessageDataType::User => 8,
            MessageDataType::Report => 9,
            MessageDataType::Room => 10
        }
    }
}

enum MessageActionType {
    All,
    Create,
    Update,
    Delete,
    Complete,
}

async fn message(
    conn: &mut PoolConnection<Postgres>,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    homeserver_url: String,
    token: String,
    data_type: MessageDataType,
    _action_type: MessageActionType,
    msg: String,
) -> Result<(), anyhow::Error> {
    let rooms = get_rooms(conn, organization_id).await;
    for room in rooms {
        if room.message_types == data_type {
            send_room_message(
                matrix_user_id.clone(),
                organization_id,
                homeserver_url.clone(),
                token.clone(),
                room.matrix_room_id.clone(),
                msg.clone(),
            ).await.expect("message sent");
        }
    }
    Ok(())
}
async fn send_room_message(
    matrix_user_id: String,
    _organization_id: uuid::Uuid,
    homeserver_url: String,
    token: String,
    room_id: String,
    msg: String,
) -> Result<(), anyhow::Error> {
    let user_id_str = matrix_user_id.to_string();
    let user_id = <&UserId>::try_from(user_id_str.as_str()).expect("parse user id");
    let homeserver_url =
        Url::parse(&homeserver_url).unwrap_or(Url::parse("https://matrix-client.matrix.org")?);
    let client = Client::new(homeserver_url).await?;
    
    let session = Session {
        access_token: token,
        refresh_token: None,
        user_id: OwnedUserId::from(user_id),
        device_id: device_id!("kinbrio").to_owned(),
    };
    client.restore_login(session).await.expect("Send login");
    client
        .sync_once(SyncSettings::default())
        .await
        .expect("sync");
    let room_id = <&RoomId>::try_from(room_id.as_str()).expect("parse room id");
    let room = client.get_joined_room(room_id).expect("Retrieve room");

    let content = RoomMessageEventContent::text_plain(msg);
    room.send(content, None).await.expect("Send room");
    Ok(())
}

pub(crate) async fn post_room_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    room: &Room,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New Room 🚀 \n {} \n `{}`\n  https://kinbrio.com/room/{}",
        room.name, room.description, room.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Room,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}

pub(crate) async fn post_file_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    
    file: &File,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New File 🚀 \n {}\n `{}`\n  https://kinbrio.com/file/{}",
        file.name, file.description, file.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::File,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}

pub(crate) async fn post_milestone_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    
    milestone: &Milestone,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New Milestone 🚀 \n {}\n `{}`\n  https://kinbrio.com/milestone/{}",
        milestone.name, milestone.description, milestone.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Milestone,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}

pub(crate) async fn post_project_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    
    project: &Project,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New Project 🚀 \n {}\n `{}`\n  https://kinbrio.com/project/{}",
        project.name, project.description, project.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Project,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}

pub(crate) async fn post_entity_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    
    entity: &Entity,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New Entity Added 🚀 \n  {}\n `{}`\n  https://kinbrio.com/entity/{}",
        entity.name, entity.description, entity.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Entity,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}

pub(crate) async fn post_note_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    
    note: &Note,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New Note Added 🚀 \n  {} \n  https://kinbrio.com/contact/{}",
        note.title, note.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Entity,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}
pub(crate) async fn post_contact_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    
    contact: &Contact,
) -> Result<(), anyhow::Error> {
    let msg: String = format!(
        "New Contact Added 🚀 \n  {} {}\n  https://kinbrio.com/contact/{}",
        contact.first_name, contact.last_name, contact.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Entity,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}

pub(crate) async fn post_board_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    board: &Board,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New Board 🚀 \n  {}\n `{}`\n  https://kinbrio.com/board/{}",
        board.name, board.description, board.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Board,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}

pub(crate) async fn post_task_create(
    conn: &mut PoolConnection<Postgres>,
    homeserver_url: String,
    matrix_user_id: String,
    organization_id: uuid::Uuid,
    token: String,
    
    task: &Task,
) -> Result<(), anyhow::Error> {
    let msg = format!(
        "New Task 🚀 \n {} day(s) Task: {}\n `{}`\n  https://kinbrio.com/task/{}",
        task.estimated_quarter_days as f64 * 0.25,
        task.name,
        task.description,
        task.key
    );
    message(
        conn,
        matrix_user_id,
        organization_id,
        homeserver_url,
        token,
        MessageDataType::Task,
        MessageActionType::Create,
        msg,
    )
    .await?;
    Ok(())
}
