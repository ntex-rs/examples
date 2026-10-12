use ntex::http;
use ntex::web::{self, HttpResponse, WebError, error};
use ntex_session::Session;
use serde::Deserialize;
use tera::Context;

use crate::session::{self, FlashMessage};
use crate::{AppState, db};

type Error = WebError<AppState>;

pub async fn index(state: &AppState, _: (), session: Session) -> Result<HttpResponse, Error> {
    let pool = state.pool.clone();
    let tasks = web::block(move || db::get_all_tasks(&pool))
        .await
        .map_err(Error::from_err)?;

    let mut context = Context::new();
    context.insert("tasks", &tasks);

    // Show a message set by an operation that redirected back to this page.
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
    state: &AppState,
    _: (),
    params: web::types::Form<CreateForm>,
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
    state: &AppState,
    _: (),
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
    state: &AppState,
    params: web::types::Path<UpdateParams>,
) -> Result<HttpResponse, Error> {
    let pool = state.pool.clone();
    web::block(move || db::toggle_task(params.id, &pool))
        .await
        .map_err(Error::from_err)?;
    Ok(redirect_to("/"))
}

async fn delete(
    state: &AppState,
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
