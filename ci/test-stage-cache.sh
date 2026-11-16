#!/usr/bin/env bash
# ci/test-stage-cache.sh - checks ci/stage-cache.sh (decisions 0391, 0395) against a fake aws (a directory store, under $TMPDIR/opencode; no network, no credential, no R2):
# save writes the three objects and reads them back; restore of the same key puts the outputs back (directories, files and glob patterns) and says hit=true; a key with no entry
# says hit=false and makes no directory; an entry whose tar, file list, member, or file does not match fails the step (exit 1) and says so; a directory with a symbolic link is
# not saved; a path that is missing, or outside the build's directory, stops the step; a directory among the outputs is emptied before the restore.
# The fake aws answers only the calls ci/stage-cache.sh makes (head-object, get-object, put-object); anything else fails the test.
# usage: bash ci/test-stage-cache.sh
set -u -o pipefail
here=$(cd "$(dirname "$0")" && pwd) || exit 2
script="$here/stage-cache.sh"
base=${TMPDIR:-/tmp}/opencode/stage-cache-test
rm -rf "$base"; mkdir -p "$base/bin" "$base/store"
pass=0; fail=0
check() { if [ "$2" = yes ]; then pass=$((pass + 1)); echo "PASS  $1"; else fail=$((fail + 1)); echo "FAIL  $1"; fi; }

cat > "$base/bin/aws" <<'FAKE'
#!/usr/bin/env bash
# fake aws for ci/test-stage-cache.sh: the store is $FAKE_STORE/<bucket>/<key>
set -u
op=${1:-}; sub=${2:-}; shift 2 || true
[ "$op" = s3api ] || { echo "fake aws: unexpected $op" >&2; exit 9; }
bucket=""; key=""; body=""; ctype=""; positional=""
while [ $# -gt 0 ]; do
  case "$1" in
    --bucket) bucket=$2; shift 2 ;;
    --key) key=$2; shift 2 ;;
    --body) body=$2; shift 2 ;;
    --content-type) ctype=$2; shift 2 ;;
    --endpoint-url) shift 2 ;;
    --output) shift 2 ;;
    --*) echo "fake aws: unexpected option $1" >&2; exit 9 ;;
    *) positional=$1; shift ;;
  esac
done
obj="$FAKE_STORE/$bucket/$key"
case "$sub" in
  head-object)
    if [ -f "$obj" ]; then echo '{"ContentLength": 1}'; exit 0; fi
    echo "An error occurred (404) when calling the HeadObject operation: Not Found" >&2; exit 255 ;;
  get-object)
    [ -f "$obj" ] || { echo "An error occurred (NoSuchKey) when calling the GetObject operation: The specified key does not exist." >&2; exit 255; }
    if [ "${FAKE_FAIL:-}" = "get:$key" ]; then echo "fake: the GET of $key fails" >&2; exit 254; fi
    cp "$obj" "$positional"; exit 0 ;;
  put-object)
    mkdir -p "$(dirname "$obj")"; cp "$body" "$obj"; exit 0 ;;
  *) echo "fake aws: unexpected s3api $sub" >&2; exit 9 ;;
esac
FAKE
chmod +x "$base/bin/aws"

export PATH="$base/bin:$PATH" FAKE_STORE="$base/store" ID=test-account-placeholder
export RUNNER_TEMP="$base/runner" WORK="$base/work"
mkdir -p "$RUNNER_TEMP"
KEY=fl-hybrid-5e1f0c9a7b3d2e8f4a6c0b1d9e7f3a2c5b8d4e61
STAGE=hybrid
PATHS="w-Oz/web/flengine6.opt.wasm hyb/n0-*.wasm w-Oz/obj/fle"
export KEY STAGE PATHS

# the build's directory (BASE): the outputs of one stage, in the layout of app/scripts/build-engine-bundle.sh
src="$base/src"
mkdir -p "$src/w-Oz/web" "$src/hyb" "$src/w-Oz/obj/fle" "$src/w-Oz/obj/log"
head -c 4000 /dev/zero > "$src/w-Oz/web/flengine6.opt.wasm"
echo "glue" > "$src/w-Oz/web/flengine6.js"
echo "n0 a" > "$src/hyb/n0-fle.wasm"; echo "n0 b" > "$src/hyb/n0-dsp.wasm"; echo "not an output" > "$src/hyb/other.txt"
echo "fle-00 object" > "$src/w-Oz/obj/fle/fle-00.o"
echo "a log, not an output" > "$src/w-Oz/obj/log/x.log"

run() { # subcommand, BASE, GITHUB_OUTPUT file -> the output of the script and its rc in RC
  local out=$3
  : > "$out"
  OUT=$(BASE="$2" GITHUB_OUTPUT="$out" bash "$script" "$1" 2>&1); RC=$?
}

# save from a directory that is not the build's: the globs are read inside BASE, not inside the repository
cd "$RUNNER_TEMP" || exit 2
run save "$src" "$base/o-save"
check "save writes three objects and says so (rc 0)" "$([ "$RC" = 0 ] && [ -f "$FAKE_STORE/fl-web-cache/stage-cache/$KEY/out.tar" ] && [ -f "$FAKE_STORE/fl-web-cache/stage-cache/$KEY/out.tar.sha256" ] && [ -f "$FAKE_STORE/fl-web-cache/stage-cache/$KEY/SHA256SUMS" ] && echo yes || echo no)"
check "save reports the read-back check (件数 4: the glob gives two files)" "$(printf '%s' "$OUT" | grep -q '保存した。3 件を書き' && printf '%s' "$OUT" | grep -q '件数 4' && echo yes || echo no)"
check "save does not print the account ID" "$(printf '%s' "$OUT" | grep -q "$ID" && echo no || echo yes)"
check "save packs the glob's files and the directory's files, not the log or the other file" "$(tar -tf "$FAKE_STORE/fl-web-cache/stage-cache/$KEY/out.tar" | sort | tr '\n' ' ' | grep -q 'hyb/n0-dsp.wasm hyb/n0-fle.wasm w-Oz/obj/fle/fle-00.o w-Oz/web/flengine6.opt.wasm ' && ! tar -tf "$FAKE_STORE/fl-web-cache/stage-cache/$KEY/out.tar" | grep -q 'other.txt\|x.log' && echo yes || echo no)"

rm -rf "$RUNNER_TEMP/work"
run restore "$RUNNER_TEMP/work" "$base/o-restore"
check "restore of the saved key: rc 0, hit=true" "$([ "$RC" = 0 ] && grep -qx 'hit=true' "$base/o-restore" && echo yes || echo no)"
check "restore puts the outputs back, byte for byte (directory, file, glob)" "$(cmp -s "$src/w-Oz/web/flengine6.opt.wasm" "$RUNNER_TEMP/work/w-Oz/web/flengine6.opt.wasm" && cmp -s "$src/hyb/n0-dsp.wasm" "$RUNNER_TEMP/work/hyb/n0-dsp.wasm" && cmp -s "$src/w-Oz/obj/fle/fle-00.o" "$RUNNER_TEMP/work/w-Oz/obj/fle/fle-00.o" && echo yes || echo no)"
check "restore does not put back what was not saved (the log, the other file)" "$([ ! -e "$RUNNER_TEMP/work/hyb/other.txt" ] && [ ! -e "$RUNNER_TEMP/work/w-Oz/obj/log" ] && echo yes || echo no)"

# a stale file in a directory among the outputs is removed before the restore
echo stale > "$RUNNER_TEMP/work/w-Oz/obj/fle/stale.o"
run restore "$RUNNER_TEMP/work" "$base/o-stale"
check "a directory among the outputs is emptied before the restore (a stale file goes)" "$([ "$RC" = 0 ] && [ ! -e "$RUNNER_TEMP/work/w-Oz/obj/fle/stale.o" ] && echo yes || echo no)"

rm -rf "$RUNNER_TEMP/work"
KEYMISS=fl-hybrid-ffffffffffffffffffffffffffffffffffffffff
KEY=$KEYMISS run restore "$RUNNER_TEMP/work" "$base/o-miss"
check "a key with no entry: rc 0, hit=false, and no output is made" "$([ "$RC" = 0 ] && grep -qx 'hit=false' "$base/o-miss" && [ ! -e "$RUNNER_TEMP/work/w-Oz" ] && echo yes || echo no)"
check "a key with no entry is reported as 404, not as a failure" "$(printf '%s' "$OUT" | grep -q '404' && echo yes || echo no)"

# an entry that is there and does not restore: the step fails
tamper() { # label, object name, a command that changes the stored object (it sees $obj)
  local label=$1 name=$2 cmd=$3
  local obj="$FAKE_STORE/fl-web-cache/stage-cache/$KEY/$name"
  cp "$obj" "$obj.orig"
  eval "$cmd"
  rm -rf "$RUNNER_TEMP/work"
  run restore "$RUNNER_TEMP/work" "$base/o-$label"
  check "$label: the step fails (exit 1) and asks for a re-run" "$([ "$RC" = 1 ] && printf '%s' "$OUT" | grep -q '再実行' && echo yes || echo no)"
  check "$label: no hit line is written" "$(grep -q '^hit=' "$base/o-$label" && echo no || echo yes)"
  mv "$obj.orig" "$obj"
}
tamper tar-changed out.tar "printf 'x' >> \"\$obj\""
tamper list-changed SHA256SUMS "sed -i '1s/^[0-9a-f]\{64\}/$(printf '0%.0s' {1..64})/' \"\$obj\""

# a tar with a member that is not safe (an absolute name)
python3 - "$base/evil.tar" <<'PY'
import io, sys, tarfile
with tarfile.open(sys.argv[1], "w") as t:
    d = b"x"
    i = tarfile.TarInfo("/etc/evil"); i.size = len(d); t.addfile(i, io.BytesIO(d))
PY
obj="$FAKE_STORE/fl-web-cache/stage-cache/$KEY"
sha=$(sha256sum "$base/evil.tar" | cut -d' ' -f1)
cp "$base/evil.tar" "$obj/out.tar"; printf '%s  out.tar\n' "$sha" > "$obj/out.tar.sha256"
rm -rf "$RUNNER_TEMP/work"; run restore "$RUNNER_TEMP/work" "$base/o-evil"
check "a tar with an absolute member: the step fails (exit 1)" "$([ "$RC" = 1 ] && printf '%s' "$OUT" | grep -q '想定外' && echo yes || echo no)"
if [ ! -e "/etc/evil" ]; then check "a tar with an absolute member writes nothing outside (no /etc/evil)" yes; else check "a tar with an absolute member writes nothing outside (no /etc/evil)" no; fi

# a directory with a symbolic link is not saved
ln -sfn ../hyb/other.txt "$src/w-Oz/obj/fle/link.o"
KEY2=fl-hybrid-1111111111111111111111111111111111111111
KEY=$KEY2 run save "$src" "$base/o-link"
check "a directory with a symbolic link is not saved (exit 1)" "$([ "$RC" = 1 ] && printf '%s' "$OUT" | grep -q '記号リンク' && echo yes || echo no)"
rm -f "$src/w-Oz/obj/fle/link.o"

# an output that does not exist stops the save (exit 1)
KEY3=fl-hybrid-2222222222222222222222222222222222222222
KEY=$KEY3 PATHS="w-Oz/web/flengine6.opt.wasm w-Oz/web/nothere.wasm" run save "$src" "$base/o-missing"
check "an output that does not exist stops the save (exit 1, nothing is written)" "$([ "$RC" = 1 ] && printf '%s' "$OUT" | grep -q '出力が無い' && [ ! -e "$FAKE_STORE/fl-web-cache/stage-cache/$KEY3/out.tar" ] && echo yes || echo no)"

# an output outside the build's directory is refused, on both subcommands
KEY4=fl-hybrid-3333333333333333333333333333333333333333
KEY=$KEY4 PATHS="../../etc" run save "$src" "$base/o-outside"
check "an output outside the build's directory is refused (exit 1)" "$([ "$RC" = 1 ] && printf '%s' "$OUT" | grep -q 'BASE の外' && echo yes || echo no)"
KEY=$KEY4 PATHS="/etc" run restore "$src" "$base/o-outside2"
check "an absolute output is refused on restore too (exit 1)" "$([ "$RC" = 1 ] && printf '%s' "$OUT" | grep -q 'BASE の外' && echo yes || echo no)"

# a get that fails after the head said the entry is there: the step fails
KEY=$KEY run save "$src" "$base/o-save2" > /dev/null
rm -rf "$RUNNER_TEMP/work"
export FAKE_FAIL="get:stage-cache/$KEY/out.tar"; run restore "$RUNNER_TEMP/work" "$base/o-get"; unset FAKE_FAIL
check "a get that fails on an entry that exists: the step fails (exit 1)" "$([ "$RC" = 1 ] && printf '%s' "$OUT" | grep -q '取り出しに失敗' && echo yes || echo no)"

# a key of another stage is refused
KEYBAD=fl-wasm-5e1f0c9a7b3d2e8f4a6c0b1d9e7f3a2c5b8d4e61
rm -rf "$RUNNER_TEMP/work"
KEY=$KEYBAD run restore "$RUNNER_TEMP/work" "$base/o-bad"
check "a key of another stage is refused (exit 1, no restore)" "$([ "$RC" = 1 ] && [ ! -e "$RUNNER_TEMP/work" ] && echo yes || echo no)"

echo "test-stage-cache: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
