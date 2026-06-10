# gphotos-mcp-rust

![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)

Rust bootstrap for a Google Photos MCP server with OAuth-based login and tool-based access to common Google Photos use cases.

## Implemented MCP tools

- `auth_status`
- `start_auth`
- `search_photos`
- `search_media_by_filter`
- `get_photo`
- `list_albums`
- `get_album`
- `create_album`
- `list_album_photos`
- `list_media_items`
- `create_picker_session`
- `poll_picker_session`

## Quick start for personal use

1. Create an OAuth client in Google Cloud and keep the consent screen in **Testing**.
2. Add your own email address under **Test users** on the OAuth consent screen.
3. If Google shows the unverified-app warning, choose **Advanced** and continue as your own test user.
4. Add your callback URI (for example `http://localhost:3000/auth/callback`).
5. Copy and fill env vars:

```bash
cp .env.example .env
```

6. Run:

```bash
cargo run
```

7. Start auth:

```bash
curl http://127.0.0.1:3000/auth/start
```

8. Open returned `auth_url`, complete consent, then use `/mcp`.

## OAuth login before MCP usage

This server requires an OAuth login before most MCP tools will work.
The current consent flow requests broader Google Photos access so album and picker tools can use the same token set.
It is intended for a personal test app / single-user setup, not a public multi-user deployment.

If you are the only user, you do not need Google's formal verification process. Keep the app in Testing and only authorize with the test-user account you added.

1. Start the server:

```bash
cargo run
```

2. Start the login flow:

```bash
curl http://127.0.0.1:3000/auth/start
```

3. Open the returned `auth_url` in a browser and approve access.
4. After Google redirects to `/auth/callback`, the server stores tokens locally.
5. Confirm the session:

```bash
curl http://127.0.0.1:3000/health
```

6. Call `auth_status` or any other MCP tool through `/mcp`.

If `auth_status` reports unauthenticated, rerun `/auth/start` and complete the browser consent flow again.
If you previously authorized the app, revoke access in your Google Account permissions page or delete the local token file before restarting the flow so Google prompts for the expanded scopes again.

If Google shows a warning screen, that is expected while the app remains in Testing mode; continue only with your own test user account.

## MCP endpoint example

```bash
curl -s http://127.0.0.1:3000/mcp \
  -H 'content-type: application/json' \
  -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"tools/call",
    "params":{"name":"auth_status","arguments":{}}
  }' | jq
```

## Security notes

- OAuth state is random, short-lived, and single-use.
- Tokens are stored in a local file with strict permissions (`0600` on Unix).
- CI does not use `pull_request_target` and uses least-privilege permissions.
- Security tests verify workflow hardening controls.
- Pre-commit hooks are configured to block secret-bearing paths and scan staged diffs for common credential patterns.

### Enable pre-commit locally

```bash
python3 -m pip install pre-commit
pre-commit install
pre-commit run --all-files
```
