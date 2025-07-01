use chrono::NaiveDate;
use pulldown_cmark::{Options, Parser, html};
use rocket::State;
use rocket::catch;
use rocket::catchers;
use rocket::fs::NamedFile;
use rocket::fs::relative;
use rocket::get;
use rocket::http::ContentType;
use rocket::launch;
use rocket::response::Redirect;
use rocket::routes;
use rocket::serde::Serialize;
use rocket::serde::json::Json;
use rocket::shield::Hsts;
use rocket::shield::Shield;
use rocket::time::Duration;
use rocket_dyn_templates::Template;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Instant;
use toml::Table;

const RELOAD_THROTTLE_SECONDS: i64 = 10;

#[derive(Serialize, Clone, Debug)]
pub struct Post {
    slug: String,
    title: String,
    date: NaiveDate,
    friendly_date: String,
    body: String,
}

#[launch]
fn rocket() -> _ {
    let posts = load_posts();

    let reload_state = Mutex::new(ReloadState {
        last_reload: Instant::now() - Duration::new(RELOAD_THROTTLE_SECONDS, 0),
    });

    rocket::build()
        .manage(RwLock::new(posts))
        .manage(reload_state)
        .mount(
            "/",
            routes![home, postpage, static_pages, version, rss_feed, reload],
        )
        .attach(Template::fairing())
        .attach(Shield::default().enable(Hsts::IncludeSubDomains(Duration::new(31536000, 0))))
        .register("/", catchers![not_found])
}

fn load_posts() -> Vec<Post> {
    let posts_dir = Path::new(relative!("static/posts"));
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
                        let date = toml
                            .get("date")
                            .and_then(|v| v.as_str())
                            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

                        posts.push(Post {
                            slug: file_name.clone(),
                            title: toml
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&file_name)
                                .to_string(),
                            date,
                            friendly_date: date.format("%B %d, %Y").to_string(),
                            body: markdown_to_html(&body),
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
pub async fn home(state: &State<RwLock<Vec<Post>>>) -> Redirect {
    let posts = {
        let posts = state.read().unwrap();
        posts.clone()
    };

    let latest_post = posts.first().expect("No posts found");
    Redirect::to(format!("/{}", latest_post.slug))
}

#[get("/static/<path..>")]
pub async fn static_pages(path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("static")).join(path);

    if path.is_dir() {
        path.push("index.html");
    }

    NamedFile::open(path).await.ok()
}

#[catch(404)]
pub async fn not_found() -> Redirect {
    Redirect::to("/")
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
struct PostContext {
    posts: Vec<Post>,
    post: Post,
}

#[get("/<postname>")]
pub async fn postpage(postname: &str, state: &State<RwLock<Vec<Post>>>) -> Option<Template> {
    let posts = {
        let posts = state.read().unwrap();
        posts.clone()
    };

    let post = posts.iter().find(|p| p.slug == postname)?.clone();
    let context = PostContext { posts, post };
    Some(Template::render("post", &context))
}

fn markdown_to_html(markdown_input: &str) -> String {
    let parser = Parser::new_ext(markdown_input, Options::all());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[get("/rss")]
fn rss_feed(state: &State<RwLock<Vec<Post>>>) -> (ContentType, String) {
    let posts = {
        let posts = state.read().unwrap();
        posts.clone()
    };

    let mut rss = String::new();
    rss.push_str(r#"<?xml version="1.0" encoding="UTF-8" ?>"#);
    rss.push_str(r#"<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:atom="http://www.w3.org/2005/Atom"><channel>"#);
    rss.push_str("<title>bl.elg.gg</title>");
    rss.push_str("<link>https://bl.elg.gg/</link>");
    rss.push_str("<description>bl.elg.gg</description>");
    rss.push_str("<language>en-us</language>");

    rss.push_str(
        r#"<atom:link rel="self" href="https://bl.elg.gg/rss" type="application/rss+xml" />"#,
    );

    for post in posts.iter().take(10) {
        let pub_date = format!("{} 00:00:00 GMT", post.date.format("%a, %d %b %Y"));
        rss.push_str("<item>");
        rss.push_str(&format!("<title>{}</title>", xml_escape(&post.title)));

        rss.push_str(&format!(
            "<link>https://bl.elg.gg/{}</link>",
            xml_escape(&post.slug)
        ));

        rss.push_str(&format!(
            "<description>{}</description>",
            xml_escape(&post.title)
        ));

        rss.push_str(&format!("<pubDate>{}</pubDate>", pub_date));

        rss.push_str(&format!(
            "<guid>https://bl.elg.gg/{}</guid>",
            xml_escape(&post.slug)
        ));

        rss.push_str(&format!(
            "<content:encoded><![CDATA[{}]]></content:encoded>",
            markdown_to_html(&post.body)
        ));

        rss.push_str("</item>");
    }

    rss.push_str("</channel></rss>");
    (ContentType::XML, rss)
}

fn xml_escape(input: &str) -> String {
    input
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

struct ReloadState {
    last_reload: Instant,
}

#[get("/reload")]
fn reload(state: &State<Mutex<ReloadState>>, posts: &State<RwLock<Vec<Post>>>) -> String {
    let mut reload_state = state.lock().unwrap();
    let now = Instant::now();

    if now.duration_since(reload_state.last_reload) < Duration::new(RELOAD_THROTTLE_SECONDS, 0) {
        return "Reload throttled.".into();
    }

    let mut posts_lock = posts.write().unwrap();
    *posts_lock = load_posts();
    reload_state.last_reload = now;
    "Posts reloaded.".into()
}
