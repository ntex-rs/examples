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
the ntex::web::State trait, while the other must
implement the ntex::web::WebResponseError trait. All errors used in
the application must be convertible to an `error container`.
*/
use ntex::web::{self, App, HttpResponse, WebResponseError, types::Json};
use rand::{Rng, distributions::Distribution, distributions::Standard, thread_rng};

struct MyErr;

#[derive(Copy, Clone, Debug)]
struct MyState;

impl web::State for MyState {
    type Error = MyErr;
}

#[derive(thiserror::Error, Debug)]
#[error("MyErrContainer({0})")]
struct MyErrContainer(Box<dyn WebResponseError<MyState, MyErr>>);

impl web::WebResponseError<MyState, MyErr> for MyErrContainer {
    fn error_response(&self, st: &MyState) -> HttpResponse {
        self.0.error_response(st)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CustomError {
    #[error("Custom Error 1")]
    CustomOne,
    #[error("Custom Error 2")]
    CustomTwo,
    #[error("Custom Error 3")]
    CustomThree,
    #[error("Custom Error 4")]
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

/// Converts this error into an HTTP response using application state.
impl WebResponseError<MyState, MyErr> for CustomError {
    fn error_response(&self, _: &MyState) -> HttpResponse {
        match self {
            CustomError::CustomOne => {
                println!("do some stuff related to CustomOne error");
                HttpResponse::Forbidden().build()
            }

            CustomError::CustomTwo => {
                println!("do some stuff related to CustomTwo error");
                HttpResponse::Unauthorized().build()
            }

            CustomError::CustomThree => {
                println!("do some stuff related to CustomThree error");
                HttpResponse::InternalServerError().build()
            }

            _ => {
                println!("do some stuff related to CustomFour error");
                HttpResponse::BadRequest().build()
            }
        }
    }
}

impl From<CustomError> for MyErrContainer {
    fn from(err: CustomError) -> Self {
        MyErrContainer(Box::new(err))
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

impl From<web::error::JsonPayloadError> for MyErrContainer {
    fn from(err: web::error::JsonPayloadError) -> Self {
        MyErrContainer(Box::new(err))
    }
}

/// Implement WebResponseError for JsonPayloadError
impl WebResponseError<MyState, MyErr> for web::error::JsonPayloadError {
    fn error_response(&self, _: &MyState) -> HttpResponse {
        println!("do some stuff related to json error");
        HttpResponse::BadRequest().build()
    }
}

async fn do_something(_: Json<TestPayload>) -> Result<HttpResponse, MyErrContainer> {
    do_something_random().await?;

    Ok(HttpResponse::Ok().body("Nothing interesting happened. Try again."))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    web::server(async move |_| {
        App::new()
            .service(web::resource("/something").route(web::get().to(do_something)))
            .build_with(MyState)
    })
    .bind("127.0.0.1:8088", ntex::SharedCfg::new("S"))?
    .run()
    .await
}
