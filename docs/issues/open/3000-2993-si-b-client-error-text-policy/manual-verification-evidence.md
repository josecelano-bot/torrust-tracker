---
doc-type: manual-verification-evidence
issue-spec: docs/issues/open/3000-2993-si-b-client-error-text-policy/ISSUE.md
last-updated-utc: "2026-11-26 17:40"
---

# Manual Verification Evidence

<!-- cspell:ignore torrents latere -->

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

## Test Increments

Each increment records the red and green runs of its first tests and the test-design review
(prose-first Arrange-Act-Assert comparison) that the specification requires before the next area.
Commands ran with the stable toolchain (`cargo +stable test -q`).

### T4 Database label, request id in the request span, request id scope

- **Red 1:**
  `cargo +stable test -q -p torrust-tracker-core --lib databases::error` failed with
  `left: "The MySQL query was malformed: error returned from database: Duplicate entry ..."`,
  `right: "The MySQL database server returned an error: Duplicate entry ..."` for
  `it_should_say_that_the_database_server_returned_an_error_when_the_server_rejects_a_statement`.
  Green after adding the `ServerError` variant (23 passed). The older tests that read an error
  raised by the server as `InvalidQuery` now accept `ServerError` for the missing-table cases; the
  decode cases stay `InvalidQuery`.
- **Red 2:**
  `cargo +stable test -q -p torrust-tracker-axum-server --lib request_log` failed for
  `it_should_let_an_event_inside_the_request_span_carry_the_request_id_that_is_recorded_later`
  (the logged span was `request{method=GET uri=/health_check version=HTTP/1.1}` with no id).
  Green after declaring `request_id = tracing::field::Empty` in `RedactedMakeSpan` (39 passed).
- **Red 3:**
  `request_id::tests::it_should_make_the_id_of_the_request_header_available_to_the_handler` failed
  (`left: "unknown"`, `right: "an-id-from-the-header"`) while the middleware did not scope the id;
  green once it did (5 passed).
- **Test-design review.** Test 1 changes one thing, the sqlx error the conversion receives (a
  server-raised error instead of a connection error), and asserts the whole message, whose
  expected text is written from the ADR and not computed by the code. Test 2 holds the span
  construction (the production Act) and one recorded value; the fixture owns only the request
  builder. Test 3 uses a real router with the production middleware, so the Act is the request
  and the expected text is the header the test sent. The helper `serve` owns the incidental body
  reading.

### T5 HTTP tracker text

- **Red:** `cargo +stable test -q -p torrust-tracker-axum-http-server --test database_failure`
  failed with `the client should read the operation and its request id, got:
  d14:failure reason76:Internal tracker error: the announce could not be completed, try again latere`
  (the sentence was already fixed by the new mapping, the request id and the `ERROR` event were
  missing). Green after `failure_response` in `handlers/mod.rs` added the id and logged the event:
  announce and scrape both pass (2 passed).
- **Red by mutation for the mapping:** with the announce database arm of `error_mapping` changed to
  print the driver text, `it_should_treat_a_database_failure_as_an_internal_failure_that_names_the_operation_only`
  failed (`case_1_announce`); the file was restored and the unit tests pass (200 in `http-core`).
  The `echo` helper and the `Error::internal`, `is_internal` and `with_request_id` helpers were
  written with their tests in one step (their symbols did not exist before, so the tests did not
  compile without them).
- **Existing tests that pinned the old texts** turned red with the change and were updated to the
  ADR texts: the `Bad request. Cannot parse query params...` prefixes became `Invalid announce
  request`, `Invalid scrape request` and `Invalid request`, the `Tracker ... error:` prefixes went,
  the whitelist sentence is `The torrent <ih> is not whitelisted on this tracker`, and the missing
  client address is an internal failure. `invalid param a=b=c` became `malformed query param a=b=c`.
- **Test-design review.** The real-boundary test changes one thing in the initial state, a database
  whose table was dropped after start, and states the whole expected sentence from the ADR with the
  request id read from the reply's own header, so the Act is the production request and the
  expectation is independent of the code. The fixture helper owns the mechanics (starting the
  tracker, dropping the table, reading the header). The mapping unit tests build the cause with a
  unique marker string in its source and assert that the marker is absent from the reply, which is
  the property the rule states.

### T6 UDP tracker text and the error event

- **Red:** `cargo +stable test -q -p torrust-tracker-udp-server --lib error::tests::what_the_client`
  did not compile (`no method named client_message found for enum error::Error`); green after
  `Error::client_message` and its use in `handlers/error.rs` (239 passed in the library). Two older
  tests that pinned the reply `tracker scrape error: Connection cookie error: cookie value is
  expired: 1, expected > 2` were updated to the ADR text.
- **Red by mutation for the real boundary:**
  `cargo +stable test -q -p torrust-tracker-udp-server --test database_failure` with the reply
  taken from `error.to_string()` again failed with `got: tracker announce error: Tracker core
  announce error: Database error: The Sqlite3 database server returned an error: (code: 1) no such
  table: torrents`; restored, it passes: the client reads the operation and the request id, and the
  log line holds the driver text and the same id.
- **Integration tests added or updated** (`--test integration`, 19 passed): a wrong connection id
  gets `Invalid or expired connection id: send a new connect request`; a 3-byte packet gets
  `Invalid request: Couldn't parse action`; the empty datagram gets `Invalid request: Protocol
  identifier missing`; the whitelist refusal is `The torrent <ih> is not whitelisted on this tracker`.
- **Event payload:** unchanged (`ErrorKind` keeps the detail, as rule 8 says); `SendableRequestParseError`
  gained `reply_message` for the reply and keeps `message` for the event, so the short-packet event
  text is as before.
- **Test-design review.** The unit table builds each error from its cause and states the whole
  expected sentence from the ADR; the cookie cases differ only in the cookie variant, so the one
  causal difference is visible and the three variants share one expected text. The real-boundary
  test differs from a healthy tracker only by the dropped table, reads the request id from the reply
  text and looks for that same id in the log, so it does not restate how the id is made. The helper
  owns the connect handshake and the announce builder.

### T7 REST API text

- **Red by mutation for the real boundary:**
  `cargo +stable test -q -p torrust-tracker-axum-rest-api-server --test database_failure` with
  `failed_to_add_key_response` printing the error again failed with `left: "Unhandled rejection: Err
  { reason: \"failed to add key: The Sqlite3 database server returned an error: the database
  rejected the statement (kind: Other, code: 1)\" }"`, `right: "... failed to add key: internal
  tracker error (request id: <id>)"`; restored, both tests pass (2 passed). The log line of that
  failure holds `the database rejected the statement` (for a statement that binds a key SI-A keeps
  only the kind and the code, never the server's message) and the same request id.
- **Existing tests that pinned the old texts** turned red with the change and were updated to the ADR
  texts: the three token reasons (`unauthorized: send the header Authorization: Bearer <token>`,
  `unauthorized: the token is not valid`, `unauthorized: the Authorization header must be Bearer
  <token>`), the `failed to <operation>: internal tracker error (request id: <id>)` reasons, the
  key duration overflow (`... 18446744073709551615 seconds is too long`) and the two query
  parameter rejections of the torrents endpoint.
- **New tests:** `rejections` (a real router with the production extractor wrapper: malformed JSON,
  a wrong type, a missing field, a missing content type, a bad query parameter), the adapter test that
  a key the tracker already holds is `AlreadyExists` and any other database failure stays a database
  error, the `echo` cut and escape, and the internal failure reply with a scoped request id.
- **Decision recorded in the ADR:** a duplicate key keeps its 500 in v1, with the reason `failed to
  add key: the tracker already holds this authentication key`; the existing contract test
  `should_fail_adding_a_key_that_already_exists_without_echoing_it` passes unchanged.
- **Test-design review.** The database test changes one thing in the initial state (a dropped
  table) and states the whole expected reply with the request id read from the reply's header, so
  the production request is the Act and the expectation does not restate how the id is made. The
  rejection table builds each request from one defect of the body or the query and states the
  expected sentence from the ADR; the fixture helper owns only the router and the body reading.
  The `WithRejection` wrapper is exercised through a real router, not by calling the conversion.

### T7b Health-check API text

- **Red:** `cargo +stable test -q -p torrust-tracker-client-lib --lib check_should` failed with
  `left: "Unrecognized UDP tracker response. Expected a valid UDP response, got: [106, 117, 110, 107]"`,
  `right: "the service answered with an invalid response"` once the junk-peer test asked for the
  ADR text; green after the probe returned the fixed sentences and logged the client error. The
  health-check contract tests that pinned the old results (`error sending request`, `Timed Out`,
  `Unrecognized UDP tracker response`) turned red with the change (4 failures) and were updated to
  the ADR sentences; the panicked-job test now asserts the whole result
  `Err("the health check job failed")`, so no join error text can pass.
- **Green:** `cargo +stable test -q -p torrust-tracker-axum-health-check-api-server` (13 passed
  integration, 13 unit, request logging 1); the shutting-down `503` test passes unchanged and the
  failed-probe report keeps its `200` status.
- **Test-design review.** Each contract case differs from a healthy report by one fault of one
  service (an unreachable port, a peer that answers junk, a peer that never answers, a job that
  panics) and states the whole expected sentence from the ADR; the unit test of the UDP probe uses a
  real socket peer, not a mock of the client. The helpers own the registration of the service and the
  report decoding only.

The full workspace run (`cargo +stable test --no-fail-fast --tests --benches --examples --workspace
--all-targets --all-features`, the pre-push roster) found three more unit tests of the HTTP and REST
servers that pinned the old probe results (`500 Internal Server Error` and the checked URL in the
reply). They were updated to `the service answered 500 Internal Server Error` and `the service did
not answer`; the run then passed (110 test binaries ok, exit 0).
