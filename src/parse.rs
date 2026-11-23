// Rust, src/parse.rs, rust 2024, цель: разбор HTML-выдачи shodan.io/search в структуры Host
use scraper::{ElementRef, Html, Selector};

use crate::model::{FacetHit, Host};

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("static selector")
}

fn text_of(el: Option<ElementRef>) -> String {
    match el {
        Some(e) => collapse(&e.text().collect::<Vec<_>>().join(" ")),
        None => String::new(),
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_link(link: &str) -> (String, Option<u16>) {
    match url::Url::parse(link) {
        Ok(u) => (u.scheme().to_string(), u.port_or_known_default()),
        Err(_) => (String::new(), None),
    }
}

pub fn parse_page(html: &str) -> Vec<Host> {
    let doc = Html::parse_document(html);

    let s_result = sel("div.result");
    let s_title = sel("div.heading a.title");
    let s_ext = sel("div.heading a.text-danger");
    let s_ts = sel("div.timestamp");
    let s_hostnames = sel("li.hostnames");
    let s_org = sel("a.filter-org");
    let s_detail_li = sel("div.result-details li");
    let s_flag = sel("img.flag");
    let s_a = sel("a");
    let s_tag = sel("li.tags a.tag");
    let s_banner = sel("div.banner-data pre");

    let mut out = Vec::new();
    for r in doc.select(&s_result) {
        let title_el = r.select(&s_title).next();
        let href = title_el
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default();
        let ip = href
            .trim_start_matches("/host/")
            .trim_end_matches('/')
            .to_string();
        if ip.is_empty() {
            continue;
        }

        let ext = r
            .select(&s_ext)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default();
        let (scheme, port) = split_link(ext);

        let mut country = String::new();
        let mut city = String::new();
        for li in r.select(&s_detail_li) {
            if let Some(img) = li.select(&s_flag).next() {
                country = img.value().attr("title").unwrap_or_default().to_string();
                if let Some(a) = li.select(&s_a).last() {
                    city = text_of(Some(a));
                }
            }
        }

        let tags = r
            .select(&s_tag)
            .map(|e| collapse(&e.text().collect::<Vec<_>>().join(" ")))
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>();

        let banner = r
            .select(&s_banner)
            .next()
            .map(|e| e.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        out.push(Host {
            ip,
            port,
            scheme,
            title: text_of(title_el),
            hostnames: text_of(r.select(&s_hostnames).next()),
            org: text_of(r.select(&s_org).next()),
            country,
            city,
            tags,
            timestamp: text_of(r.select(&s_ts).next()),
            link: ext.to_string(),
            banner,
        });
    }
    out
}

pub fn parse_facet(html: &str) -> Vec<FacetHit> {
    let doc = Html::parse_document(html);
    let s_name = sel("div.facets-card div.name a");
    let s_value = sel("div.facets-card div.value");

    let names: Vec<String> = doc
        .select(&s_name)
        .map(|e| collapse(&e.text().collect::<Vec<_>>().join(" ")))
        .collect();
    let counts: Vec<u64> = doc
        .select(&s_value)
        .map(|e| e.text().collect::<String>().trim().parse().unwrap_or(0))
        .collect();

    names
        .into_iter()
        .zip(counts)
        .filter(|(ip, _)| !ip.is_empty())
        .map(|(ip, count)| FacetHit { ip, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_result() {
        let html = r##"<div class="result">
            <div class="heading">
                <a href="/host/1.2.3.4" class="title text-dark">SSH-2.0</a>
                <a href="http://1.2.3.4:22" class="text-danger">x</a>
                <div class="timestamp text-secondary normalize">2026-01-01T00:00:00</div>
            </div>
            <div class="result-details">
                <ul>
                    <li class="hostnames text-secondary">host.example</li>
                    <li><a href="#" class="filter-link filter-org">Acme</a></li>
                    <li><img class="flag" title="Germany"/><a href="#">Germany</a>, <a href="#">Berlin</a></li>
                    <li class="tags"><a class="tag" href="#">ssh</a></li>
                </ul>
            </div>
            <div class="banner-data"><pre>SSH-2.0-OpenSSH_9.6</pre></div>
        </div>"##;
        let hosts = parse_page(html);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].ip, "1.2.3.4");
        assert_eq!(hosts[0].port, Some(22));
        assert_eq!(hosts[0].country, "Germany");
        assert_eq!(hosts[0].city, "Berlin");
        assert_eq!(hosts[0].tags, vec!["ssh".to_string()]);
    }
}
