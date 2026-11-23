// Rust, src/client.rs, rust 2024, цель: HTTP-клиент и запросы к shodan.io
use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;

pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/124.0 Safari/537.36";

pub fn build_client(proxy: Option<&str>) -> Result<Client> {
    let mut builder = Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(30))
        .cookie_store(true);
    if let Some(p) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(p).context("bad proxy url")?);
    }
    Ok(builder.build()?)
}

pub fn get_html(client: &Client, url: &str, cookie: Option<&str>) -> Result<String> {
    let mut req = client
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml")
        .header("Accept-Language", "en-US,en;q=0.9");
    if let Some(c) = cookie {
        req = req.header("Cookie", c);
    }
    let resp = req.send().with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if status == 403 || status == 429 {
        bail!("Shodan returned {status}: slow down or use a cookie");
    }
    if !status.is_success() {
        bail!("Shodan returned {status}");
    }
    resp.text().context("read body")
}

pub fn fetch_page(client: &Client, query: &str, cookie: Option<&str>) -> Result<String> {
    let url = format!(
        "https://www.shodan.io/search?query={}&page=1",
        urlencoding::encode(query)
    );
    get_html(client, &url, cookie)
}

pub fn fetch_facet(client: &Client, query: &str, cookie: Option<&str>) -> Result<String> {
    let url = format!(
        "https://www.shodan.io/search/facet?query={}&facet=ip",
        urlencoding::encode(query)
    );
    get_html(client, &url, cookie)
}
