// Rust, src/model.rs, rust 2024, цель: структура одного результата Shodan
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Host {
    pub ip: String,
    pub port: Option<u16>,
    pub scheme: String,
    pub title: String,
    pub hostnames: String,
    pub org: String,
    pub country: String,
    pub city: String,
    pub tags: Vec<String>,
    pub timestamp: String,
    pub link: String,
    pub banner: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FacetHit {
    pub ip: String,
    pub count: u64,
}
