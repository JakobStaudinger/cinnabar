use api::Server;
use config::AppConfig;
use diesel::prelude::*;
use diesel::{prelude::Queryable, Connection, Selectable, SqliteConnection};

mod api;
mod config;
mod orchestrator;
mod parser;
mod runner;
mod schema;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = AppConfig::from_environment()?;
    let connection = &mut SqliteConnection::establish(&config.database.url)
        .map_err(|e| format!("Failed to connect to database {e}"))?;

    use self::schema::posts::dsl::*;

    let results = posts
        .filter(published.eq(true))
        .limit(5)
        .select(Post::as_select())
        .load(connection)
        .map_err(|e| format!("Failed to query posts: {e}"))?;

    for post in results {
        println!("{}", post.title);
        println!("------------");
        println!("{}", post.body);
    }

    // let server = Server::new(config);

    // server
    //     .start()
    //     .await
    //     .map_err(|e| format!("Failed to start HTTP server {e}"))?;

    Ok(())
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::posts)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}
