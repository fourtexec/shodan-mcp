#!/usr/bin/env bash
# Bash, install.sh, цель: подключить shodan-mcp к OpenCode без затирания существующего конфига
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NAME="shodan"
BIN=""
CONFIG=""
SCOPE="project"
BUILD=0
DRY=0
UNINSTALL=0

usage() {
  cat <<EOF
Usage: ./install.sh [options]

Options:
  --bin PATH        Path to shodan-mcp binary (default: ./target/release/shodan-mcp)
  --config PATH     Explicit opencode.json / opencode.jsonc path (highest priority)
  --name NAME       MCP server name (default: shodan)
  --project         Write to ./opencode.json in the current directory (default)
  --global          Write to \${XDG_CONFIG_HOME:-~/.config}/opencode/opencode.json
  --build           Run 'cargo build --release' before installing
  --dry-run         Print the resulting JSON, do not write anything
  --uninstall       Remove the server entry from the config
  -h, --help        Show this help

Examples:
  ./install.sh
  ./install.sh --config /path/to/opencode.json
  ./install.sh --global --bin /usr/local/bin/shodan-mcp
  ./install.sh --dry-run --config ./opencode.json
  ./install.sh --uninstall --config ./opencode.json
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --bin)       BIN="${2:?--bin needs a value}"; shift 2 ;;
    --config)    CONFIG="${2:?--config needs a value}"; shift 2 ;;
    --name)      NAME="${2:?--name needs a value}"; shift 2 ;;
    --project)   SCOPE="project"; shift ;;
    --global)    SCOPE="global"; shift ;;
    --build)     BUILD=1; shift ;;
    --dry-run)   DRY=1; shift ;;
    --uninstall) UNINSTALL=1; shift ;;
    -h|--help)   usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [ "$BUILD" -eq 1 ]; then
  echo "[*] cargo build --release"
  (cd "$SCRIPT_DIR" && cargo build --release)
fi

if [ -z "$BIN" ]; then
  BIN="$SCRIPT_DIR/target/release/shodan-mcp"
fi
if [ "${BIN#/}" = "$BIN" ]; then
  BIN="$PWD/$BIN"
fi
if [ ! -f "$BIN" ]; then
  echo "[!] binary not found: $BIN" >&2
  echo "    build it first: cargo build --release   (or pass --build)" >&2
  exit 1
fi
if [ ! -x "$BIN" ]; then
  echo "[!] not executable: $BIN" >&2
  exit 1
fi
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"

if [ -z "$CONFIG" ]; then
  if [ "$SCOPE" = "global" ]; then
    CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/opencode/opencode.json"
  else
    CONFIG="$PWD/opencode.json"
  fi
else
  if [ "${CONFIG#/}" = "$CONFIG" ]; then
    CONFIG="$PWD/$CONFIG"
  fi
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "[!] python3 is required to merge JSON safely" >&2
  exit 1
fi

mkdir -p "$(dirname "$CONFIG")"

python3 - "$CONFIG" "$NAME" "$BIN" "$DRY" "$UNINSTALL" <<'PY'
import json
import os
import shutil
import sys

path, name, binpath, dry, uninstall = sys.argv[1:6]
dry = dry == "1"
uninstall = uninstall == "1"

data = {}
if os.path.exists(path) and os.path.getsize(path) > 0:
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        sys.stderr.write(f"[!] cannot parse {path}: {e}\n")
        sys.stderr.write("    file left untouched\n")
        sys.exit(2)

if not isinstance(data, dict):
    sys.stderr.write(f"[!] {path}: root is not a JSON object, left untouched\n")
    sys.exit(2)

mcp = data.get("mcp")
if mcp is None:
    mcp = {}
    data["mcp"] = mcp
if not isinstance(mcp, dict):
    sys.stderr.write(f"[!] {path}: 'mcp' is not an object, left untouched\n")
    sys.exit(2)

servers = mcp.get("servers")
if servers is None:
    servers = {}
    mcp["servers"] = servers
if not isinstance(servers, dict):
    sys.stderr.write(f"[!] {path}: 'mcp.servers' is not an object, left untouched\n")
    sys.exit(2)

if uninstall:
    removed = servers.pop(name, None)
    if removed is None:
        sys.stderr.write(f"[=] server '{name}' is not present in {path}\n")
        sys.exit(0)
    action = "removed"
else:
    servers[name] = {
        "type": "local",
        "command": [binpath],
    }
    action = "installed"

text = json.dumps(data, indent=2, ensure_ascii=False) + "\n"

if dry:
    sys.stdout.write(text)
    sys.stderr.write(f"[=] dry-run: {action} '{name}' in {path}\n")
    sys.exit(0)

if os.path.exists(path):
    shutil.copy2(path, path + ".bak")

tmp = path + ".tmp"
with open(tmp, "w", encoding="utf-8") as f:
    f.write(text)
os.replace(tmp, path)

sys.stderr.write(f"[+] {action} '{name}' in {path}\n")
if os.path.exists(path + ".bak"):
    sys.stderr.write(f"[+] backup: {path}.bak\n")
PY

echo "[+] binary: $BIN"
echo "[+] server: $NAME -> local"
echo "[+] check:  opencode2 mcp list"
