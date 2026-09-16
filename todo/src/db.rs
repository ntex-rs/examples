use std::{io, ops::Deref};

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};

use crate::model::{NewTask, Task};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

pub fn init_pool(database_url: &str) -> Result<PgPool, PoolError> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder().build(manager)
}

fn get_conn(pool: &PgPool) -> io::Result<PgPooledConnection> {
    pool.get().map_err(io::Error::other)
}

pub fn get_all_tasks(pool: &PgPool) -> io::Result<Vec<Task>> {
    Task::all(get_conn(pool)?.deref()).map_err(io::Error::other)
}

pub fn create_task(todo: String, pool: &PgPool) -> io::Result<()> {
    let new_task = NewTask { description: todo };
    Task::insert(new_task, get_conn(pool)?.deref())
        .map(|_| ())
        .map_err(io::Error::other)
}

pub fn toggle_task(id: i32, pool: &PgPool) -> io::Result<()> {
    Task::toggle_with_id(id, get_conn(pool)?.deref())
        .map(|_| ())
        .map_err(io::Error::other)
}

pub fn delete_task(id: i32, pool: &PgPool) -> io::Result<()> {
    Task::delete_with_id(id, get_conn(pool)?.deref())
        .map(|_| ())
        .map_err(io::Error::other)
}
