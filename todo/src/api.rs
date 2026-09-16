// use ntex_files::NamedFile;
use ntex::http;
use ntex::web::{self, HttpResponse, WebError, error};
use ntex_session::Session;
use serde::Deserialize;
use tera::Context;

use crate::session::{self, FlashMessage};
use crate::{AppState, db};

type Error = WebError<AppState>;

pub async fn index(
    state: web::types::State<AppState>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let pool = state.pool.clone();
    let tasks = web::block(move || db::get_all_tasks(&pool))
        .await
        .map_err(Error::from_err)?;

    let mut context = Context::new();
    context.insert("tasks", &tasks);

    //Session is set during operations on other endpoints
    //that can redirect to index
    if let Some(flash) = session::get_flash(&session).map_err(Error::from_err)? {
        context.insert("msg", &(flash.kind, flash.message));
        session::clear_flash(&session);
    }

    let rendered = state
        .templates
        .render("index.html.tera", &context)
        .map_err(|err| Error::from_err(error::ErrorInternalServerError(err)))?;

    Ok(HttpResponse::Ok().body(rendered))
}

#[derive(Deserialize)]
pub struct CreateForm {
    description: String,
}

pub async fn create(
    params: web::types::Form<CreateForm>,
    state: web::types::State<AppState>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let pool = state.pool.clone();

    if params.description.is_empty() {
        session::set_flash(&session, FlashMessage::error("Description cannot be empty"))
            .map_err(Error::from_err)?;
        Ok(redirect_to("/"))
    } else {
        web::block(move || db::create_task(params.into_inner().description, &pool))
            .await
            .map_err(Error::from_err)?;
        session::set_flash(&session, FlashMessage::success("Task successfully added"))
            .map_err(Error::from_err)?;
        Ok(redirect_to("/"))
    }
}

#[derive(Deserialize)]
pub struct UpdateParams {
    id: i32,
}

#[derive(Deserialize)]
pub struct UpdateForm {
    _method: String,
}

pub async fn update(
    state: web::types::State<AppState>,
    params: web::types::Path<UpdateParams>,
    form: web::types::Form<UpdateForm>,
    session: Session,
) -> Result<HttpResponse, Error> {
    match form._method.as_ref() {
        "put" => toggle(state, params).await,
        "delete" => delete(state, params, session).await,
        unsupported_method => {
            let msg = format!("Unsupported HTTP method: {}", unsupported_method);
            Err(Error::from_err(error::ErrorBadRequest(msg)))
        }
    }
}

async fn toggle(
    state: web::types::State<AppState>,
    params: web::types::Path<UpdateParams>,
) -> Result<HttpResponse, Error> {
    let pool = state.pool.clone();
    web::block(move || db::toggle_task(params.id, &pool))
        .await
        .map_err(Error::from_err)?;
    Ok(redirect_to("/"))
}

async fn delete(
    state: web::types::State<AppState>,
    params: web::types::Path<UpdateParams>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let pool = state.pool.clone();
    web::block(move || db::delete_task(params.id, &pool))
        .await
        .map_err(Error::from_err)?;
    session::set_flash(&session, FlashMessage::success("Task was deleted."))
        .map_err(Error::from_err)?;
    Ok(redirect_to("/"))
}

fn redirect_to(location: &str) -> HttpResponse {
    HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .build()
}

// pub fn bad_request<B>(res: dev::WebResponse<B>) -> Result<ErrorHandlerResponse<B>> {
//     let new_resp = NamedFile::open("static/errors/400.html")?
//         .set_status_code(res.status())
//         .into_response(res.request())?;
//     Ok(ErrorHandlerResponse::Response(
//         res.into_response(new_resp.into_body()),
//     ))
// }

// pub fn not_found<B>(res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
//     let new_resp = NamedFile::open("static/errors/404.html")?
//         .set_status_code(res.status())
//         .into_response(res.request())?;
//     Ok(ErrorHandlerResponse::Response(
//         res.into_response(new_resp.into_body()),
//     ))
// }

// pub fn internal_server_error<B>(
//     res: dev::ServiceResponse<B>,
// ) -> Result<ErrorHandlerResponse<B>> {
//     let new_resp = NamedFile::open("static/errors/500.html")?
//         .set_status_code(res.status())
//         .into_response(res.request())?;
//     Ok(ErrorHandlerResponse::Response(
//         res.into_response(new_resp.into_body()),
//     ))
// }
