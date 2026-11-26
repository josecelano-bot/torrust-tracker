---
name: handle-errors-in-code
description: "Write Rust error types: thiserror enums over anyhow, with context and actionable messages, and no source locations. Use when adding error types or handling a Result."
metadata:
  author: torrust
  version: "1.1"
---

# Handling Errors in Code

## Core Principles

1. **Clarity** — Users immediately understand what went wrong
2. **Context** — Include what/where/when/why
3. **Actionability** — Tell users how to fix it
4. **Explicit enums over `anyhow`** — Prefer structured errors for pattern matching

## Prefer Explicit Enum Errors

```rust
// ✅ Correct: explicit, matchable, clear
#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("Torrent '{info_hash}' not found in whitelist")]
    TorrentNotWhitelisted { info_hash: InfoHash },

    #[error("Peer limit exceeded for torrent '{info_hash}': max {limit}")]
    PeerLimitExceeded { info_hash: InfoHash, limit: usize },
}

// ❌ Wrong: opaque, hard to match
return Err(anyhow::anyhow!("Something went wrong"));
return Err("Invalid input".into());
```

## Include Actionable Fix Instructions in Display

When the error is user-facing, add instructions:

```rust
#[error(
    "Configuration file not found at '{path}'.\n\
     Copy the default: cp share/default/config/tracker.toml {path}"
)]
ConfigNotFound { path: PathBuf },
```

## Context Requirements

Each error should answer:

- **What**: What operation was being performed?
- **Where**: Which component, file, or resource? (A runtime resource such as a config file or an address, not a source location.)
- **When**: Under what conditions?
- **Why**: What caused the failure?

```rust
// ✅ Good: full context
#[error("UDP socket bind failed for '{addr}': {source}. Is port {port} already in use?")]
SocketBindFailed { addr: SocketAddr, port: u16, source: std::io::Error },

// ❌ Bad: no context
return Err("bind failed".into());
```

## No Source Locations in Errors

Errors must not record or print source locations. See
[ADR 20261118045443](../../../docs/adrs/20261118045443_errors_do_not_record_source_locations.md).

- No `location` field and no `Location::caller()` in error types.
- No `torrust-located-error`: keep a source error as a plain `#[source]` (or `#[from]`) field.
- `Display` states what failed and the offending input, never a source path. A secret (an
  authentication key, a token, a password) is never echoed or kept in the error: name the parameter
  and the expected format instead (see the `handle-secrets` skill).
- Text sent to a client is chosen by the protocol layer from the class and the variant of the
  error, not from its `Display` chain; see "Text Sent to a Client" below. Never format a source error
  from a crate the workspace does not control into client text.
- For developer diagnostics, emit a `tracing` event at the site; it records file and line on the
  server only.
- `#[track_caller]` is allowed only in test helpers and assertion helpers.

```rust
// ✅ Correct: names what failed and the expected format, and does not keep the secret
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("invalid authentication key: expected 32 alphanumeric characters, got {length}")]
    InvalidKeyFormat { length: usize },
}
```

```rust
// ❌ Wrong: a location that reaches clients and logs
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("invalid key format, {location}")]
    InvalidKeyFormat { location: &'static Location<'static> },
}
```

Existing `location` fields and `LocatedError` uses are being removed by EPIC #2912; do not add new
ones.

## Text Sent to a Client

What the HTTP tracker, the UDP tracker, the REST API and the health-check API tell a client when a
request fails is decided by
[ADR 20261125175726](../../../docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md),
which amends rule 4 of the ADR above. Its tables list every current and proposed text; this is the
rule to follow when you add or change a reply:

- **Classify the failure first.** A *bad request* (the client's own input) says what is wrong and
  what is expected. A *refusal* (not whitelisted, key missing or expired, wrong token) says what was
  refused and what the client can do. An *internal failure* (a database that is down, a state the code
  did not expect) is the tracker's fault: one fixed sentence that names the operation and the request
  id, for example `Internal tracker error: the announce could not be completed, try again later
  (request id: <id>)`, and nothing from the error or its source.
- **A cause the code can tell and the client caused is not an internal failure**, even when a lower
  layer reports it as an error (a duplicate authentication key says so).
- **One sentence, not the Rust error tree.** The protocol layer builds the text from the variant
  (`error_mapping.rs` for the HTTP tracker, `Error::client_message` for UDP, the response functions
  of the REST API, the probes of the health-check API). Never forward `Display`, `Debug`, a type name,
  a database driver text, a library text (`serde`, `axum`, `reqwest`) or a server timing value.
- **Echo a client value only within the limits:** at most 64 characters as sent, then `...`, escaped
  with `char::escape_debug` after the cut, and never a secret (use `echo` of `http-protocol` or of the
  REST API server).
- **The detail goes to the server.** Every internal failure emits one `ERROR` event with the full
  error (its source included) and the request id, inside the request span (the span declares
  `request_id`; `torrust_tracker_axum_server::request_id::current_request_id` reads it below the
  handlers). A secret is never in that event either (the `handle-secrets` skill).
- **No reply text is a contract.** Clients rely on the signal each protocol defines (`failure
  reason`, UDP action 3, the status code); tests pin texts to keep the policy, not to promise them.
- **Event payloads follow the rules of a log**, not of a reply: they may carry the detail, never a
  secret, and state what happened whatever the client was told.
- **Statuses of REST API v1 stay**; a status that is wrong in v1 is corrected by the v2 error model
  (EPIC #144), which follows the same three classes.

A test for a client text has the shape the existing ones show: build the cause with a unique marker
string in its source, assert the reply does not contain the marker, and for an internal failure also
that the log holds the detail and the same request id.

## Unwrap and Expect Policy

| Context                | `.unwrap()` | `.expect("msg")`                          | `?` / `Result` |
| ---------------------- | ----------- | ----------------------------------------- | -------------- |
| Production code        | Never       | Only when failure is logically impossible | Default        |
| Tests and doc examples | Acceptable  | Preferred when message adds clarity       | —              |

```rust
// ✅ Production: propagate errors with ?
fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::FileAccess { path: path.to_path_buf(), source: e })?;
    toml::from_str(&content)
        .map_err(|e| ConfigError::InvalidToml { path: path.to_path_buf(), source: e })
}

// ✅ Tests: unwrap() is fine
#[test]
fn it_should_parse_valid_config() {
    let config = Config::parse(VALID_TOML).unwrap();
    assert_eq!(config.http_api.bind_address, "127.0.0.1:1212");
}
```

## When Public APIs Return `Result`

Return `Result` only when an operation can fail today, crosses an I/O boundary, or is a trait or port
with an existing or planned backend that can fail. Otherwise return a plain value; a real failure later is a
semver-signalled breaking change. Never use `Infallible` or an empty error enum as a placeholder, and mark
existing public error enums `#[non_exhaustive]` before first publish, except an enum whose variants select
different recovery actions (stop or continue, reply or drop): keep it exhaustive and say why in a doc comment, so
that a new variant is a deliberate breaking change. See [ADR 20261005145329](../../../docs/adrs/20261005145329_return_result_only_for_concretely_fallible_public_apis.md).

## Quick Checklist

- [ ] Error type uses `thiserror::Error` derive
- [ ] Error message includes specific context (names, paths, addresses, values), but no secrets
- [ ] Error message includes fix instructions where possible
- [ ] Prefer `enum` over `Box<dyn Error>` or `anyhow` in library code
- [ ] No vague messages like "invalid input" or "error occurred"
- [ ] No `.unwrap()` in production code (tests and doc examples are fine)
- [ ] No `location` field, no `Location::caller()` and no `torrust-located-error` (see the ADR above)
- [ ] A reply to a client holds no driver text, library text, type name or server clock; an internal
      failure names the operation and the request id, and its detail is in an `ERROR` event
