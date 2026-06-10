# gphotos-mcp-rust

![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)
[![CI](https://github.com/whatnick/gphotos-mcp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/whatnick/gphotos-mcp-rust/actions/workflows/ci.yml)
[![Release](https://github.com/whatnick/gphotos-mcp-rust/actions/workflows/release.yml/badge.svg)](https://github.com/whatnick/gphotos-mcp-rust/releases/latest)

Rust MCP server for Google Photos with OAuth2 login and tool-based access to photos, albums, and the photo picker.

---

## Download

Pre-built binaries are attached to every [GitHub Release](https://github.com/whatnick/gphotos-mcp-rust/releases/latest).

| Platform | Architecture | Download |
|---|---|---|
| Linux | x86\_64 | [gphotos-mcp-rust-x86_64-unknown-linux-gnu.tar.gz](https://github.com/whatnick/gphotos-mcp-rust/releases/latest/download/gphotos-mcp-rust-x86_64-unknown-linux-gnu.tar.gz) |
| Linux | ARM64 | [gphotos-mcp-rust-aarch64-unknown-linux-gnu.tar.gz](https://github.com/whatnick/gphotos-mcp-rust/releases/latest/download/gphotos-mcp-rust-aarch64-unknown-linux-gnu.tar.gz) |
| macOS | Apple Silicon | [gphotos-mcp-rust-aarch64-apple-darwin.tar.gz](https://github.com/whatnick/gphotos-mcp-rust/releases/latest/download/gphotos-mcp-rust-aarch64-apple-darwin.tar.gz) |
| macOS | Intel | [gphotos-mcp-rust-x86_64-apple-darwin.tar.gz](https://github.com/whatnick/gphotos-mcp-rust/releases/latest/download/gphotos-mcp-rust-x86_64-apple-darwin.tar.gz) |
| Windows | x86\_64 | [gphotos-mcp-rust-x86_64-pc-windows-msvc.zip](https://github.com/whatnick/gphotos-mcp-rust/releases/latest/download/gphotos-mcp-rust-x86_64-pc-windows-msvc.zip) |

SHA256 checksums are published as `SHA256SUMS.txt` on each release.

### Install via one-liner (Linux / macOS)

```bash
# Linux x86_64
curl -fsSL https://github.com/whatnick/gphotos-mcp-rust/releases/latest/download/gphotos-mcp-rust-x86_64-unknown-linux-gnu.tar.gz | tar xz
chmod +x gphotos-mcp-rust

# macOS Apple Silicon
curl -fsSL https://github.com/whatnick/gphotos-mcp-rust/releases/latest/download/gphotos-mcp-rust-aarch64-apple-darwin.tar.gz | tar xz
chmod +x gphotos-mcp-rust
```

### Build from source

```bash
cargo install --locked --git https://github.com/whatnick/gphotos-mcp-rust
```

---

## Prerequisites: Google Cloud OAuth app

1. Create a project in [Google Cloud Console](https://console.cloud.google.com/).
2. Enable the **Google Photos Library API** and **Google Photos Picker API**.
3. Create an **OAuth 2.0 Client ID** (Application type: Web application).
4. Keep the consent screen in **Testing** and add your own account as a test user.
5. Add `http://localhost:3000/auth/callback` as an authorised redirect URI.
6. Note your **Client ID** and **Client Secret**.

---

## Configuration

Copy the example env file and fill in your credentials:

```bash
cp .env.example .env
```

`.env` variables:

| Variable | Required | Default | Description |
|---|---|---|---|
| `GOOGLE_CLIENT_ID` | ✅ | — | OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | ✅ | — | OAuth client secret |
| `GOOGLE_REDIRECT_URI` | | `http://localhost:3000/auth/callback` | OAuth callback URL |
| `HOST` | | `127.0.0.1` | Listen address |
| `PORT` | | `3000` | Listen port |
| `TOKENS_PATH` | | `~/.config/gphotos-mcp-rust/tokens.json` | Token storage path |

---

## First-time OAuth login

The server must be running before connecting any agent. Start it once and complete the browser consent flow:

```bash
# Export credentials (or use a .env file)
export GOOGLE_CLIENT_ID=your_client_id
export GOOGLE_CLIENT_SECRET=your_client_secret

./gphotos-mcp-rust
```

In another terminal:

```bash
# Get the auth URL
curl -s http://127.0.0.1:3000/auth/start | jq -r .auth_url
```

Open the returned URL in a browser, approve access, and Google will redirect to `/auth/callback`. Tokens are saved to `~/.config/gphotos-mcp-rust/tokens.json` with `0600` permissions and are reused on restart — you only need to do this once unless you revoke access.

Confirm authentication:

```bash
curl -s http://127.0.0.1:3000/health
```

---

## MCP tools

| Tool | Description |
|---|---|
| `auth_status` | Check whether the server is authenticated |
| `start_auth` | Return the OAuth browser URL |
| `search_photos` | Search photos by feature keyword |
| `search_media_by_filter` | Search with a structured filter object |
| `get_photo` | Get details for a single media item |
| `list_albums` | List albums with pagination |
| `get_album` | Get a single album by ID |
| `create_album` | Create a new album |
| `list_album_photos` | List photos in an album |
| `list_media_items` | List all media items |
| `create_picker_session` | Start a Google Photos Picker session |
| `poll_picker_session` | Poll a picker session and retrieve selected items |

---

## Setup with coding agents

This server exposes an **HTTP MCP endpoint** at `http://localhost:3000/mcp`. Start the server before launching any agent session. All agents below connect to that endpoint — no stdio wrapper is needed.

### Crush

Add to `crush.json` in your project root or `~/.config/crush/crush.json`:

```json
{
  "mcp": {
    "google-photos": {
      "type": "http",
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

### Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "google-photos": {
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

Restart Claude Desktop after saving.

### Cursor

Create or edit `~/.cursor/mcp.json` (global) or `.cursor/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "google-photos": {
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

### VS Code (GitHub Copilot)

Create `.vscode/mcp.json` in your workspace:

```json
{
  "servers": {
    "google-photos": {
      "type": "http",
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

### Kiro

Create or edit `.kiro/settings/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "google-photos": {
      "type": "http",
      "url": "http://localhost:3000/mcp",
      "enabled": true
    }
  }
}
```

### Windsurf

Edit `~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "google-photos": {
      "serverUrl": "http://localhost:3000/mcp"
    }
  }
}
```

---

## Auto-start the server (optional)

To keep the server running across reboots, install it as a user service.

**Linux (systemd)**

```ini
# ~/.config/systemd/user/gphotos-mcp-rust.service
[Unit]
Description=Google Photos MCP server

[Service]
ExecStart=/usr/local/bin/gphotos-mcp-rust
EnvironmentFile=%h/.config/gphotos-mcp-rust/env
Restart=on-failure

[Install]
WantedBy=default.target
```

```bash
systemctl --user enable --now gphotos-mcp-rust
```

**macOS (launchd)**

```xml
<!-- ~/Library/LaunchAgents/dev.whatnick.gphotos-mcp-rust.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>dev.whatnick.gphotos-mcp-rust</string>
  <key>ProgramArguments</key>
  <array><string>/usr/local/bin/gphotos-mcp-rust</string></array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>GOOGLE_CLIENT_ID</key>     <string>your_client_id</string>
    <key>GOOGLE_CLIENT_SECRET</key> <string>your_client_secret</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict>
</plist>
```

```bash
launchctl load ~/Library/LaunchAgents/dev.whatnick.gphotos-mcp-rust.plist
```

---

## Security notes

- OAuth state tokens are random (40 chars), single-use, and expire after 10 minutes.
- Access and refresh tokens are stored in a local file with `0600` permissions.
- Tokens are never written to logs.
- CI uses least-privilege permissions, `persist-credentials: false`, and no `pull_request_target` triggers.
- Weekly `cargo audit` runs via the Security workflow.
- Pre-commit hooks block accidental secret commits.

### Enable pre-commit locally

```bash
pip install pre-commit
pre-commit install
```

---

## MCP endpoint example

```bash
curl -s http://127.0.0.1:3000/mcp \
  -H 'content-type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {"name": "auth_status", "arguments": {}}
  }' | jq
```
