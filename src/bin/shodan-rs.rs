// Rust, src/bin/shodan-rs.rs, rust 2024, цель: CLI shodan-rs — поиск + подкоманда install
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};

use shodan_rs::{client, install, output, parse};

#[derive(Parser, Debug)]
#[command(
    name = "shodan-rs",
    version,
    about = "Shodan scraper + installer — no API key",
    args_conflicts_with_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Search query, e.g. 'ollama'. Filters like port:11434 work via facet.
    query: Option<String>,

    /// Output JSON instead of a table
    #[arg(long)]
    json: bool,

    /// Also write CSV to this path
    #[arg(long, value_name = "FILE")]
    csv: Option<PathBuf>,

    /// Write stdout output to this file
    #[arg(short, long, value_name = "FILE")]
    out: Option<PathBuf>,

    /// Session cookie (or env SHDN_COOKIE)
    #[arg(long)]
    cookie: Option<String>,

    /// HTTP proxy, e.g. http://127.0.0.1:8080
    #[arg(long)]
    proxy: Option<String>,

    /// Use the facet endpoint (plain IP list, not hit by the search limit)
    #[arg(long)]
    facet: bool,

    /// Debug: parse a saved HTML file instead of hitting the network
    #[arg(long, value_name = "FILE", hide = true)]
    html: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Install shodan-mcp into an OpenCode config without touching other settings
    Install(InstallArgs),
}

#[derive(Args, Debug)]
struct InstallArgs {
    /// Explicit opencode.json / opencode.jsonc path
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Write to ~/.config/opencode/opencode.json
    #[arg(long)]
    global: bool,

    /// MCP server name
    #[arg(long, default_value = "shodan")]
    name: String,

    /// Path to shodan-mcp (default: next to this binary)
    #[arg(long, value_name = "FILE")]
    bin: Option<PathBuf>,

    /// Remove the server entry instead of adding it
    #[arg(long)]
    uninstall: bool,

    /// Print the resulting JSON, write nothing
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Command::Install(args)) = cli.command {
        return install::run(install::Opts {
            config: args.config,
            global: args.global,
            name: args.name,
            bin: args.bin,
            uninstall: args.uninstall,
            dry_run: args.dry_run,
        });
    }

    let query = cli.query.as_deref().context("query required")?;
    let cookie = cli
        .cookie
        .clone()
        .or_else(|| std::env::var("SHDN_COOKIE").ok())
        .filter(|c| !c.trim().is_empty());

    let client = client::build_client(cli.proxy.as_deref())?;

    let mut facet_mode = cli.facet;
    let mut body = match &cli.html {
        Some(path) => {
            eprintln!("[*] reading {}", path.display());
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?
        }
        None if facet_mode => client::fetch_facet(&client, query, cookie.as_deref())?,
        None => client::fetch_page(&client, query, cookie.as_deref())?,
    };
    eprintln!("[*] {}", if facet_mode { "facet" } else { "page=1" });

    if !facet_mode && body.contains("Search usage limit reached") {
        eprintln!("[!] page=1 hit the anonymous limit — falling back to facet");
        facet_mode = true;
        body = client::fetch_facet(&client, query, cookie.as_deref())?;
    }
    if body.to_ascii_lowercase().contains("captcha") {
        bail!("captcha/challenge returned — retry later or use --cookie");
    }
    if body.contains("Please log in to use search filters") {
        eprintln!("[!] filters require a logged-in session — pass --cookie or SHDN_COOKIE");
    }

    if facet_mode {
        let hits = parse::parse_facet(&body);
        eprintln!("[+] {} ips", hits.len());
        let rendered = if cli.json {
            output::facet_json(&hits)?
        } else {
            output::facet_table(&hits)
        };
        emit(&rendered, cli.out.as_ref())?;
        if let Some(path) = &cli.csv {
            fs::write(path, output::facet_csv(&hits))
                .with_context(|| format!("write {}", path.display()))?;
            eprintln!("[+] csv -> {}", path.display());
        }
        return Ok(());
    }

    let mut seen: HashSet<(String, Option<u16>)> = HashSet::new();
    let hosts: Vec<_> = parse::parse_page(&body)
        .into_iter()
        .filter(|h| seen.insert((h.ip.clone(), h.port)))
        .collect();
    eprintln!("[+] {} hosts", hosts.len());

    let rendered = if cli.json {
        output::to_json(&hosts)?
    } else {
        output::to_table(&hosts)
    };
    emit(&rendered, cli.out.as_ref())?;

    if let Some(path) = &cli.csv {
        fs::write(path, output::to_csv(&hosts))
            .with_context(|| format!("write {}", path.display()))?;
        eprintln!("[+] csv -> {}", path.display());
    }

    Ok(())
}

fn emit(rendered: &str, out: Option<&PathBuf>) -> Result<()> {
    match out {
        Some(path) => {
            fs::write(path, rendered).with_context(|| format!("write {}", path.display()))?
        }
        None => print!("{rendered}"),
    }
    Ok(())
}
