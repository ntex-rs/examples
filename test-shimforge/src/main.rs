//! An example of testing a state-aware function service with `shimforge`.
//!
//! Shimforge can mock methods on the concrete state type, so production code
//! does not need a trait just for testing. The test replaces a state attribute
//! getter, mocks a method on a nested object, and verifies the call order.

use std::convert::Infallible;

use ntex::service::{Pipeline, fn_service_st};

struct AppState {
    prefix: String,
    calls: CallRecorder,
}

struct CallRecorder {
    source: String,
}

impl AppState {
    fn prefix(&self) -> String {
        self.prefix.clone()
    }
}

impl CallRecorder {
    fn record_call(&self, name: &str) {
        println!("{} greeted {name}", self.source);
    }
}

async fn greet(state: &AppState, name: String) -> Result<String, Infallible> {
    let prefix = state.prefix();
    state.calls.record_call(&name);

    Ok(format!("{prefix}, {name}!"))
}

#[ntex::main]
async fn main() {
    let service = Pipeline::new(
        AppState {
            prefix: "Hello".to_owned(),
            calls: CallRecorder {
                source: "example".to_owned(),
            },
        },
        fn_service_st(greet),
    );

    println!("{}", service.call("World".to_owned()).await.unwrap());
}

#[cfg(test)]
mod tests {
    use shimforge::{Sequence, Session, mock};

    use super::*;

    #[ntex::test]
    async fn test_service_state_access() {
        let mut session = Session::new();
        let order = Sequence::new();

        let prefix = mock!(session, AppState::prefix, fn(&AppState) -> String);
        prefix
            .expect()
            .with(|state| state.prefix == "Hello")
            .once()
            .in_sequence(&order)
            .returns("Welcome".to_owned());

        let record_call = mock!(session, CallRecorder::record_call, fn(&CallRecorder, &str));
        record_call
            .expect()
            .with(|recorder, name| recorder.source == "test" && *name == "Alice")
            .once()
            .in_sequence(&order)
            .returns(());

        let service = Pipeline::new(
            AppState {
                prefix: "Hello".to_owned(),
                calls: CallRecorder {
                    source: "test".to_owned(),
                },
            },
            fn_service_st(greet),
        );

        let response = service.call("Alice".to_owned()).await.unwrap();
        assert_eq!(response, "Welcome, Alice!");

        session.verify();
    }
}
