// Rust, src/output.rs, rust 2024, цель: вывод результатов таблицей, JSON или CSV
use crate::model::{FacetHit, Host};

fn trunc(s: &str, n: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= n {
        return flat;
    }
    flat.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
}

fn endpoint(h: &Host) -> String {
    match h.port {
        Some(p) => format!("{}:{}", h.ip, p),
        None => h.ip.clone(),
    }
}

pub fn to_table(hosts: &[Host]) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{:<28} {:<38} {:<26} {:<18} {:<19}\n",
        "IP:PORT", "TITLE", "ORG", "COUNTRY", "SEEN"
    ));
    s.push_str(&format!("{}\n", "-".repeat(133)));
    for h in hosts {
        s.push_str(&format!(
            "{:<28} {:<38} {:<26} {:<18} {:<19}\n",
            trunc(&endpoint(h), 28),
            trunc(&h.title, 38),
            trunc(&h.org, 26),
            trunc(&h.country, 18),
            trunc(&h.timestamp, 19),
        ));
    }
    s
}

pub fn to_json(hosts: &[Host]) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(hosts)?)
}

fn csv_field(s: &str) -> String {
    let guarded = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    format!("\"{}\"", guarded.replace('"', "\"\""))
}

pub fn facet_table(hits: &[FacetHit]) -> String {
    let mut s = String::new();
    s.push_str(&format!("{:<20} {:>8}\n", "IP", "COUNT"));
    s.push_str(&format!("{}\n", "-".repeat(29)));
    for h in hits {
        s.push_str(&format!("{:<20} {:>8}\n", trunc(&h.ip, 20), h.count));
    }
    s
}

pub fn facet_json(hits: &[FacetHit]) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(hits)?)
}

pub fn facet_csv(hits: &[FacetHit]) -> String {
    let mut s = String::from("ip,count\n");
    for h in hits {
        s.push_str(&format!("{},{}", csv_field(&h.ip), h.count));
        s.push('\n');
    }
    s
}

pub fn to_csv(hosts: &[Host]) -> String {
    let mut s = String::from(
        "ip,port,scheme,title,hostnames,org,country,city,tags,timestamp,link,banner\n",
    );
    for h in hosts {
        let port = h.port.map(|p| p.to_string()).unwrap_or_default();
        let row = [
            h.ip.as_str(),
            port.as_str(),
            h.scheme.as_str(),
            h.title.as_str(),
            h.hostnames.as_str(),
            h.org.as_str(),
            h.country.as_str(),
            h.city.as_str(),
            &h.tags.join("|"),
            h.timestamp.as_str(),
            h.link.as_str(),
            h.banner.as_str(),
        ];
        s.push_str(
            &row.iter()
                .map(|f| csv_field(f))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push('\n');
    }
    s
}
