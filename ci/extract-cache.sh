#!/usr/bin/env bash
# ci/extract-cache.sh SUBCOMMAND - the extraction layer (decision 0386): the extracted tree of the FL Studio installer (HOST), from the R2 cache when the cache has it.
# Used by .github/actions/extract-layer, which deploy-engine.yml and agent.yml call (each subcommand is one step of that action, run from the repository root).
#   key      the cache key: the SHA-256 of the installer's row in ci/artifacts.lock and of the files the extraction reads (the same method as the prefix key of deploy-engine.yml)
#   account  the account ID of the R2 endpoint, from the API token (env CLOUDFLARE_API_TOKEN). Masked, written to GITHUB_OUTPUT as id
#   restore  reads the entry extract-cache/<key>/ from the cache bucket and checks it against the SHA-256 recorded at save. Any failure: hit=false (the next steps extract again)
#   fetch    the installer from the artifact bucket, checked against ci/artifacts.lock (size and SHA-256)
#   extract  ci/lib/extract_fl.py into HOST; the manifest (SHA-256 per file) goes to WORK
#   save     packs HOST and writes the tar, its SHA-256 and the manifest to the cache bucket (the manifest last), then reads them back and checks them
# env: RUNNER_TEMP, HOST (the tree to fill), WORK (scratch), and per subcommand ID, KEY, CLOUDFLARE_API_TOKEN, ARTIFACT_* or CACHE_* (the R2 keys: only the subcommands that use them get them).
set -u

INST=installers/flstudio_win64_26.1.7.5653.exe
CACHE_BUCKET=fl-web-cache
ARTIFACT_BUCKET=fl-web-artifacts

die() { echo "$*" >&2; exit 1; }

s3env() {
  : "${ID:?}"
  export AWS_DEFAULT_REGION=auto AWS_EC2_METADATA_DISABLED=true AWS_REQUEST_CHECKSUM_CALCULATION=when_required AWS_RESPONSE_CHECKSUM_VALIDATION=when_required
  ENDPOINT="https://${ID}.r2.cloudflarestorage.com"
}

mask() {
  sed -E -e "s/${ID}/[伏せ]/g" -e 's/[A-Za-z0-9.-]+\.r2\.cloudflarestorage\.com/[ホスト伏せ]/g'
}

clean_host() {
  case "$HOST" in
    "$RUNNER_TEMP"/*) rm -rf "$HOST" ;;
    *) die "HOST が RUNNER_TEMP の下に無い(中止)" ;;
  esac
}

miss() {
  echo "抽出キャッシュ: 使わない($1)。取得と抽出をする"
  clean_host
  echo "hit=false" >> "$GITHUB_OUTPUT"
  exit 0
}

key() {
  row=$(awk -F'\t' -v k="$INST" '$1 == k' ci/artifacts.lock)
  [ -n "$row" ] || die "ci/artifacts.lock に、インストーラの行が無い"
  k=$({ echo "extract-layer v1"; printf '%s\n' "$row"; git ls-files -s -- ci/extract-dlls.tsv ci/lib/extract_fl.py tools/extract_nsis2.py tools/nsis_script_dump.py \
    lift/harness/seeds/gen_install_tree.py lift/harness/seeds/vfs-seed-idle1.tsv lift/harness/seeds/os-fonts; } | sha256sum | cut -d' ' -f1)
  [[ $k =~ ^[0-9a-f]{64}$ ]] || die "抽出キャッシュの鍵が作れない"
  echo "key=$k" >> "$GITHUB_OUTPUT"
  echo "抽出キャッシュの鍵: 入力のハッシュ(同じ入力なら同じ鍵)"
}

account() {
  : "${CLOUDFLARE_API_TOKEN:?}" "${WORK:?}"
  W=$WORK
  mkdir -p "$W"
  curl -sS -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" https://api.cloudflare.com/client/v4/accounts > "$W/accounts.json"
  cand=$(python3 -c 'import json, sys; d = json.load(open(sys.argv[1])); [print(a["id"]) for a in d.get("result", [])]' "$W/accounts.json" \
    | grep -E '^[0-9a-f]{32}$' | sort -u || true)
  rm -f "$W/accounts.json"
  n=$(printf '%s\n' "$cand" | grep -c . || true)
  if [ "$n" -ne 1 ]; then
    die "アカウント ID を 1 つに決められない(件数: $n)"
  fi
  echo "::add-mask::$cand"
  echo "id=$cand" >> "$GITHUB_OUTPUT"
  echo "アカウント ID を実行時に 1 つ得た(値は出さない)"
}

restore() {
  : "${KEY:?}" "${HOST:?}" "${WORK:?}"
  s3env
  W=$WORK
  mkdir -p "$W"
  P="extract-cache/$KEY"
  if ! aws s3api head-object --bucket "$CACHE_BUCKET" --key "$P/SHA256SUMS" --endpoint-url "$ENDPOINT" > "$W/h.json" 2> "$W/h.err" < /dev/null; then
    grep -qE '404|Not Found|NoSuchKey' "$W/h.err" || miss "抽出キャッシュの確認が 404 以外で失敗"
    miss "無い(404)"
  fi
  for f in host.tar host.tar.sha256 SHA256SUMS; do
    aws s3api get-object --bucket "$CACHE_BUCKET" --key "$P/$f" --endpoint-url "$ENDPOINT" "$W/c-$f" > "$W/g.txt" 2>&1 < /dev/null || miss "取り出しに失敗($f)"
  done
  want=$(awk '{print $1}' "$W/c-host.tar.sha256")
  got=$(sha256sum "$W/c-host.tar" | cut -d' ' -f1)
  [ -n "$want" ] && [ "$got" = "$want" ] || miss "tar の SHA-256 が保存時と違う"
  python3 - "$W/c-host.tar" <<'PY' || miss "tar の中身が想定外(絶対パス・..・リンク)"
import sys, tarfile
with tarfile.open(sys.argv[1]) as t:
    for m in t.getmembers():
        if m.name.startswith("/") or ".." in m.name.split("/") or not (m.isfile() or m.isdir()):
            sys.exit(1)
PY
  clean_host
  mkdir -p "$HOST"
  tar -C "$HOST" -xf "$W/c-host.tar" || miss "tar の展開に失敗"
  cp "$W/c-SHA256SUMS" "$W/SHA256SUMS"
  n=$(wc -l < "$W/SHA256SUMS" | tr -d ' ')
  f=$(cd "$HOST" && find . -type f | wc -l | tr -d ' ')
  [ "$n" = "$f" ] || miss "件数が違う(記録 $n、展開 $f)"
  (cd "$HOST" && sha256sum -c "$W/SHA256SUMS" > "$W/check.txt" 2>&1) || miss "保存時の SHA-256 と違うファイルがある"
  echo "抽出キャッシュ: 復元した(${n} 件。保存時の SHA-256 と一致)"
  echo "hit=true" >> "$GITHUB_OUTPUT"
  exit 0
}

fetch() {
  : "${ID:?}" "${WORK:?}" "${HOST:?}"
  s3env
  W=$WORK
  mkdir -p "$W/in"
  INSTF="$W/in/$(basename "$INST")"
  if ! aws s3 cp "s3://$ARTIFACT_BUCKET/$INST" "$INSTF" --only-show-errors --endpoint-url "$ENDPOINT" > "$W/s3.txt" 2>&1 < /dev/null; then
    echo "インストーラの取得: 失敗(伏せた出力)"
    mask < "$W/s3.txt" | tail -n 5
    exit 1
  fi
  row=$(awk -F'\t' -v k="$INST" '$1 == k' ci/artifacts.lock)
  want_size=$(printf '%s\n' "$row" | cut -f2)
  want_sha=$(printf '%s\n' "$row" | cut -f3)
  got_size=$(stat -c %s "$INSTF")
  got_sha=$(sha256sum "$INSTF" | cut -d' ' -f1)
  if [ -z "$want_sha" ] || [ "$got_size" != "$want_size" ] || [ "$got_sha" != "$want_sha" ]; then
    echo "照合: 不一致(大きさ $got_size B、期待 $want_size B)"
    rm -f "$INSTF"
    exit 1
  fi
  echo "照合: 一致(大きさ $got_size B、SHA-256 一致)"
}

extract() {
  : "${WORK:?}" "${HOST:?}"
  W=$WORK
  INSTF="$W/in/$(basename "$INST")"
  clean_host
  mkdir -p "$HOST" "$W"
  if python3 ci/lib/extract_fl.py "$INSTF" "$HOST" --dlls ci/extract-dlls.tsv --seed lift/harness/seeds/vfs-seed-idle1.tsv --manifest "$W/SHA256SUMS" > "$W/extract.txt" 2>&1; then
    echo "抽出: 成功(rc=0。記録した SHA-256 の件数 $(wc -l < "$W/SHA256SUMS" | tr -d ' '))"
    rm -f "$INSTF"
  else
    rc=$?
    echo "抽出: 失敗(rc=$rc。末尾 40 行)"
    tail -n 40 "$W/extract.txt"
    exit 1
  fi
}

save() {
  : "${KEY:?}" "${HOST:?}" "${WORK:?}"
  s3env
  W=$WORK
  P="extract-cache/$KEY"
  (cd "$HOST" && sha256sum -c "$W/SHA256SUMS" > "$W/check.txt" 2>&1) || die "保存の前の検査: 失敗(中止)"
  tar -C "$HOST" -cf "$W/host.tar" . || die "tar が作れない"
  printf '%s  host.tar\n' "$(sha256sum "$W/host.tar" | cut -d' ' -f1)" > "$W/host.tar.sha256"
  put() {
    aws s3api put-object --bucket "$CACHE_BUCKET" --key "$P/$1" --body "$2" --content-type "$3" --endpoint-url "$ENDPOINT" > "$W/p.txt" 2>&1 < /dev/null \
      || { echo "保存: 書けない($1。伏せた末尾)"; mask < "$W/p.txt" | tail -n 3; exit 1; }
  }
  put host.tar "$W/host.tar" application/x-tar
  put host.tar.sha256 "$W/host.tar.sha256" text/plain
  put SHA256SUMS "$W/SHA256SUMS" text/plain
  aws s3api get-object --bucket "$CACHE_BUCKET" --key "$P/host.tar" --endpoint-url "$ENDPOINT" "$W/v-host.tar" > "$W/g.txt" 2>&1 < /dev/null \
    || { echo "保存の確認: tar を読めない(伏せた末尾)"; mask < "$W/g.txt" | tail -n 3; exit 1; }
  aws s3api get-object --bucket "$CACHE_BUCKET" --key "$P/SHA256SUMS" --endpoint-url "$ENDPOINT" "$W/v-SHA256SUMS" > "$W/g.txt" 2>&1 < /dev/null \
    || { echo "保存の確認: 記録を読めない(伏せた末尾)"; mask < "$W/g.txt" | tail -n 3; exit 1; }
  want=$(awk '{print $1}' "$W/host.tar.sha256")
  [ "$(sha256sum "$W/v-host.tar" | cut -d' ' -f1)" = "$want" ] || { echo "保存の確認: tar の SHA-256 が違う"; exit 1; }
  cmp -s "$W/v-SHA256SUMS" "$W/SHA256SUMS" || { echo "保存の確認: 記録が違う"; exit 1; }
  echo "保存: 3 件を書き、読み直して SHA-256 を確かめた(tar $(stat -c %s "$W/host.tar") B)"
}

case "${1:-}" in
  key) key ;;
  account) account ;;
  restore) restore ;;
  fetch) fetch ;;
  extract) extract ;;
  save) save ;;
  *) echo "usage: ci/extract-cache.sh key|account|restore|fetch|extract|save" >&2; exit 2 ;;
esac
