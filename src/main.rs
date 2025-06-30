use chrono::NaiveDate;
use pulldown_cmark::{Options, Parser, html};
use rocket::State;
use rocket::catch;
use rocket::catchers;
use rocket::fs::NamedFile;
use rocket::fs::relative;
use rocket::get;
use rocket::launch;
use rocket::response::Redirect;
use rocket::routes;
use rocket::serde::Serialize;
use rocket::serde::json::Json;
use rocket::shield::Hsts;
use rocket::shield::Shield;
use rocket::time::Duration;
use rocket_dyn_templates::Template;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use toml::Table;

#[derive(Serialize, Debug)]
pub struct Post {
    name: String,
    title: String,
    date: NaiveDate,
    body: String,
}

#[launch]
fn rocket() -> _ {
    let posts_dir = Path::new(relative!("static/posts"));
    let posts = load_posts(posts_dir);

    rocket::build()
        .manage(posts)
        .mount("/", routes![home, postpage, static_pages, version])
        .attach(Template::fairing())
        .attach(Shield::default().enable(Hsts::IncludeSubDomains(Duration::new(31536000, 0))))
        .register("/", catchers![not_found])
}

fn load_posts(posts_dir: &Path) -> Vec<Post> {
    let mut posts = Vec::new();

    if let Ok(entries) = posts_dir.read_dir() {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let file_path = posts_dir.join(&file_name);

                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    let mut lines = content.lines();

                    if lines.next() != Some("+++") {
                        continue;
                    }

                    let mut frontmatter = String::new();

                    for line in &mut lines {
                        if line == "+++" {
                            break;
                        }

                        frontmatter.push_str(line);
                        frontmatter.push('\n');
                    }

                    let body: String = lines.collect::<Vec<_>>().join("\n");

                    if let Ok(toml) = frontmatter.parse::<Table>() {
                        posts.push(Post {
                            name: file_name.clone(),
                            title: toml
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&file_name)
                                .to_string(),
                            date: toml
                                .get("date")
                                .and_then(|v| v.as_str())
                                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                                .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                            body,
                        });
                    }
                }
            }
        }
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));
    posts
}

#[get("/")]
pub async fn home(state: &State<Vec<Post>>) -> Template {
    let mut context = HashMap::new();
    let posts = state.inner();
    context.insert("posts", posts);
    Template::render("home", context)
}

#[get("/<path..>", rank = 2)]
pub async fn static_pages(path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("static")).join(path);

    if path.is_dir() {
        path.push("index.html");
    }

    NamedFile::open(path).await.ok()
}

#[catch(404)]
pub async fn not_found() -> Result<NamedFile, Redirect> {
    Ok(
        NamedFile::open(Path::new(relative!("static")).join("404.html"))
            .await
            .unwrap(),
    )
}

#[derive(Serialize)]
pub struct VersionInfo {
    version: String,
}

#[get("/version")]
pub fn version() -> Json<VersionInfo> {
    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(serde::Serialize)]
struct PostContext<'a> {
    posts: &'a [Post],
    post: Post,
}

#[get("/<postname>", rank = 1)]
pub async fn postpage(postname: String, state: &State<Vec<Post>>) -> Option<Template> {
    let posts = state.inner();
    let post = posts.iter().find(|p| p.name == postname)?;

    let post = Post {
        name: post.name.clone(),
        title: post.title.clone(),
        date: post.date,
        body: markdown_to_html(&post.body),
    };

    let context = PostContext { posts, post };
    Some(Template::render("post", &context))
}

fn markdown_to_html(markdown_input: &str) -> String {
    let parser = Parser::new_ext(markdown_input, Options::all());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
