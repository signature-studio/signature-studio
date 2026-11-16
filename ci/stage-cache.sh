#!/usr/bin/env bash
# ci/stage-cache.sh restore|save - the output directory of one stage of app/scripts/build-engine-bundle.sh, in the R2 cache (decision 0391). Used by .github/actions/stage-cache
# (one step per subcommand, run from the repository root; the same shape as ci/extract-cache.sh of decision 0386).
#   restore  reads stage-cache/<KEY>/ from the cache bucket. No entry (404): hit=false, the stage runs. An entry that does not restore (a missing object, the tar's SHA-256
#            not the one recorded at save, a member that is not a file or a directory, a file count or a file SHA-256 that differs): the step FAILS, because the entry
#            is there and the stage must come from it (re-run the job; a transient R2 error is the usual cause).
#   save     packs DIR into out.tar, writes out.tar.sha256 and SHA256SUMS (the SHA-256 of each file, written last), then reads all three back and checks them.
# env: RUNNER_TEMP, STAGE, KEY, DIR (the output directory, under RUNNER_TEMP), WORK (scratch); for the R2 subcommands ID (the account ID, masked here),
#      AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY (the cache token: the action gives them to these steps only).
set -u -o pipefail
CACHE_BUCKET=fl-web-cache

die() { echo "$*" >&2; exit 1; }

s3env() {
  : "${ID:?}"
  export AWS_DEFAULT_REGION=auto AWS_EC2_METADATA_DISABLED=true AWS_REQUEST_CHECKSUM_CALCULATION=when_required AWS_RESPONSE_CHECKSUM_VALIDATION=when_required
  ENDPOINT="https://${ID}.r2.cloudflarestorage.com"
}

mask() {
  sed -E -e "s/${ID}/[伏せ]/g" -e 's/[A-Za-z0-9.-]+\.r2\.cloudflarestorage\.com/[ホスト伏せ]/g'
}

fail_restore() {
  echo "::error::段 $STAGE: 鍵 $KEY の復元に失敗: $1。R2 の一時的な失敗かもしれない。ジョブを再実行する(決定 0391)"
  exit 1
}

clean_dir() {
  case "$DIR" in
    "$RUNNER_TEMP"/*) rm -rf "$DIR" ;;
    *) die "DIR が RUNNER_TEMP の下に無い(中止)" ;;
  esac
}

restore() {
  : "${STAGE:?}" "${KEY:?}" "${DIR:?}" "${RUNNER_TEMP:?}" "${WORK:?}"
  case "$KEY" in fl-"$STAGE"-[0-9a-f]*) ;; *) die "鍵の形が段 $STAGE と違う(中止)" ;; esac
  s3env
  mkdir -p "$WORK"
  P="stage-cache/$KEY"
  if ! aws s3api head-object --bucket "$CACHE_BUCKET" --key "$P/SHA256SUMS" --endpoint-url "$ENDPOINT" > "$WORK/h.json" 2> "$WORK/h.err" < /dev/null; then
    grep -qE '404|Not Found|NoSuchKey' "$WORK/h.err" || { echo "段 $STAGE の鍵の確認: 404 以外の失敗(伏せた末尾)"; mask < "$WORK/h.err" | tail -n 3; exit 1; }
    echo "段 $STAGE: R2 に鍵 $KEY が無い(404)。段を実行する"
    echo "hit=false" >> "$GITHUB_OUTPUT"
    exit 0
  fi
  for f in out.tar out.tar.sha256 SHA256SUMS; do
    aws s3api get-object --bucket "$CACHE_BUCKET" --key "$P/$f" --endpoint-url "$ENDPOINT" "$WORK/c-$f" > "$WORK/g.txt" 2>&1 < /dev/null \
      || { mask < "$WORK/g.txt" | tail -n 3; fail_restore "取り出しに失敗($f)"; }
  done
  want=$(awk '{print $1}' "$WORK/c-out.tar.sha256")
  got=$(sha256sum "$WORK/c-out.tar" | cut -d' ' -f1)
  [ -n "$want" ] && [ "$got" = "$want" ] || fail_restore "tar の SHA-256 が保存時と違う"
  python3 - "$WORK/c-out.tar" <<'PY' || fail_restore "tar の中身が想定外(絶対パス・..・リンク)"
import sys, tarfile
with tarfile.open(sys.argv[1]) as t:
    for m in t.getmembers():
        if m.name.startswith("/") or ".." in m.name.split("/") or not (m.isfile() or m.isdir()):
            sys.exit(1)
PY
  clean_dir
  mkdir -p "$DIR"
  tar -C "$DIR" -xf "$WORK/c-out.tar" || fail_restore "tar の展開に失敗"
  n=$(wc -l < "$WORK/c-SHA256SUMS" | tr -d ' ')
  f=$(cd "$DIR" && find . -type f | wc -l | tr -d ' ')
  [ "$n" = "$f" ] || fail_restore "件数が違う(記録 $n、展開 $f)"
  (cd "$DIR" && sha256sum -c "$WORK/c-SHA256SUMS" > "$WORK/check.txt" 2>&1) || fail_restore "保存時の SHA-256 と違うファイルがある"
  echo "段 $STAGE: R2 から復元した(${n} 件。保存時の SHA-256 と一致。鍵 $KEY)"
  echo "hit=true" >> "$GITHUB_OUTPUT"
}

save() {
  : "${STAGE:?}" "${KEY:?}" "${DIR:?}" "${RUNNER_TEMP:?}" "${WORK:?}"
  case "$KEY" in fl-"$STAGE"-[0-9a-f]*) ;; *) die "鍵の形が段 $STAGE と違う(中止)" ;; esac
  s3env
  mkdir -p "$WORK"
  P="stage-cache/$KEY"
  [ -d "$DIR" ] || die "段 $STAGE: 出力の場所が無い(保存しない)"
  [ -z "$(find "$DIR" -type l)" ] || die "段 $STAGE: 出力に記号リンクがある(保存しない)"
  (cd "$DIR" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) > "$WORK/SHA256SUMS" || die "段 $STAGE: SHA-256 の記録が作れない"
  [ -s "$WORK/SHA256SUMS" ] || die "段 $STAGE: 出力が空(保存しない)"
  tar -C "$DIR" -cf "$WORK/out.tar" . || die "段 $STAGE: tar が作れない"
  printf '%s  out.tar\n' "$(sha256sum "$WORK/out.tar" | cut -d' ' -f1)" > "$WORK/out.tar.sha256"
  put() {
    aws s3api put-object --bucket "$CACHE_BUCKET" --key "$P/$1" --body "$2" --content-type "$3" --endpoint-url "$ENDPOINT" > "$WORK/p.txt" 2>&1 < /dev/null \
      || { echo "保存: 書けない($1。伏せた末尾)"; mask < "$WORK/p.txt" | tail -n 3; exit 1; }
  }
  put out.tar "$WORK/out.tar" application/x-tar
  put out.tar.sha256 "$WORK/out.tar.sha256" text/plain
  put SHA256SUMS "$WORK/SHA256SUMS" text/plain
  aws s3api get-object --bucket "$CACHE_BUCKET" --key "$P/out.tar" --endpoint-url "$ENDPOINT" "$WORK/v-out.tar" > "$WORK/g.txt" 2>&1 < /dev/null \
    || { echo "保存の確認: tar を読めない(伏せた末尾)"; mask < "$WORK/g.txt" | tail -n 3; exit 1; }
  aws s3api get-object --bucket "$CACHE_BUCKET" --key "$P/SHA256SUMS" --endpoint-url "$ENDPOINT" "$WORK/v-SHA256SUMS" > "$WORK/g.txt" 2>&1 < /dev/null \
    || { echo "保存の確認: 記録を読めない(伏せた末尾)"; mask < "$WORK/g.txt" | tail -n 3; exit 1; }
  want=$(awk '{print $1}' "$WORK/out.tar.sha256")
  [ "$(sha256sum "$WORK/v-out.tar" | cut -d' ' -f1)" = "$want" ] || { echo "保存の確認: tar の SHA-256 が違う"; exit 1; }
  cmp -s "$WORK/v-SHA256SUMS" "$WORK/SHA256SUMS" || { echo "保存の確認: 記録が違う"; exit 1; }
  echo "段 $STAGE: 保存した。3 件を書き、読み直して SHA-256 を確かめた(鍵 $KEY。tar $(stat -c %s "$WORK/out.tar") B、件数 $(wc -l < "$WORK/SHA256SUMS" | tr -d ' '))"
}

case "${1:-}" in
  restore) restore ;;
  save) save ;;
  *) echo "usage: ci/stage-cache.sh restore|save" >&2; exit 2 ;;
esac
