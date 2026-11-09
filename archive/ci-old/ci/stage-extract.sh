#!/bin/bash
# ci/stage-extract.sh fl|vst3 - the stage "fetch + extract": the installers come from the ARTIFACT R2 bucket (or CI_INSTALLER_DIR for a local run), are verified against ci/artifacts.lock, and the
# product files are extracted from them. Outputs: $HOST_ROOT (fl: binaries/win64, lift/out-*; what app/scripts/build-engine-bundle.sh reads as HOST_ROOT) and $CI_WORK/vst3 (vst3).
# Only the stage's own cache key decides whether this runs at all: on a hit the installers are not downloaded.
#   env (fetch from R2): ARTIFACT_S3_ENDPOINT=https://<account>.r2.cloudflarestorage.com ARTIFACT_BUCKET=fl-web-artifacts <CREDENTIAL-NAME> <CREDENTIAL-NAME> (read-only token)
#   env (local):         CI_INSTALLER_DIR=<directory that has the installers under their file names>
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
WHAT=${1:-}
DL=$CI_WORK/dl; mkdir -p "$DL"
fetch() {   # fetch KEY -> prints the local path; verifies size and SHA-256 against ci/artifacts.lock
  local key=$1 size sha etag base path want_etag
  IFS=$'\t' read -r _ size sha want_etag _ < <(awk -F'\t' -v k="$key" '$1==k' "$CI_DIR/artifacts.lock")
  [ -n "${sha:-}" ] || die "$key is not in ci/artifacts.lock"
  base=$(basename "$key")
  if [ -n "${CI_INSTALLER_DIR:-}" ]; then
    path=$CI_INSTALLER_DIR/$base
  else
    : "${ARTIFACT_S3_ENDPOINT:?}" "${ARTIFACT_BUCKET:?}" "${<CREDENTIAL-NAME>:?}" "${<CREDENTIAL-NAME>:?}"
    path=$DL/$base
    export AWS_DEFAULT_REGION=auto AWS_EC2_METADATA_DISABLED=true AWS_MAX_ATTEMPTS=5
    etag=$(aws s3api head-object --bucket "$ARTIFACT_BUCKET" --key "$key" --endpoint-url "$ARTIFACT_S3_ENDPOINT" --query ETag --output text | tr -d '"')
    [ "$etag" = "$want_etag" ] || die "$key: the bucket's ETag is $etag, ci/artifacts.lock says $want_etag"
    log "downloading $key ($size bytes)"
    aws s3 cp "s3://$ARTIFACT_BUCKET/$key" "$path" --endpoint-url "$ARTIFACT_S3_ENDPOINT" --only-show-errors || die "download of $key failed"
  fi
  [ "$(stat -L -c %s "$path")" = "$size" ] || die "$base: size $(stat -L -c %s "$path"), expected $size"
  [ "$(sha256sum "$path" | cut -d' ' -f1)" = "$sha" ] || die "$base: SHA-256 differs from ci/artifacts.lock"
  echo "$path"
}
t0=$SECONDS
case "$WHAT" in
  fl)
    inst=$(fetch installers/flstudio_win64_26.1.7.5653.exe) || exit 2
    rm -rf "$HOST_ROOT"; mkdir -p "$HOST_ROOT"
    python3 "$CI_DIR/lib/extract_fl.py" "$inst" "$HOST_ROOT" --dlls "$CI_DIR/extract-dlls.tsv" --seed "$REPO/lift/harness/seeds/vfs-seed-idle1.tsv" --manifest "$CI_WORK/extract-fl.sha256" || die "extraction failed"
    log "extract identity (SHA-256 of the file manifest): $(sha256sum "$CI_WORK/extract-fl.sha256" | cut -d' ' -f1)"
    [ -z "${CI_INSTALLER_DIR:-}" ] && rm -f "$inst"
    ;;
  vst3)
    rm -rf "$CI_WORK/vst3"; mkdir -p "$CI_WORK/vst3"
    for k in installers/Install_Xfer_Serum2_2.1.5.exe installers/Install_Xfer_OTT_137.exe installers/VitalInstaller.exe; do
      p=$(fetch "$k") || exit 2; bash "$CI_DIR/lib/extract_vst3.sh" "$p" "$CI_WORK/vst3" || die "extraction from $k failed"
      [ -z "${CI_INSTALLER_DIR:-}" ] && rm -f "$p"
    done
    ;;
  *) die "usage: stage-extract.sh fl|vst3" ;;
esac
log "extract $WHAT done in $((SECONDS - t0)) s"
