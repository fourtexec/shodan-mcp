# shodan-rs

CLI-скрейпер **первой страницы** `shodan.io/search` без API-ключа. Принимает произвольный запрос, парсит HTML-выдачу и отдаёт результаты таблицей, JSON или CSV.

Всегда запрашивает только `page=1`.

## Сборка

```bash
cargo build --release
# бинари: target/release/shodan-rs, target/release/shodan-mcp
```

## MCP-сервер

`shodan-mcp` — JSON-RPC 2.0 over stdio, два инструмента:

| инструмент | аргументы | что возвращает |
|---|---|---|
| `shodan_search` | `query`, `cookie?`, `proxy?` | JSON-массив карточек (page=1), при лимите — facet-IP |
| `shodan_facet` | `query`, `cookie?`, `proxy?` | JSON `[{ip, count}]`, работает с фильтрами и без лимита |

Подключение в opencode (`opencode.jsonc`, V2 — `mcp.servers`):

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "shodan": {
        "type": "local",
        "command": ["/data/data/com.termux/files/home/op/shodan-rs/target/release/shodan-mcp"]
      }
    }
  }
}
```

Claude Desktop:

```json
{
  "mcpServers": {
    "shodan": {
      "command": "/data/data/com.termux/files/home/op/shodan-rs/target/release/shodan-mcp",
      "args": []
    }
  }
}
```

Проверка вручную:

```bash
printf '%s\n' \
'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}' \
'{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
| ./target/release/shodan-mcp
```

## Использование

```bash
# свободный текстовый запрос — работает без логина
./target/release/shodan-rs ollama

# JSON в stdout
./target/release/shodan-rs "openssh" --json

# таблица в файл + CSV
./target/release/shodan-rs "nginx" -o result.txt --csv result.csv

# через прокси
./target/release/shodan-rs ollama --proxy http://127.0.0.1:8080

# facet-режим: список IP без лимита поиска
./target/release/shodan-rs 'port:11434 product:"Ollama"' --facet
```

## Что отдаёт Shodan

Проверено вручную через curl (2026-11-23):

| запрос | без cookie |
|---|---|
| `query=ollama&page=1` | 200, 10 результатов, всего 846 |
| `query=port:11434` | `Please log in to use search filters` |
| `search/facet?query=...&facet=ip` | 200, список IP, без лимита и без логина |

Анонимно доступна только первая страница свободного текстового запроса. Операторы (`port:`, `product:`, `country:`) на `/search` требуют cookie, но на `/search/facet` работают и без неё.

Если `page=1` поймал `Search usage limit reached`, инструмент автоматически переключается на facet и отдаёт список IP. Принудительно: `--facet`.

Периодически Shodan отдаёт `Search usage limit reached` — это временный троттлинг по IP, через паузу снова работает.

## Фильтры и cookie

`port:11434`, `product:"Ollama"`, `country:DE` и прочие операторы без авторизации отдают `Please log in to use search filters`. Чтобы фильтры работали — передай cookie залогиненной сессии:

```bash
export SHDN_COOKIE='session=...; ...'
./target/release/shodan-rs 'port:11434 product:"Ollama"'
```

Cookie берётся из DevTools → Network → любой запрос к `shodan.io` → заголовок `Cookie`.

## Что вытаскивает

`ip`, `port`, `scheme`, `title`, `hostnames`, `org`, `country`, `city`, `tags`, `timestamp`, `link`, `banner`.

## Флаги

| флаг | значение |
|---|---|
| `--json` | JSON вместо таблицы |
| `--csv FILE` | дополнительно записать CSV |
| `-o, --out FILE` | stdout в файл |
| `--cookie COOKIE` | cookie сессии (или env `SHDN_COOKIE`) |
| `--facet` | facet-режим: только список IP, без лимита |
| `--proxy URL` | HTTP-прокси |
