use std::io::{stdin, Read};

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

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let args = args.iter().map(|arg| arg.as_str()).collect::<Vec<_>>();

    match args[..] {
        ["write"] => {
            let mut title = String::new();
            let mut body = String::new();

            println!("What would you like to name your post?");
            stdin()
                .read_line(&mut title)
                .map_err(|e| format!("Failed to read from stdin: {e}"))?;

            let title = title.trim_end();

            println!("Start writing! (Press Ctrl+D when done)");
            stdin()
                .read_to_string(&mut body)
                .map_err(|e| format!("Failed to read from stdin: {e}"))?;

            let post = create_post(connection, &title, &body)?;
            println!("Saved draft post {title} with id {}", post.id);
        }
        ["write", ..] => println!("Syntax: cargo run write"),
        ["publish", post_id] => {
            let post_id = post_id
                .parse::<i32>()
                .map_err(|_| format!("Publish accepts an integer id. Got {post_id}"))?;

            println!("Publishing post {post_id}");

            use self::schema::posts::dsl::{posts, published};

            let post = diesel::update(posts.find(post_id))
                .set(published.eq(true))
                .returning(Post::as_returning())
                .get_result(connection)
                .map_err(|e| format!("Failed to publish post {post_id}: {e}"))?;

            println!("Published post {}", post.title);
        }
        ["publish", ..] => println!("Syntax: cargo run publish <id>"),
        ["unpublish", post_id] => {
            let post_id = post_id
                .parse::<i32>()
                .map_err(|_| format!("Unpublish accepts an integer id. Got {post_id}"))?;

            println!("Unpublishing post {post_id}");

            use self::schema::posts::dsl::{posts, published};

            let post = diesel::update(posts.find(post_id))
                .set(published.eq(false))
                .returning(Post::as_returning())
                .get_result(connection)
                .map_err(|e| format!("Faield to unpublish post {post_id}: {e}"))?;

            println!("Unpublished post {}", post.title);
        }
        ["unpublish", ..] => println!("Syntax: cargo run unpublish <id>"),
        ["delete", post_id] => {
            let post_id = post_id
                .parse::<i32>()
                .map_err(|_| format!("Delete accepts an integer id. Got {post_id}"))?;

            println!("Deleting post {post_id}");

            use self::schema::posts::dsl::*;

            diesel::delete(posts.find(post_id).filter(published.eq(false)))
                .execute(connection)
                .map_err(|e| format!("Could not delete post {post_id}: {e}"))?;
        }
        ["delete", ..] => println!("Syntax: cargo run delete <id>"),
        _ => {
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
        }
    }

    // let server = Server::new(config);

    // server
    //     .start()
    //     .await
    //     .map_err(|e| format!("Failed to start HTTP server {e}"))?;

    Ok(())
}

fn create_post(conn: &mut SqliteConnection, title: &str, body: &str) -> Result<Post, String> {
    use crate::schema::posts;

    let new_post = NewPost { title, body };

    diesel::insert_into(posts::table)
        .values(&new_post)
        .returning(Post::as_returning())
        .get_result(conn)
        .map_err(|e| format!("Failed to insert post: {e}"))
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::posts)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::posts)]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub body: &'a str,
}
