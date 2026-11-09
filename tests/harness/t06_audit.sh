#!/usr/bin/env bash
# t06: scripts/audit-session.sh.
#  (a) against the previous session ses_eafd6e97fffeXSloTmS42jhMt4, read-only: the numbers must equal the previous measurement (R3 section B.10; the model ids contain the
#      vendor word, so the test replaces it with <V> before comparing, and the expected file is stored that way). That session is closed; its rows no longer change.
#  (b) against a small synthetic database (HOME is redirected to a sandbox): off-pin detection, errors, and "never writes".
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

V="anthr""opic"
prev=ses_eafd6e97fffeXSloTmS42jhMt4
real_db="$HOME/.local/share/opencode/opencode.db"

# ---------------- (a) the previous session
if [ ! -r "$real_db" ] || [ "$(sqlite3 -readonly "$real_db" "select count(*) from session_v2 where id='$prev';" 2> /dev/null)" != 1 ]; then
  skip "(a) the previous session is not in this machine's database: cannot compare with the previous measurement"
else
  out=$(bash "$ROOT/scripts/audit-session.sh" "$prev" 2> "${TMPDIR:-/tmp}/t06.err"); rc=$?
  expect_eq "audit-session.sh exits 0 on the previous session" 0 "$rc"
  norm=$(printf '%s\n' "$out" | sed "s/$V/<V>/g")
  # numbers measured before (R3 B.10 and 02-what-went-wrong V-F01 / V-F04 / R3 A.4)
  val() { awk -v s="$1" '$0 ~ s {on=1; next} on && /^[-]+/ {dash=1; next} on && dash && NF {print; exit}' <<< "$norm"; }
  expect_eq "main model-switched count (R3: 20)" "20" "$(val '^## 1\. main' | awk '{print $1}')"
  expect_eq "children: model-switched rows / sessions (R3: 30 rows, 17 sessions)" "30 17" "$(val '^## 1c' | awk '{print $1" "$2}')"
  expect_eq "children count in the header (R3: 465)" "465" "$(sed -n '1s/.*children: \([0-9]*\)).*/\1/p' <<< "$norm")"
  expect_eq "main direct work edit+write+patch (V-F01: 174)" "174" "$(val '^## 3b' | awk '{print $1}')"
  expect_eq "gaps >= 60 min: count and hours (R3 A.4: 72, 211.0 h)" "72 211.0" "$(val '^## 4\. gaps' | awk '{print $1" "$2}')"
  expect_eq "subagent calls / with a model argument (R3: 572 / 0)" "572 0" "$(val '^## 6\.' | awk '{print $1" "$2}')"
  while read -r kind model want; do
    row=$(grep -E "^$kind +$model +[0-9]+ *$" <<< "$norm" | head -1)
    expect_eq "assistant rows $kind $model (R3 A.2 / B.10)" "$want" "$(awk '{print $NF}' <<< "$row")"
  done << 'ROWS'
main console-aws-bedrock/global.<V>.claude-sonnet-5-5 5914
main console-openrouter/<V>/claude-sonnet-5.5 2907
main console-cloudflare-ai-gateway-custom/claude-sonnet-5-5 2204
main console-cloudflare-ai-gateway-custom/claude-haiku-5-5 848
child console-openrouter/<V>/claude-sonnet-5.5 53601
child console-aws-bedrock/global.<V>.claude-sonnet-5-5 27828
child console-cloudflare-ai-gateway-custom/claude-sonnet-5-5 20022
child console-cloudflare-ai-gateway-custom/claude-haiku-5-5 2682
ROWS
  expect_contains "section 3 lists the main session's shell calls (8709 completed + 25 errors)" "$norm" "shell  completed  8709"
  expect_contains "section 3 lists edit 115+1 and write 57+1" "$norm" "edit   completed  115"
  # the whole output equals the stored one (a byte-for-byte comparison of the previous measurement)
  exp="$HERE/expected/audit-prev-session.txt"
  if [ -r "$exp" ]; then
    if diff <(printf '%s\n' "$norm") "$exp" > "${TMPDIR:-/tmp}/t06.diff"; then pass "the full output equals tests/harness/expected/audit-prev-session.txt"
    else fail "the full output differs from the stored previous measurement: $(head -4 "${TMPDIR:-/tmp}/t06.diff" | tr '\n' '|' | cut -c1-300)"; fi
  else fail "tests/harness/expected/audit-prev-session.txt is missing"; fi
  expect_eq "nothing was written to stderr" "0" "$(wc -c < "${TMPDIR:-/tmp}/t06.err" | tr -d ' ')"
  rm -f "${TMPDIR:-/tmp}/t06.err" "${TMPDIR:-/tmp}/t06.diff"
fi

# ---------------- (b) synthetic database
sb=$(new_sandbox t06)
home="$sb/home"; mkdir -p "$home/.local/share/opencode"
db="$home/.local/share/opencode/opencode.db"
python3 - "$db" << 'PY'
import json, sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.executescript("""
create table session_v2 (id text primary key, parent_id text, cost real default 0);
create table session_message (id text primary key, session_id text not null, type text not null, seq integer not null, time_created integer not null, time_updated integer not null, data text not null);
""")
SON = {"providerID": "console-cloudflare-ai-gateway-custom", "id": "claude-sonnet-5-5"}
HAI = {"providerID": "console-cloudflare-ai-gateway-custom", "id": "claude-haiku-5-5"}
OTHER = {"providerID": "someone-else", "id": "big-model"}
T0 = 1_800_000_000_000
n = [0]
def msg(sid, typ, t, data):
    n[0] += 1
    db.execute("insert into session_message values (?,?,?,?,?,?,?)", ("msg_%d" % n[0], sid, typ, n[0], t, t, json.dumps(data)))
def asst(sid, t, model, tools=()):
    msg(sid, "assistant", t, {"model": model, "content": [{"type": "tool", "name": nm, "state": {"status": "completed", "input": inp}} for nm, inp in tools]})
db.execute("insert into session_v2 values ('ses_MAIN',null,0)")
db.execute("insert into session_v2 values ('ses_KID1','ses_MAIN',0)")
db.execute("insert into session_v2 values ('ses_KID2','ses_MAIN',0)")
db.execute("insert into session_v2 values ('ses_OTHER',null,0)")
asst("ses_MAIN", T0, SON, [("read", {"path": "x"})])
asst("ses_MAIN", T0 + 60_000, SON, [("subagent", {"agent": "worker", "prompt": "p"}), ("subagent", {"agent": "worker", "prompt": "q", "model": "x/y"})])
asst("ses_MAIN", T0 + 120_000, HAI, [("edit", {"path": "a"}), ("write", {"path": "b"}), ("shell", {"command": "env sleep 1"}), ("shell", {"command": "opencode auth export"})])
msg("ses_MAIN", "model-switched", T0 + 110_000, {"model": {**HAI, "variant": "high"}, "previous": {**SON, "variant": "xhigh"}})
asst("ses_MAIN", T0 + 120_000 + 2 * 3600_000, SON)    # a 2-hour gap before this row
asst("ses_KID1", T0 + 1000, HAI, [("shell", {"command": "git push --force origin x"})])
asst("ses_KID1", T0 + 2000, HAI)
asst("ses_KID2", T0 + 3000, SON)                       # a child on the wrong model
msg("ses_KID2", "model-switched", T0 + 2500, {"model": SON, "previous": HAI})
asst("ses_OTHER", T0, OTHER, [("edit", {"path": "z"})])
db.commit()
PY
before=$(md5sum "$db" | cut -d' ' -f1)
run() { HOME="$home" bash "$ROOT/scripts/audit-session.sh" "$@" 2>&1; }
out=$(run ses_MAIN); rc=$?
expect_eq "synthetic: exit 0" 0 "$rc"
expect_contains "synthetic: header counts the 2 children" "$out" "(children: 2)"
expect_contains "synthetic: pins come from opencode.jsonc" "$out" "main=console-cloudflare-ai-gateway-custom/claude-sonnet-5-5  children=console-cloudflare-ai-gateway-custom/claude-haiku-5-5"
sec() { awk -v s="$1" 'index($0, s) == 1 {on=1; next} on && /^## / {exit} on {print}' <<< "$out"; }
expect_eq "synthetic: model-switched count 1" "1" "$(sec '## 1. main' | awk 'NR==3 {print $1}')"
expect_contains "synthetic: the switch is listed from -> to" "$(sec '## 1b')" "claude-sonnet-5-5#xhigh"
expect_eq "synthetic: child switch rows 1 / children 1" "1 1" "$(sec '## 1c' | awk 'NR==3 {print $1" "$2}')"
expect_eq "synthetic: direct work (edit+write) is 2" "2" "$(sec '## 3b' | awk 'NR==3 {print $1}')"
expect_contains "synthetic: shell calls are counted" "$(sec '## 3.')" "shell  completed  2"
expect_eq "synthetic: one gap of 60+ minutes, 2.0 hours" "1 2.0" "$(sec '## 4. gaps' | awk 'NR==3 {print $1" "$2}')"
off=$(sec '## 5.')
expect_contains "synthetic: the child on Sonnet is reported off the pin" "$off" "child  console-cloudflare-ai-gateway-custom/claude-sonnet-5-5"
expect_contains "synthetic: the main row on Haiku is reported off the pin" "$off" "main   console-cloudflare-ai-gateway-custom/claude-haiku-5-5"
expect_not_contains "synthetic: a row ON the pin is not reported (child Haiku)" "$off" "child  console-cloudflare-ai-gateway-custom/claude-haiku-5-5"
expect_not_contains "synthetic: another main session is not mixed in" "$out" "someone-else/big-model"
expect_eq "synthetic: 2 subagent calls, 1 with a model argument" "2 1" "$(sec '## 6.' | awk 'NR==3 {print $1" "$2}')"
risky=$(sec '## 7.')
expect_contains "synthetic: risky commands: main has 1 opencode auth and 1 sleep bypass" "$risky" "main"
expect_eq "synthetic: main opencode_auth=1 sleep_bypass=1" "1 1" "$(awk '$1=="main" {print $3" "$7}' <<< "$risky")"
expect_eq "synthetic: child force_push=1" "1" "$(awk '$1=="child" {print $6}' <<< "$risky")"
expect_eq "the database file is byte-identical after the audit (read-only)" "$before" "$(md5sum "$db" | cut -d' ' -f1)"
expect_eq "no journal/wal file was left next to the database" "1" "$(find "$home/.local/share/opencode" -type f | wc -l | tr -d ' ')"
check "every sqlite3 call in the script is -readonly" bash -c "! grep -n 'sqlite3' '$ROOT/scripts/audit-session.sh' | grep -v -- '-readonly' | grep -v '^[0-9]*:#' | grep -v 'command -v'"
check "the script never copies the database" bash -c "! grep -nE '\bcp\b|\bcopy\b|\.backup|VACUUM' '$ROOT/scripts/audit-session.sh'"

# argument handling
out=$(run ses_NOPE); rc=$?
expect_eq "unknown session id exits 2" 2 "$rc"; expect_contains "and names the id" "$out" "session not found in the database: ses_NOPE"
out=$(run "x'; drop table session_v2;--"); rc=$?
expect_eq "an id that is not ses_<alnum> exits 2 (no SQL injection through the argument)" 2 "$rc"
expect_eq "(and the table is still there)" 1 "$(sqlite3 -readonly "$db" "select count(*) from session_v2 where id='ses_MAIN';")"
out=$(HOME="$home" env -u OPENCODE_SESSION_ID bash "$ROOT/scripts/audit-session.sh" 2>&1); rc=$?
expect_eq "no argument and no OPENCODE_SESSION_ID exits 2" 2 "$rc"
out=$(HOME="$home" OPENCODE_SESSION_ID=ses_MAIN bash "$ROOT/scripts/audit-session.sh" 2>&1); rc=$?
expect_eq "no argument uses OPENCODE_SESSION_ID" 0 "$rc"; expect_contains "(and audits that session)" "$out" "# audit of ses_MAIN"
out=$(HOME="$sb/empty" bash "$ROOT/scripts/audit-session.sh" ses_MAIN 2>&1); rc=$?
expect_eq "a missing database exits 2" 2 "$rc"; expect_contains "and names the file" "$out" "cannot read the database"

# pins follow opencode.jsonc: with the profile `haiku` the main session must be on Haiku
repo="$sb/repo"; make_repo_copy "$repo"
python3 - "$repo/opencode.jsonc" << 'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace('"model": "console-cloudflare-ai-gateway-custom/claude-sonnet-5-5#xhigh"', '"model": "console-cloudflare-ai-gateway-custom/claude-haiku-5-5#high"', 1)
open(p, "w").write(s)
PY
out=$(HOME="$home" bash "$repo/scripts/audit-session.sh" ses_MAIN 2>&1)
expect_contains "the main pin is read from the copy's opencode.jsonc (agents.build.model)" "$out" "main=console-cloudflare-ai-gateway-custom/claude-haiku-5-5"

finish
