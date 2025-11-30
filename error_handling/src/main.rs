/*
The goal of this example is to show how to propagate a custom error type,
to a web handler that will evaluate the type of error that
was raised and return an appropriate HTTPResponse.

This example uses a 50/50 chance of returning 200 Ok, otherwise one of four possible
http errors will be chosen, each with an equal chance of being selected:
    1. 403 Forbidden
    2. 401 Unauthorized
    3. 500 InternalServerError
    4. 400 BadRequest

This example demonstrates how to override error rendering
for all errors. Two types are required: one must implement
the ntex::web::error::ErrorRenderer trait, while the other must
implement the ntex::web::ErrorContainer trait. All errors used in
the application must be convertible to an `error container`.
*/

use derive_more::Display;
use ntex::web::{self, types::Json, App, HttpRequest, HttpResponse, WebResponseError};
use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};

struct MyErrRenderer;

impl ntex::web::error::ErrorRenderer for MyErrRenderer {
    type Container = MyErrContainer;
}

#[derive(thiserror::Error, Debug)]
#[error("MyErrContainer({0})")]
struct MyErrContainer(Box<dyn WebResponseError<MyErrRenderer>>);

impl ntex::web::ErrorContainer for MyErrContainer {
    fn error_response(&self, req: &HttpRequest) -> HttpResponse {
        self.0.error_response(req)
    }
}

impl ntex::http::ResponseError for MyErrContainer {}

impl From<CustomError> for MyErrContainer {
    fn from(e: CustomError) -> MyErrContainer {
        MyErrContainer(Box::new(e))
    }
}

#[derive(Debug, Display)]
pub enum CustomError {
    #[display(fmt = "Custom Error 1")]
    CustomOne,
    #[display(fmt = "Custom Error 2")]
    CustomTwo,
    #[display(fmt = "Custom Error 3")]
    CustomThree,
    #[display(fmt = "Custom Error 4")]
    CustomFour,
}

impl Distribution<CustomError> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CustomError {
        match rng.gen_range(0..4) {
            0 => CustomError::CustomOne,
            1 => CustomError::CustomTwo,
            2 => CustomError::CustomThree,
            _ => CustomError::CustomFour,
        }
    }
}

/// ntex uses `ResponseError` for conversion of errors to a response
impl WebResponseError<MyErrRenderer> for CustomError {
    fn error_response(&self, _: &HttpRequest) -> HttpResponse {
        match self {
            CustomError::CustomOne => {
                println!("do some stuff related to CustomOne error");
                HttpResponse::Forbidden().finish()
            }

            CustomError::CustomTwo => {
                println!("do some stuff related to CustomTwo error");
                HttpResponse::Unauthorized().finish()
            }

            CustomError::CustomThree => {
                println!("do some stuff related to CustomThree error");
                HttpResponse::InternalServerError().finish()
            }

            _ => {
                println!("do some stuff related to CustomFour error");
                HttpResponse::BadRequest().finish()
            }
        }
    }
}

/// randomly returns either () or one of the 4 CustomError variants
async fn do_something_random() -> Result<(), CustomError> {
    let mut rng = thread_rng();

    // 20% chance that () will be returned by this function
    if rng.gen_bool(2.0 / 10.0) {
        Ok(())
    } else {
        Err(rand::random::<CustomError>())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TestPayload {
    dummy: u32,
}

/// ntex uses `ResponseError` for conversion of errors to a response
impl From<ntex::web::error::JsonPayloadError> for MyErrContainer {
    fn from(e: ntex::web::error::JsonPayloadError) -> MyErrContainer {
        MyErrContainer(Box::new(e))
    }
}

impl WebResponseError<MyErrRenderer> for ntex::web::error::JsonPayloadError {
    fn error_response(&self, _: &HttpRequest) -> HttpResponse {
        println!("do some stuff related to json error");
        HttpResponse::BadRequest().finish()
    }
}

async fn do_something(_: Json<TestPayload>) -> Result<HttpResponse, MyErrContainer> {
    do_something_random().await?;

    Ok(HttpResponse::Ok().body("Nothing interesting happened. Try again."))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "ntex=info");
    env_logger::init();

    web::server(async move || {
        App::with(MyErrRenderer)
            .service(web::resource("/something").route(web::get().to(do_something)))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
