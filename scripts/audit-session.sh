#!/usr/bin/env bash
# usage: scripts/audit-session.sh [SESSION_ID] [--since "YYYY-MM-DD HH:MM"]     (no argument: $OPENCODE_SESSION_ID)
# --since (JST): count only the children created at or after that time, and only the main session's rows from that time on.
# Read-only audit of one main session and its children, from opencode's database (opened with sqlite3 -readonly; never copied, never written).
# What it shows: model switches, assistant rows per model (main vs children), the main session's own edit/write/shell calls,
# gaps of 60 minutes or more without any row, rows whose model is not the pinned one, and a few risky command counts.
# The pins are read from opencode.jsonc (agents.build.model for the main session, agents.worker/reviewer.model for children).
# Exit 0 when the audit ran. It reports; it does not decide. A non-zero count under "rows NOT on the pinned model" is a finding to report.
set -o pipefail

die() { echo "audit-session: $*" >&2; exit 2; }

sid=""; since=""; since_given=0
while [ $# -gt 0 ]; do
  case "$1" in
    --since) [ $# -ge 2 ] || die "--since には \"YYYY-MM-DD HH:MM\"(JST)を指定してください"; since="$2"; since_given=1; shift 2 ;;
    *) [ -n "$sid" ] || sid="$1"; shift ;;
  esac
done
sid="${sid:-${OPENCODE_SESSION_ID:-}}"
[ -n "$sid" ] || die "no session id: pass it as the argument, or run inside a session (OPENCODE_SESSION_ID)"
[[ "$sid" =~ ^ses_[A-Za-z0-9]+$ ]] || die "invalid session id (expected ses_ followed by letters and digits): $sid"
for tool in sqlite3 jq python3; do command -v "$tool" > /dev/null || die "$tool is required"; done
since_ms=""
if [ "$since_given" = 1 ]; then
  re='^[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}$'
  [[ "$since" =~ $re ]] || die "--since の書式が不正です(\"YYYY-MM-DD HH:MM\"、JST): $since"
  since_ms=$(python3 -c 'import sys; from datetime import datetime, timedelta, timezone; d = datetime.strptime(sys.argv[1], "%Y-%m-%d %H:%M").replace(tzinfo=timezone(timedelta(hours=9))); print(int(d.timestamp() * 1000))' "$since" 2> /dev/null) || die "--since の日時が存在しません(JST): $since"
fi
# " and <column>>=<since>" when --since is given; nothing otherwise (the queries below are then unchanged)
tm() { [ -n "$since_ms" ] && printf ' and %s>=%s' "$1" "$since_ms"; return 0; }
# the rows of the main session and its children (alias m = session_message, s = session_v2); main rows by message time, children by creation time
scope() {
  if [ -n "$since_ms" ]; then printf "((s.id='%s' and m.time_created>=%s) or (s.parent_id='%s' and s.time_created>=%s))" "$sid" "$since_ms" "$sid" "$since_ms"
  else printf "(s.id='%s' or s.parent_id='%s')" "$sid" "$sid"; fi
}

db="$HOME/.local/share/opencode/opencode.db"
[ -r "$db" ] || die "cannot read the database: $db"
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || die "cannot resolve the repository root"
config="$root/opencode.jsonc"
[ -r "$config" ] || die "cannot read $config"

# opencode.jsonc -> JSON on stdout (comments and trailing commas removed; strings are respected).
jsonc_to_json() {
  python3 - "$1" << 'PY'
import json, re, sys
t = open(sys.argv[1], encoding="utf-8").read()
out, i, n, ins = [], 0, len(t), False
while i < n:
    c = t[i]
    if ins:
        out.append(c)
        if c == "\\":
            out.append(t[i + 1]); i += 2; continue
        if c == '"': ins = False
        i += 1; continue
    if c == '"': ins = True; out.append(c); i += 1; continue
    if t.startswith("//", i):
        while i < n and t[i] != "\n": i += 1
        continue
    if t.startswith("/*", i):
        i = t.index("*/", i + 2) + 2; continue
    out.append(c); i += 1
s = re.sub(r",(\s*[}\]])", r"\1", "".join(out))
json.dump(json.loads(s), sys.stdout)
PY
}

json=$(jsonc_to_json "$config") || die "cannot parse $config as JSONC"
# "provider/model#variant" -> "provider/model" (assistant rows carry provider and model separately).
pin() { jq -r --arg a "$1" '.agents[$a].model // empty | split("#")[0]' <<< "$json"; }
main_pin=$(pin build)
worker_pin=$(pin worker)
reviewer_pin=$(pin reviewer)
{ [ -n "$main_pin" ] && [ -n "$worker_pin" ] && [ -n "$reviewer_pin" ]; } || die "agents.build/worker/reviewer.model missing in $config"
child_pins=$(printf '%s\n%s\n' "$worker_pin" "$reviewer_pin" | sort -u | paste -sd, -)
main_in="'$main_pin'"
child_in="'${child_pins//,/\',\'}'"

q() { sqlite3 -readonly -header -column "$db" "$1" || die "query failed (database busy or schema changed?)"; }

exists=$(sqlite3 -readonly "$db" "select count(*) from session_v2 where id='$sid';") || die "cannot query the database"
[ "$exists" = "1" ] || die "session not found in the database: $sid"
children=$(sqlite3 -readonly "$db" "select count(*) from session_v2 where parent_id='$sid'$(tm time_created);")

echo "# audit of $sid (children: $children)"
echo "# pins from opencode.jsonc: main=$main_pin  children=$child_pins"
if [ -n "$since_ms" ]; then echo "# since (JST): $since  (children created at or after it; main rows from then on)"; fi
echo
echo "## 1. main: model-switched count"
q "select count(*) as model_switched from session_message where session_id='$sid'$(tm time_created) and type='model-switched';"
echo
echo "## 1b. main: model-switched list (time JST, from -> to)"
q "select datetime(time_created/1000+9*3600,'unixepoch') as jst,
   json_extract(data,'\$.previous.providerID')||'/'||json_extract(data,'\$.previous.id')||'#'||coalesce(json_extract(data,'\$.previous.variant'),'') as from_model,
   json_extract(data,'\$.model.providerID')||'/'||json_extract(data,'\$.model.id')||'#'||coalesce(json_extract(data,'\$.model.variant'),'') as to_model
   from session_message where session_id='$sid'$(tm time_created) and type='model-switched' order by seq;"
echo
echo "## 1c. children: model-switched rows (someone switched a child's model)"
q "select count(*) as switched_rows, count(distinct m.session_id) as children
   from session_message m join session_v2 s on s.id=m.session_id where s.parent_id='$sid'$(tm s.time_created) and m.type='model-switched';"
echo
echo "## 2. assistant rows by model: main vs child"
q "select case when s.parent_id is null then 'main' else 'child' end as kind,
   json_extract(m.data,'\$.model.providerID')||'/'||json_extract(m.data,'\$.model.id') as model,
   count(*) as rows
   from session_message m join session_v2 s on s.id=m.session_id
   where m.type='assistant' and $(scope)
   group by 1,2 order by 1,3 desc;"
echo
echo "## 3. main: edit / write / patch / shell tool calls (the main session should make none)"
q "select json_extract(c.value,'\$.name') as tool, json_extract(c.value,'\$.state.status') as status, count(*) as calls
   from session_message m, json_each(m.data,'\$.content') c
   where m.session_id='$sid'$(tm m.time_created) and m.type='assistant' and json_extract(c.value,'\$.type')='tool'
     and json_extract(c.value,'\$.name') in ('edit','write','patch','shell','bash')
   group by 1,2 order by 1,2;"
echo
echo "## 3b. main: direct work total (edit+write+patch)"
q "select count(*) as direct_work
   from session_message m, json_each(m.data,'\$.content') c
   where m.session_id='$sid'$(tm m.time_created) and m.type='assistant' and json_extract(c.value,'\$.type')='tool'
     and json_extract(c.value,'\$.name') in ('edit','write','patch');"
echo
echo "## 4. gaps >= 60 min between consecutive rows (count, total hours)"
q "with t as (select time_created, lag(time_created) over (order by seq) as p from session_message where session_id='$sid'$(tm time_created))
   select count(*) as gaps, round(sum(time_created-p)/3600000.0,1) as total_hours from t where time_created-p>=3600000;"
echo
echo "## 4b. the first 12 gaps (start JST, minutes, model of the last assistant row before the gap)"
q "with t as (select seq, time_created, lag(time_created) over (order by seq) as p from session_message where session_id='$sid'$(tm time_created)),
   g as (select p, time_created from t where time_created-p>=3600000)
   select datetime(g.p/1000+9*3600,'unixepoch') as gap_start_jst, round((g.time_created-g.p)/60000.0) as minutes,
     (select json_extract(a.data,'\$.model.providerID')||'/'||json_extract(a.data,'\$.model.id') from session_message a where a.session_id='$sid' and a.type='assistant' and a.time_created<=g.p order by a.time_created desc limit 1) as last_model
   from g order by g.p limit 12;"
echo
echo "## 5. assistant rows NOT on the pinned model (main must be $main_pin; children must be one of: $child_pins)"
q "select case when s.parent_id is null then 'main' else 'child' end as kind,
   json_extract(m.data,'\$.model.providerID')||'/'||json_extract(m.data,'\$.model.id') as model, count(*) as rows_off_pin
   from session_message m join session_v2 s on s.id=m.session_id
   where m.type='assistant' and $(scope)
     and ((s.parent_id is null and json_extract(m.data,'\$.model.providerID')||'/'||json_extract(m.data,'\$.model.id') not in ($main_in))
       or (s.parent_id is not null and json_extract(m.data,'\$.model.providerID')||'/'||json_extract(m.data,'\$.model.id') not in ($child_in)))
   group by 1,2 order by 3 desc limit 12;"
echo
echo "## 6. subagent tool calls carrying a model argument (main)"
q "select count(*) as subagent_calls,
   coalesce(sum(coalesce(json_extract(c.value,'\$.state.input.model'),'')!=''),0) as with_model_arg
   from session_message m, json_each(m.data,'\$.content') c
   where m.session_id='$sid'$(tm m.time_created) and m.type='assistant' and json_extract(c.value,'\$.type')='tool' and json_extract(c.value,'\$.name')='subagent';"
echo
echo "## 7. risky shell commands (only state.input.command is searched; main / child). Counts of calls that contain the text"
q "with c as (
     select case when s.parent_id is null then 'main' else 'child' end as kind,
            json_extract(t.value,'\$.state.input.command') as cmd
     from session_message m join session_v2 s on s.id=m.session_id, json_each(m.data,'\$.content') t
     where m.type='assistant' and $(scope)
       and json_extract(t.value,'\$.type')='tool' and json_extract(t.value,'\$.name')='shell')
   select kind, count(*) as shell_calls,
     sum(cmd like '%opencode auth%') as opencode_auth,
     sum(cmd like '%gh secret%') as gh_secret,
     sum(cmd like '%gh api%-X%' or cmd like '%gh api%--method%') as gh_api_write,
     sum(cmd like '%git push%--force%' or cmd like '%git push%-f %' or cmd like '%git push%+%') as force_push,
     sum(cmd like '%env sleep%' or cmd like '%=% sleep %' or cmd like '%bash -c%sleep%' or cmd like '%xargs sleep%') as sleep_bypass,
     sum(cmd like '%GH_CONFIG_DIR=%' or cmd like '%GIT_CONFIG_GLOBAL=%') as other_identity,
     sum(cmd like '%.config/botid%' or cmd like '%.local/share/botid%') as botid_paths
   from c group by kind order by kind;"
