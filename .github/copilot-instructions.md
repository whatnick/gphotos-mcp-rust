# Copilot instructions for `gphotos-mcp-rust`

## Project goals
- Maintain a secure, production-minded Rust MCP server for Google Photos.
- Keep OAuth handling strict: explicit errors, no silent fallbacks, and no token logging.
- Prefer minimal, well-tested changes over broad refactors.

## Engineering standards
- Use stable Rust idioms and keep code `cargo fmt` + `cargo clippy -- -D warnings` clean.
- Preserve strong typing and avoid `unwrap()` in runtime paths.
- Reuse existing modules (`auth`, `mcp`, `photos`, `config`) before adding new abstractions.
- Keep external API interactions explicit, with clear request/response mapping and surfaced failures.

## Testing expectations
- For behavior changes, update or add targeted tests under `tests/`.
- Maintain and extend security-oriented tests, especially workflow hardening assertions.
- Prefer deterministic tests with mocks (`wiremock`) over network-dependent tests.

## Security expectations
- Do not add `pull_request_target` workflows.
- Keep GitHub Actions permissions least-privilege and `persist-credentials: false` on checkout steps.
- Do not print OAuth tokens, refresh tokens, or secrets in logs.
- Keep token storage local and permission-restricted.
