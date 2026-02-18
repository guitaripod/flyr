# flyr-mcp

MCP server for flyr CLI - search Google Flights from **local AI agents**.

Use with local LLMs (Ollama, LM Studio, etc.) via MCP-compatible clients to have AI agents search and book flights completely offline.

## Why Local LLMs?

- **Privacy** - flight searches never leave your machine
- **Cost** - no API fees, just local compute
- **Speed** - running locally means lower latency
- **Offline** - works without internet for the LLM itself

## Prerequisites

- [flyr](https://github.com/guitaripod/flyr) installed (`cargo install flyr-cli`)
- Node.js 18+
- A local LLM via MCP-compatible client (opencode, Claude Code, etc.)

## Quick Start (opencode + Ollama)

1. Install flyr: `cargo install flyr-cli`

2. Pull a model with tool support:

   ```bash
   ollama pull llama3.1
   ```

3. Add to your opencode config (`~/.config/opencode/opencode.json`):

   ```json
   {
     "mcp": {
       "flyr": {
         "type": "local",
         "command": ["node", "/path/to/flyr/mcp-server/flyr-mcp"],
         "enabled": true
       }
     },
     "provider": {
       "ollama": {
         "options": { "baseURL": "http://localhost:11434/v1" },
         "models": {
           "llama3.1": { "tools": true }
         }
       }
     }
   }
   ```

4. Ask opencode:
   > "find cheap flights from Helsinki to Barcelona tomorrow, open the results in my browser"

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

> "Find round-trip business class from JFK to NRT departing March 1st"

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
