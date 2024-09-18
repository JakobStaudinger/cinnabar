use std::io::{stdin, Read};

use api::Server;
use config::AppConfig;

mod api;
mod config;
mod orchestrator;
mod parser;
mod runner;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = AppConfig::from_environment()?;
    // let connection = &mut SqliteConnection::establish(&config.database.url)
    //     .map_err(|e| format!("Failed to connect to database {e}"))?;

    // let mut repository = PostsRepositoryImpl { connection };

    // let args = std::env::args().skip(1).collect::<Vec<_>>();
    // let args = args.iter().map(|arg| arg.as_str()).collect::<Vec<_>>();

    // match args[..] {
    //     ["write"] => {
    //         let mut title = String::new();
    //         let mut body = String::new();

    //         println!("What would you like to name your post?");
    //         stdin()
    //             .read_line(&mut title)
    //             .map_err(|e| format!("Failed to read from stdin: {e}"))?;

    //         let title = title.trim_end();

    //         println!("Start writing! (Press Ctrl+D when done)");
    //         stdin()
    //             .read_to_string(&mut body)
    //             .map_err(|e| format!("Failed to read from stdin: {e}"))?;

    //         let body = body.as_str();

    //         let post_id = repository
    //             .create_post(&NewPost { title, body })
    //             .map_err(|_| "Failed to insert post")?;

    //         println!("Saved draft post {title} with id {post_id}");
    //     }
    //     ["write", ..] => println!("Syntax: cargo run write"),
    //     ["publish", post_id] => {
    //         let post_id = post_id
    //             .parse::<i32>()
    //             .map_err(|_| format!("Publish accepts an integer id. Got {post_id}"))?;

    //         println!("Publishing post {post_id}");

    //         repository
    //             .publish_post(post_id)
    //             .map_err(|_| "Failed to publish post")?;

    //         println!("Published post {post_id}");
    //     }
    //     ["publish", ..] => println!("Syntax: cargo run publish <id>"),
    //     ["unpublish", post_id] => {
    //         let post_id = post_id
    //             .parse::<i32>()
    //             .map_err(|_| format!("Unpublish accepts an integer id. Got {post_id}"))?;

    //         println!("Unpublishing post {post_id}");

    //         repository
    //             .unpublish_post(post_id)
    //             .map_err(|_| "Failed to unpublish post")?;

    //         println!("Unpublished post {post_id}");
    //     }
    //     ["unpublish", ..] => println!("Syntax: cargo run unpublish <id>"),
    //     ["delete", post_id] => {
    //         let post_id = post_id
    //             .parse::<i32>()
    //             .map_err(|_| format!("Delete accepts an integer id. Got {post_id}"))?;

    //         println!("Deleting post {post_id}");

    //         repository
    //             .delete_post(post_id)
    //             .map_err(|_| "Failed to delete post")?;

    //         println!("Deleted post {post_id}");
    //     }
    //     ["delete", ..] => println!("Syntax: cargo run delete <id>"),
    //     _ => {
    //         let posts = repository
    //             .get_published_posts()
    //             .map_err(|_| "Failed to query posts")?;

    //         for post in posts {
    //             println!("{}", post.title);
    //             println!("------------");
    //             println!("{}", post.body);
    //         }
    //     }
    // }

    let server = Server::new(config);

    server
        .start()
        .await
        .map_err(|e| format!("Failed to start HTTP server {e}"))?;

    Ok(())
}

// #[derive(Queryable, Selectable)]
// #[diesel(table_name = crate::schema::posts)]
// pub struct Post {
//     pub id: i32,
//     pub title: String,
//     pub body: String,
//     pub published: bool,
// }

// #[derive(Insertable)]
// #[diesel(table_name = crate::schema::posts)]
// pub struct NewPost<'a> {
//     pub title: &'a str,
//     pub body: &'a str,
// }

// trait PostsRepository {
//     fn get_published_posts(&mut self) -> Result<Vec<Post>, ()>;
//     fn create_post(&mut self, post: &NewPost) -> Result<i32, ()>;
//     fn publish_post(&mut self, post_id: i32) -> Result<(), ()>;
//     fn unpublish_post(&mut self, post_id: i32) -> Result<(), ()>;
//     fn delete_post(&mut self, post_id: i32) -> Result<(), ()>;
// }

// struct PostsRepositoryImpl<'a> {
//     connection: &'a mut SqliteConnection,
// }

// impl<'a> PostsRepository for PostsRepositoryImpl<'a> {
//     fn get_published_posts(&mut self) -> Result<Vec<Post>, ()> {
//         use crate::schema::posts::dsl::*;

//         posts
//             .filter(published.eq(true))
//             .limit(5)
//             .select(Post::as_select())
//             .load(self.connection)
//             .map_err(|_| ())
//     }

//     fn create_post(&mut self, post: &NewPost) -> Result<i32, ()> {
//         use crate::schema::posts;

//         let post = diesel::insert_into(posts::table)
//             .values(post)
//             .returning(Post::as_returning())
//             .get_result(self.connection)
//             .map_err(|e| format!("Failed to insert post: {e}"))
//             .map_err(|_| ())?;

//         Ok(post.id)
//     }

//     fn publish_post(&mut self, post_id: i32) -> Result<(), ()> {
//         use self::schema::posts::dsl::{posts, published};

//         diesel::update(posts.find(post_id))
//             .set(published.eq(true))
//             .returning(Post::as_returning())
//             .get_result(self.connection)
//             .map_err(|_| ())?;

//         Ok(())
//     }

//     fn unpublish_post(&mut self, post_id: i32) -> Result<(), ()> {
//         use self::schema::posts::dsl::{posts, published};

//         diesel::update(posts.find(post_id))
//             .set(published.eq(false))
//             .returning(Post::as_returning())
//             .get_result(self.connection)
//             .map_err(|_| ())?;

//         Ok(())
//     }

//     fn delete_post(&mut self, post_id: i32) -> Result<(), ()> {
//         use self::schema::posts::dsl::*;

//         let result = diesel::delete(posts.find(post_id).filter(published.eq(false)))
//             .execute(self.connection)
//             .map_err(|_| ())?;

//         match result {
//             1 => Ok(()),
//             _ => Err(()),
//         }
//     }
// }
