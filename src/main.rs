mod akaunting;
mod board;
mod common;
mod entity;
mod file;
mod home;
mod matrix;
mod milestone;
mod note;
mod organization;
mod project;
mod service_item;
mod task;
mod user;
use dotenv::dotenv;
use file::get_file_fs;
use sqlx::{PgPool, Pool};
use tide::{Body, Request, Response, StatusCode, http::mime};
use tokio::io;
use user::{read_jwt_cookie, UserJwtState};
#[derive(Clone, Debug)]
pub struct State {
    db_pool: PgPool,
}

pub async fn serve_dir(req: Request<State>) -> tide::Result {
    let f_local_path = "./assets".to_string() + req.url().path().replace("/fs", "").as_str();
    match Body::from_file(f_local_path.clone()).await {
        Ok(body) => {
            let mut builder = Response::builder(StatusCode::Ok).body(body);
            if f_local_path.clone().contains(".css") || f_local_path.contains(".js") {
                builder = builder.header("Cache-Control", "max-age=31536000, immutable");
            }
            Ok(builder.build())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Response::new(StatusCode::NotFound)),
        Err(e) => Err(e.into()),
    }
}

pub async fn serve_s3(req: Request<State>) -> tide::Result {
    let claims: UserJwtState = match read_jwt_cookie(req.cookie("token")) {
        Some(c) => c,
        None => {
            return Ok(Response::new(StatusCode::Unauthorized));
        }
    };
    let format = match req.param("format") {
        Ok(i) => i,
        Err(e) => return Ok(Response::new(StatusCode::BadRequest)),
    };
    let association_type = match req.param("association_type") {
        Ok(i) => i,
        Err(e) => return Ok(Response::new(StatusCode::BadRequest)),
    };
    let association_id = match req.param("association_id") {
        Ok(i) => i,
        Err(e) => return Ok(Response::new(StatusCode::BadRequest)),
    };
    let name = match req.param("name") {
        Ok(i) => i,
        Err(e) => return Ok(Response::new(StatusCode::BadRequest)),
    };
    let organization_id = match req.param("organization_id") {
        Ok(i) => i,
        Err(e) => return Ok(Response::new(StatusCode::BadRequest)),
    };
    let org_claim = claims.organization_key.clone();
    println!("{organization_id} {org_claim}");
    if organization_id != claims.organization_key {
        return Ok(Response::new(StatusCode::Unauthorized));
    }
    let resp = get_file_fs(association_type.to_owned(), association_id.to_owned(), name.to_owned()).await.expect("S3 worked");
    let m_type = match format {
        "jpg" => mime::JPEG,
        "jpeg" => mime::JPEG,
        "png" => mime::PNG,
        "svg" => mime::SVG,
        "ico" => mime::ICO,
        _ => mime::BYTE_STREAM,
    };
    let body = Body::from_bytes(resp.bytes().await.expect("Body").to_vec());
    Ok(tide::Response::builder(tide::StatusCode::Ok)
        .content_type(m_type)
        .body(body)
        .build())
}

// me fucking aroud making a macro that looks like log!("msg") and then does println!... it's not so spooky afterall
macro_rules! log {
    // `()` indicates that the macro takes no argument.
    ($msg:literal) => {
        // The macro will expand into the contents of this block.
        println!($msg)
    };
}

#[tokio::main]
async fn main() -> tide::Result<()> {
    dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .expect("Missing `DATABASE_URL` env variable, needed for running the server");

    let db_pool: PgPool = Pool::connect(&db_url).await.unwrap();

    let state = State { db_pool };
    let mut app = tide::with_state(state);
    app.with(tide_compress::CompressMiddleware::new());

    app.at("/").get(home::home);

    app.at("/dashboard").get(home::dashboard);
    app.at("/documentation").get(home::documentation);
    app.at("/account").get(home::account);

    app.at("/login").get(user::login);
    app.at("/logout").get(user::logout);
    app.at("/login").post(user::login_post);
    app.at("/login_matrix").get(user::login_matrix);
    app.at("/login_by_username").get(user::login_by_username);

    app.at("/register").get(user::register);
    app.at("/register").post(user::register_post);
    app.at("/register_matrix").get(user::register_post);

    app.at("/users/:user_id").post(user::update);
    app.at("/users/:user_id").get(user::get);
    app.at("/users/:user_id").delete(user::delete);

    app.at("/project").post(project::insert);
    app.at("/project/add").get(project::add);
    app.at("/project/:project_id").get(project::get);
    app.at("/project/:project_id").delete(project::delete);

    app.at("/task").post(task::insert);
    app.at("/task/add/:project_id").get(task::add);
    app.at("/task/:task_id").get(task::get);
    app.at("/task/:task_id").delete(task::delete);

    app.at("/entity").post(entity::insert);
    app.at("/entity/add").get(entity::add);
    app.at("/entity/:entity_id").get(entity::get);
    app.at("/entity/:entity_id").delete(entity::delete);
    app.at("/entity/invoices/:entity_id/:external_id")
        .get(entity::get_invoices);

    app.at("/contact").post(entity::insert_contact_route);
    app.at("/contact/add/:entity_id").get(entity::add_contact);
    app.at("/contact/:contact_id")
        .get(entity::get_contact_route);
    app.at("/contact/:contact_id")
        .delete(entity::delete_contact_route);

    app.at("/board").post(board::insert);
    app.at("/board/add").get(board::add);
    app.at("/board/:board_id").get(board::get);
    app.at("/board/:board_id").delete(board::delete);

    app.at("/service_item").post(service_item::insert);
    app.at("/service_item/add").get(service_item::add);
    app.at("/service_item/:service_item_id")
        .get(service_item::get);
    app.at("/service_item/:service_item_id")
        .delete(service_item::delete);

    app.at("/organization").post(organization::insert);
    app.at("/organization/:organization_id")
        .get(organization::get);
    app.at("/organization/:organization_id")
        .delete(organization::delete);

    app.at("/milestone").post(milestone::insert);
    app.at("/milestone/add/:project_id").get(milestone::add);
    app.at("/milestone/:milestone_id").get(milestone::get);
    app.at("/milestone/:milestone_id").delete(milestone::delete);

    app.at("/akaunting").post(akaunting::save_akaunting_options);
    app.at("/akaunting")
        .get(akaunting::get_akaunting_options_page);
    app.at("/akaunting/import_item")
        .post(akaunting::import_item);
    app.at("/akaunting/import_customer")
        .post(akaunting::import_customer);

    app.at("/file").post(file::insert);
    app.at("/file/add/:association_type/:association_id")
        .get(file::add);
    app.at("/file/:file_id").get(file::get);
    app.at("/file/:file_id").delete(file::delete);

    app.at("/note").post(note::insert);
    app.at("/note/add/:association_type/:association_id")
        .get(note::add);
    app.at("/note/:note_id").get(note::get);
    app.at("/note/:note_id").delete(note::delete);

    app.at("/fs/*").get(serve_dir);
    app.at("/files/:format/:organization_id/:association_type/:association_id/:name")
        .get(serve_s3);
    // app.at("/fs").serve_dir("./assets")?;

    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
