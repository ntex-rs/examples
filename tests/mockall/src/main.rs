//! An example of testing a state-aware function service with `mockall`.
//!
//! `mockall` mocks methods rather than fields, so the state values used by the
//! service are exposed through a small trait. The test can then verify both
//! reading a value and calling a method on the state.

use std::convert::Infallible;

use ntex::service::{Pipeline, fn_service_st};

#[cfg_attr(test, mockall::automock)]
trait ServiceState {
    fn prefix(&self) -> String;
    fn record_call(&self, name: &str);
}

struct AppState {
    prefix: String,
}

impl ServiceState for AppState {
    fn prefix(&self) -> String {
        self.prefix.clone()
    }

    fn record_call(&self, name: &str) {
        println!("Greeting {name}");
    }
}

async fn greet<S>(state: &S, name: String) -> Result<String, Infallible>
where
    S: ServiceState,
{
    let prefix = state.prefix();
    state.record_call(&name);

    Ok(format!("{prefix}, {name}!"))
}

#[ntex::main]
async fn main() {
    let service = Pipeline::new(
        AppState {
            prefix: "Hello".to_owned(),
        },
        fn_service_st(greet::<AppState>),
    );

    println!("{}", service.call("World".to_owned()).await.unwrap());
}

#[cfg(test)]
mod tests {
    use mockall::{Sequence, predicate::eq};

    use super::*;

    #[ntex::test]
    async fn test_service_state_access() {
        let mut state = MockServiceState::new();
        let mut sequence = Sequence::new();

        state
            .expect_prefix()
            .times(1)
            .in_sequence(&mut sequence)
            .return_const("Welcome".to_owned());
        state
            .expect_record_call()
            .with(eq("Alice"))
            .times(1)
            .in_sequence(&mut sequence)
            .return_const(());

        let service = Pipeline::new(state, fn_service_st(greet::<MockServiceState>));

        let response = service.call("Alice".to_owned()).await.unwrap();
        assert_eq!(response, "Welcome, Alice!");

        // Dropping the service also drops the mock and verifies its expectations.
        drop(service);
    }
}
