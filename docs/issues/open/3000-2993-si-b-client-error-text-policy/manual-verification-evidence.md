---
doc-type: manual-verification-evidence
issue-spec: docs/issues/open/3000-2993-si-b-client-error-text-policy/ISSUE.md
last-updated-utc: "2026-11-26 17:40"
---

# Manual Verification Evidence

<!-- cspell:ignore torrents -->

## Purpose

Record what a running tracker sends to clients before and after SI-B (issue #3000) changes the
client error text, as the `fix-bug` skill requires: the baseline first, the same requests again
after the last production change.

## Preserving Verification Artifacts

- [`evidence/reproduce.sh`](evidence/reproduce.sh): the script that ran every request below. It
  writes the tracker configurations it uses. It reuses the UDP probe of the EPIC #2993 evidence
  (`../2993-secrets-and-client-error-text/evidence/udp_probe.py`).
- [`evidence/baseline-output.txt`](evidence/baseline-output.txt): the complete output of the
  baseline run. The scratch work directory is shown as `<work-dir>`, and a first line with a spell-checker directive was added.

All keys, tokens and passwords are test values made up for the run. The script is disposable; the
durable checks are the Rust tests of the specification. The implementer removes it at close-out.

## Environment and Prerequisites

- Artifact under test (baseline): `target/debug/torrust-tracker`, built with
  `cargo build --bin torrust-tracker` from `develop` at the merge of pull request 3063
  (`docs(adrs): [#3000] add the client error text policy ADR`), which changes documentation only,
  so the code equals `develop` before SI-B.
- Toolchain and system: the stable toolchain of the checkout, `sqlite3`, `curl`, `python3 -I`,
  Linux 7.0.0.
- Date and time (UTC) of the baseline run: 2026-11-26 17:34.
- Command: `bash evidence/reproduce.sh <path to target/debug/torrust-tracker> <empty work dir>`.

Not run in the baseline: the failed-probe and shutting-down cases of M6 (nothing safe can make a
registered service fail on a running tracker without code changes; the existing health-check
contract tests cover them, as the Regression Test Strategy says), and a MySQL or PostgreSQL
database (M4 is run on SQLite, by dropping the `torrents` table of a running tracker, which makes
the persisted download count read fail through a real database error).

## Verification Processes

### V0 Baseline: client texts before SI-B

Each reply is in [`evidence/baseline-output.txt`](evidence/baseline-output.txt). Summary of the
texts that the ADR changes:

| Scenario | Baseline reply |
| --- | --- |
| M1 REST duplicate key | 500 `Unhandled rejection: Err { reason: "failed to add key: The Sqlite3 database already holds this authentication key" }` |
| M5 REST no token / wrong token / malformed header | 500 `Unhandled rejection: Err { reason: "unauthorized" }`, `"token not valid"`, `"unknown token provided"` |
| M5 REST malformed JSON | 400 `Failed to parse the request body as JSON: key must be a string at line 1 column 2` |
| M5 REST JSON of the wrong type | 422 `Failed to deserialize the JSON body into the target type: key: invalid type: integer ...` |
| M5 REST no content type | 415 ``Expected request with `Content-Type: application/json` `` |
| M5 REST bad query | 400 `Failed to deserialize query string: offset: invalid digit found in string` |
| M2 HTTP not whitelisted | `Tracker whitelist error: The torrent: <ih>, is not whitelisted` |
| HTTP announce parse | `Bad request. Cannot parse query params for announce request: missing param info_hash` |
| M2 UDP not whitelisted | `tracker announce error: Tracker core whitelist error: The torrent: <ih>, is not whitelisted` |
| M3 UDP wrong connection id | `tracker announce error: Connection cookie error: cookie value is expired: <decoded cookie>, expected > <server clock>` |
| M3 UDP 3-byte packet | `error parsing request: SendableRequestParseError { message: "Couldn't parse action", opt_connection_id: None, opt_transaction_id: None }` |
| M4 HTTP announce and scrape, database failure | `Tracker announce error: Database error: The Sqlite3 query was malformed: error returned from database: (code: 1) no such table: torrents` (and `Tracker scrape error: ...`) |
| M4 UDP announce, database failure | `tracker announce error: Tracker core announce error: Database error: The Sqlite3 query was malformed: ...` |
| M1 server log of the 500 | `ERROR ... status_code=500 ... request_id=<id>`: no reason |

The wrong `InvalidQuery` label shows in the M4 texts: a missing table is not a malformed query.
