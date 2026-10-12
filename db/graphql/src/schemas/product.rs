use juniper::{GraphQLInputObject, graphql_object};
use mysql::{params, prelude::Queryable};

use crate::schemas::root::Context;
use crate::schemas::user::User;

/// Product
#[derive(Default, Debug)]
pub struct Product {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub price: f64,
}

#[graphql_object(context = Context)]
impl Product {
    fn id(&self) -> &str {
        &self.id
    }
    fn user_id(&self) -> &str {
        &self.user_id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn price(&self) -> f64 {
        self.price
    }

    fn user(&self, context: &Context) -> Option<User> {
        let mut conn = context.dbpool.get().unwrap();
        conn.exec_first(
            "SELECT * FROM user WHERE id=:id",
            params! {"id" => &self.user_id},
        )
        .ok()
        .flatten()
        .map(|(id, name, email)| User { id, name, email })
    }
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Product Input")]
pub struct ProductInput {
    pub user_id: String,
    pub name: String,
    pub price: f64,
}
