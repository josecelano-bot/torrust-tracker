#!/usr/bin/env bash
# Usage: reproduce.sh <tracker-binary> <work-dir>. Reproduces the client error texts of SI-B (issue #3000):
# scenarios M1 to M5 of the specification on a sqlite tracker, plus the database failure of M4 by
# dropping the `torrents` table of a running tracker. Test values only. Uses the UDP probe of the EPIC #2993 evidence.
set -u
BIN=$1; W=$2; mkdir -p "$W"; cd "$W" || exit 1
PROBE="$(dirname "$0")/../../2993-secrets-and-client-error-text/evidence/udp_probe.py"
strip(){ sed 's/\x1b\[[0-9;]*m//g'; }
hdr='[metadata]
app = "torrust-tracker"
purpose = "configuration"
schema_version = "3.0.0"
[logging]
trace_filter = "debug"
trace_style = "full"
'
start(){ ("$BIN" --config-toml-path "$1" > "$2" 2>&1 & echo $! > "$2.pid"); for _ in $(seq 1 30); do curl -s -o /dev/null "http://127.0.0.1:$3/api/health_check" && return; sleep 1; done; }
stop(){ kill "$(cat "$1.pid")" 2>/dev/null; sleep 1; }
H='Authorization: Bearer TestAdminToken0001'
IH='info_hash=%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00&peer_id=-qB00000000000000001&port=17548&left=0'
IH2='info_hash=%01%01%01%01%01%01%01%01%01%01%01%01%01%01%01%01%01%01%01%01&peer_id=-qB00000000000000001&port=17548&left=0'
common='[http_api]\nbind_address = "127.0.0.1:41212"\n[http_api.access_tokens]\nadmin = "TestAdminToken0001"\n[[udp_trackers]]\nbind_address = "127.0.0.1:46969"\n[[http_trackers]]\nbind_address = "127.0.0.1:47070"\n[health_check_api]\nbind_address = "127.0.0.1:41313"\n'
echo "### M1, M5 private tracker, sqlite: REST"
printf '%s[core]\nlisted = false\nprivate = true\n[core.database]\ndriver = "sqlite3"\npath = "%s/private.db"\n%b' "$hdr" "$W" "$common" | sed '/udp_trackers/,+1d' > private.toml
rm -f private.db*; start private.toml private.log 41212
K='{"key":"TestPeerKey000000000000000000002","seconds_valid":100}'
echo "--- M1 REST duplicate key"; for _ in 1 2; do curl -s -X POST -H "$H" -H 'content-type: application/json' -d "$K" -w ' [%{http_code}]\n' http://127.0.0.1:41212/api/v1/keys; done
echo "--- M5 REST no token"; curl -s -w ' [%{http_code}]\n' http://127.0.0.1:41212/api/v1/stats
echo "--- M5 REST wrong token"; curl -s -w ' [%{http_code}]\n' -H 'Authorization: Bearer WrongTestToken0001' http://127.0.0.1:41212/api/v1/stats
echo "--- M5 REST malformed Authorization header"; curl -s -w ' [%{http_code}]\n' -H 'Authorization: Basic abc' http://127.0.0.1:41212/api/v1/stats
echo "--- M5 REST malformed JSON"; curl -s -X POST -H "$H" -H 'content-type: application/json' -d '{bad' -w ' [%{http_code}]\n' http://127.0.0.1:41212/api/v1/keys
echo "--- M5 REST JSON of the wrong type"; curl -s -X POST -H "$H" -H 'content-type: application/json' -d '{"key":1}' -w ' [%{http_code}]\n' http://127.0.0.1:41212/api/v1/keys
echo "--- M5 REST no content type"; curl -s -X POST -H "$H" -d "$K" -w ' [%{http_code}]\n' http://127.0.0.1:41212/api/v1/keys
echo "--- M5 REST bad query parameter"; curl -s -H "$H" -w ' [%{http_code}]\n' 'http://127.0.0.1:41212/api/v1/torrents?offset=abc'
echo "--- M5 REST key duration overflow"; curl -s -X POST -H "$H" -H 'content-type: application/json' -d '{"key":null,"seconds_valid":18446744073709551615}' -w ' [%{http_code}]\n' http://127.0.0.1:41212/api/v1/keys
echo "--- HTTP announce, no key"; curl -s "http://127.0.0.1:47070/announce?$IH"; echo
stop private.log
echo "--- log: lines for the 500 replies (status and request id, no reason today)"; strip < private.log | grep -E 'status_code=500' | head -2 | cut -c1-260

echo; echo "### M2, M3 public listed tracker: HTTP and UDP"
printf '%s[core]\nlisted = true\nprivate = false\n[core.database]\ndriver = "sqlite3"\npath = "%s/public.db"\n%b' "$hdr" "$W" "$common" > public.toml
rm -f public.db*; start public.toml public.log 41212
echo "--- M2 HTTP announce, not whitelisted"; curl -s "http://127.0.0.1:47070/announce?$IH"; echo
echo "--- HTTP announce, missing param"; curl -s "http://127.0.0.1:47070/announce?peer_id=-qB00000000000000001"; echo
echo "--- HTTP announce, bad info_hash"; curl -s "http://127.0.0.1:47070/announce?info_hash=abc&peer_id=-qB00000000000000001&port=1&left=0"; echo
python3 -I "$PROBE" 46969
echo "--- M6 health check, all services up"; curl -s -w ' [%{http_code}]\n' http://127.0.0.1:41313/health_check | cut -c1-300
stop public.log

echo; echo "### M4 database failure: the torrents table of a running tracker is dropped"
printf '%s[core]\nlisted = false\nprivate = false\n[core.tracker_policy]\npersistent_torrent_completed_stat = true\n[core.database]\ndriver = "sqlite3"\npath = "%s/broken.db"\n%b' "$hdr" "$W" "$common" > broken.toml
rm -f broken.db*; start broken.toml broken.log 41212
sqlite3 broken.db 'DROP TABLE torrents;' 2>&1 | head -2
echo "--- M4 HTTP announce, info-hash not in memory"; curl -s "http://127.0.0.1:47070/announce?$IH2"; echo
echo "--- M4 HTTP scrape"; curl -s "http://127.0.0.1:47070/scrape?$(echo "$IH2" | cut -d'&' -f1)" | head -c 300; echo
echo "--- M4 UDP announce"; python3 -I "$PROBE" 46969 | sed -n '1,2p'
echo "--- log: ERROR lines"; strip < broken.log | grep -E ' ERROR ' | head -3 | cut -c1-300
stop broken.log
