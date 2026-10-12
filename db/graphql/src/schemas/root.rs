use juniper::{
    EmptySubscription, FieldError, FieldResult, RootNode, graphql_object, graphql_value,
};
use mysql::{Error as DBError, params, prelude::Queryable};

use crate::db::Pool;

use super::product::{Product, ProductInput};
use super::user::{User, UserInput};

pub struct Context {
    pub dbpool: Pool,
}

impl juniper::Context for Context {}

fn db_error(msg: &str, err: DBError) -> FieldError {
    let err = match err {
        DBError::MySqlError(err) => err.message,
        _ => "internal error".to_owned(),
    };
    FieldError::new(msg, graphql_value!({ "internal_error": err }))
}

pub struct QueryRoot;

#[graphql_object(context = Context)]
impl QueryRoot {
    #[graphql(description = "List of all users")]
    fn users(context: &Context) -> FieldResult<Vec<User>> {
        let mut conn = context.dbpool.get()?;
        conn.query_map("select * from user", |(id, name, email)| User {
            id,
            name,
            email,
        })
        .map_err(|e| db_error("Failed to load users", e))
    }

    #[graphql(description = "Get Single user reference by user ID")]
    fn user(context: &Context, id: String) -> FieldResult<User> {
        let mut conn = context.dbpool.get()?;
        conn.exec_first("SELECT * FROM user WHERE id=:id", params! {"id" => id})
            .map_err(|e| db_error("Failed to load user", e))?
            .map(|(id, name, email)| User { id, name, email })
            .ok_or_else(|| {
                FieldError::new(
                    "User Not Found",
                    graphql_value!({ "not_found": "user not found" }),
                )
            })
    }

    #[graphql(description = "List of all products")]
    fn products(context: &Context) -> FieldResult<Vec<Product>> {
        let mut conn = context.dbpool.get()?;
        conn.query_map("select * from product", |(id, user_id, name, price)| {
            Product {
                id,
                user_id,
                name,
                price,
            }
        })
        .map_err(|e| db_error("Failed to load products", e))
    }

    #[graphql(description = "Get Single product reference by product ID")]
    fn product(context: &Context, id: String) -> FieldResult<Product> {
        let mut conn = context.dbpool.get()?;
        conn.exec_first("SELECT * FROM product WHERE id=:id", params! {"id" => id})
            .map_err(|e| db_error("Failed to load product", e))?
            .map(|(id, user_id, name, price)| Product {
                id,
                user_id,
                name,
                price,
            })
            .ok_or_else(|| {
                FieldError::new(
                    "Product Not Found",
                    graphql_value!({ "not_found": "product not found" }),
                )
            })
    }
}

pub struct MutationRoot;

#[graphql_object(context = Context)]
impl MutationRoot {
    fn create_user(context: &Context, user: UserInput) -> FieldResult<User> {
        let mut conn = context.dbpool.get()?;
        let new_id = uuid::Uuid::new_v4().simple().to_string();

        conn.exec_drop(
            "INSERT INTO user(id, name, email) VALUES(:id, :name, :email)",
            params! {
                "id" => &new_id,
                "name" => &user.name,
                "email" => &user.email,
            },
        )
        .map_err(|e| db_error("Failed to create new user", e))?;

        Ok(User {
            id: new_id,
            name: user.name,
            email: user.email,
        })
    }

    fn create_product(context: &Context, product: ProductInput) -> FieldResult<Product> {
        let mut conn = context.dbpool.get()?;
        let new_id = uuid::Uuid::new_v4().simple().to_string();

        conn.exec_drop(
            "INSERT INTO product(id, user_id, name, price) VALUES(:id, :user_id, :name, :price)",
            params! {
                "id" => &new_id,
                "user_id" => &product.user_id,
                "name" => &product.name,
                "price" => product.price,
            },
        )
        .map_err(|e| db_error("Failed to create new product", e))?;

        Ok(Product {
            id: new_id,
            user_id: product.user_id,
            name: product.name,
            price: product.price,
        })
    }
}

pub type Schema = RootNode<QueryRoot, MutationRoot, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(QueryRoot, MutationRoot, EmptySubscription::new())
}
