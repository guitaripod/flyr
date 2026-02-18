# flyr-mcp

MCP server for flyr CLI - search Google Flights from AI agents.

## Prerequisites

- [flyr](https://github.com/guitaripod/flyr) installed (`cargo install flyr-cli`)
- Node.js 18+

## Quick Start (opencode)

Add to your opencode config (`~/.config/opencode/opencode.json`):

```json
{
  "mcp": {
    "flyr": {
      "type": "local",
      "command": ["node", "/path/to/flyr/mcp-server/flyr-mcp"],
      "enabled": true
    }
  }
}
```

Or set `FLYR_PATH` if flyr isn't in your PATH:

```json
{
  "mcp": {
    "flyr": {
      "type": "local",
      "command": [
        "env",
        "FLYR_PATH=/full/path/to/flyr",
        "node",
        "/path/to/flyr/mcp-server/flyr-mcp"
      ],
      "enabled": true
    }
  }
}
```

## Tools

### flyr_search

Search for flights using Flyr CLI.

```json
{
  "from": "HEL",
  "to": "BCN,ATH",
  "date": "2026-03-01",
  "return_date": "2026-03-08",
  "seat": "economy",
  "adults": 2,
  "top": 3,
  "currency": "EUR"
}
```

### flyr_get_url

Get the Google Flights URL for a search. Use this to get a URL, then use `open_url` to open it.

```json
{
  "from": "HEL",
  "to": "BCN",
  "date": "2026-03-01"
}
```

### open_url

Open a URL in your default browser.

```json
{
  "url": "https://www.google.com/flights"
}
```

## Example Prompts

> "Find cheap flights from Helsinki to Barcelona tomorrow"

> "Search for warm destinations from HEL next weekend, top 3 results, and open in browser"

## Running Directly

```bash
# Install dependencies (none required for core functionality)
npm install

# Run the server
./flyr-mcp

# Or with custom flyr path
FLYR_PATH=/usr/local/bin/flyr ./flyr-mcp
```

## License

GPL-3.0
