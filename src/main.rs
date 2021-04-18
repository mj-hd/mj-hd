use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::{env, time::Duration};
use tera::{Context, Tera};

#[derive(Deserialize)]
struct GithubRepo {
    html_url: String,
    stargazers_count: u32,
    forks: u32,
}

#[derive(Serialize, Deserialize)]
struct RepoStat {
    name: String,
    url: String,
    stars: u32,
    forks: u32,
}

fn get_repo_stats(client: &Client, projects: Vec<&str>) -> Result<Vec<RepoStat>, String> {
    let mut stats = Vec::with_capacity(projects.len());

    for project in projects.into_iter() {
        let response = client
            .get(format!("https://api.github.com/repos/{}", project))
            .send()
            .map_err(|_e| "failed to request repo")?
            .json::<GithubRepo>()
            .map_err(|e| format!("failed to parse repo response {}", e))?;

        stats.push(RepoStat {
            name: project.to_string(),
            url: response.html_url,
            stars: response.stargazers_count,
            forks: response.forks,
        });
    }

    Ok(stats)
}

#[derive(Deserialize)]
struct GithubGist {
    description: String,
    html_url: String,
}

#[derive(Serialize, Deserialize)]
struct Gist {
    title: String,
    url: String,
}

fn get_gists(client: &Client, username: String, limit: usize) -> Result<Vec<Gist>, String> {
    let mut gists = Vec::with_capacity(limit);

    let responses = client
        .get(format!(
            "https://api.github.com/users/{}/gists?per_page={}",
            username, limit
        ))
        .send()
        .map_err(|_e| "failed to request gists")?
        .json::<Vec<GithubGist>>()
        .map_err(|_e| "failed to parse repo response")?;

    for response in responses.into_iter() {
        gists.push(Gist {
            title: response.description,
            url: response.html_url,
        });
    }

    Ok(gists)
}

#[derive(Deserialize)]
struct Rss {
    channel: Vec<RssChannel>,
}

#[derive(Deserialize)]
struct RssChannel {
    item: Vec<RssItem>,
}

#[derive(Deserialize)]
struct RssItem {
    title: String,
    link: String,
}

#[derive(Serialize, Deserialize)]
struct Post {
    title: String,
    url: String,
}

fn get_posts(client: &Client, rss: String, limit: usize) -> Result<Vec<Post>, String> {
    let mut posts = Vec::with_capacity(limit);

    let response = client
        .get(rss)
        .send()
        .map_err(|_e| "failed to request posts")?
        .text()
        .map_err(|_e| "failed to get post response")?;

    let rss = serde_xml_rs::from_str::<Rss>(&response[..]).map_err(|_e| "failed to parse rss")?;

    for item in rss.channel[0].item.iter() {
        if posts.len() >= limit {
            break;
        }

        posts.push(Post {
            title: item.title.clone(),
            url: item.link.clone(),
        });
    }

    Ok(posts)
}

fn update(context: &Context) -> Result<(), String> {
    let mut file = File::create("README.md").map_err(|_e| "failed to open file")?;

    let tera = Tera::new("*.tmpl").map_err(|e| format!("failed to load template {:?}", e))?;

    tera.render_to("README.md.tmpl", context, &mut file)
        .map_err(|_e| "failed to render")?;

    file.flush().map_err(|_e| "failed to flush")?;

    Ok(())
}

fn main() {
    let mut context = Context::new();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(5000))
        .default_headers({
            let mut headers = HeaderMap::new();

            headers.insert("user-agent", "MJHD".parse().unwrap());

            headers
        })
        .build()
        .unwrap();

    if let Ok(line) = env::var("README_DEVICONS") {
        let devicons = line.split(',').collect::<Vec<_>>();
        context.insert("devicons", &devicons);
    }

    if let Ok(line) = env::var("README_PROJECTS") {
        let projects = line.split(',').collect::<Vec<_>>();
        context.insert("projects", &get_repo_stats(&client, projects).unwrap());
    }

    let gists = get_gists(&client, "mj-hd".to_string(), 5).unwrap();
    context.insert("gists", &gists);

    let posts = get_posts(&client, "https://mjhd.hatenablog.com/rss".to_string(), 5).unwrap();
    context.insert("posts", &posts);

    update(&context).unwrap();
}
