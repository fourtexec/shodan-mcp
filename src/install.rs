// Rust, src/install.rs, rust 2024, цель: подкоманда install — прописать shodan-mcp в opencode.json без затирания
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub struct Opts {
    pub config: Option<PathBuf>,
    pub global: bool,
    pub name: String,
    pub bin: Option<PathBuf>,
    pub uninstall: bool,
    pub dry_run: bool,
}

pub fn run(opts: Opts) -> Result<()> {
    let bin = if opts.uninstall {
        None
    } else {
        Some(resolve_bin(opts.bin.as_deref())?)
    };
    let config = resolve_config(opts.config.as_deref(), opts.global)?;
    let mut data = read_json(&config)?;

    let root = data
        .as_object_mut()
        .context("config root is not a JSON object")?;
    let mcp = root.entry("mcp").or_insert_with(|| json!({}));
    let mcp_obj = mcp.as_object_mut().context("'mcp' is not an object")?;
    let servers = mcp_obj.entry("servers").or_insert_with(|| json!({}));
    let servers_obj = servers
        .as_object_mut()
        .context("'mcp.servers' is not an object")?;

    if opts.uninstall {
        if servers_obj.remove(&opts.name).is_none() {
            eprintln!(
                "[=] server '{}' not present in {}",
                opts.name,
                config.display()
            );
            return Ok(());
        }
        eprintln!("[-] removed '{}' from {}", opts.name, config.display());
    } else {
        let bin = bin.expect("binary path resolved");
        servers_obj.insert(
            opts.name.clone(),
            json!({
                "type": "local",
                "command": [bin.to_string_lossy()]
            }),
        );
        eprintln!("[+] installed '{}' -> {}", opts.name, bin.display());
    }

    let text = serde_json::to_string_pretty(&data)? + "\n";
    if opts.dry_run {
        print!("{text}");
        eprintln!("[=] dry-run, {} not written", config.display());
        return Ok(());
    }

    if config.exists() {
        let bak = PathBuf::from(format!("{}.bak", config.display()));
        fs::copy(&config, &bak).with_context(|| format!("backup {}", bak.display()))?;
        eprintln!("[+] backup: {}", bak.display());
    }
    if let Some(dir) = config.parent() {
        fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    let tmp = PathBuf::from(format!("{}.tmp", config.display()));
    fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, &config).with_context(|| format!("rename to {}", config.display()))?;
    eprintln!("[+] wrote {}", config.display());
    eprintln!("[+] check: opencode2 mcp list");
    Ok(())
}

fn resolve_config(explicit: Option<&Path>, global: bool) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(absolute(path)?);
    }
    if global {
        let base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .context("neither XDG_CONFIG_HOME nor HOME is set")?;
        return Ok(base.join("opencode").join("opencode.json"));
    }
    Ok(env::current_dir()?.join("opencode.json"))
}

fn resolve_bin(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        let path = absolute(path)?;
        if !path.is_file() {
            bail!("binary not found: {}", path.display());
        }
        return Ok(path.canonicalize()?);
    }

    let exe = env::current_exe().context("cannot resolve current executable")?;
    let dir = exe
        .parent()
        .context("current executable has no parent directory")?;
    let name = if cfg!(windows) {
        "shodan-mcp.exe"
    } else {
        "shodan-mcp"
    };
    let candidate = dir.join(name);
    if !candidate.is_file() {
        bail!(
            "{} not found next to {}; pass --bin PATH",
            name,
            exe.display()
        );
    }
    Ok(candidate.canonicalize()?)
}

fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

fn read_json(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&text)
        .with_context(|| format!("cannot parse {} — file left untouched", path.display()))
}
