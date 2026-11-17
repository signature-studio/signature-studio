#!/bin/bash
# ci/agents/install-opencode.sh - installs opencode OPENCODE_VERSION from its npm tarball (the file `opencode run` comes from) and checks the tarball against the sha512 pinned below
# (the registry's own `dist.integrity`, read 2026-11-08). No `curl | bash`. To change the version, change the three constants together.
set -eu -o pipefail
OPENCODE_VERSION=2.2.5
INTEGRITY_X64='sha512-BkHAfTqNsg/qphyHNrnH+DGkOxZsGjamNpM5DvwoEvz7ZAPNw8mEbMXGTCDOAH+/+40Eqeie6KNZBZZQwPCjaA=='
INTEGRITY_X64_BASELINE='sha512-oPOT+EMmdz56HnSFPRVwZ0QTVL5defhOacnwzazu0XOuCWUtEKh8icIn323SdCLtqZtzjuU1mt6p2/13V93WWg=='
if grep -qwi avx2 /proc/cpuinfo; then target=x64; want=$INTEGRITY_X64; else target=x64-baseline; want=$INTEGRITY_X64_BASELINE; fi
dest=${OPENCODE_INSTALL_DIR:-$HOME/.opencode/bin}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
url="https://registry.npmjs.org/@opencode/cli-linux-$target/-/cli-linux-$target-$OPENCODE_VERSION.tgz"
curl -fsSL --retry 3 -o "$tmp/cli.tgz" "$url"
got="sha512-$(openssl dgst -sha512 -binary "$tmp/cli.tgz" | base64 -w0)"
[ "$got" = "$want" ] || { echo "::error::sha512 of $url is $got, expected $want"; exit 1; }
tar -xzf "$tmp/cli.tgz" -C "$tmp"
mkdir -p "$dest"
install -m 755 "$tmp/package/bin/opencode" "$dest/opencode"
[ -z "${GITHUB_PATH:-}" ] || echo "$dest" >> "$GITHUB_PATH"
echo "opencode $OPENCODE_VERSION ($target) installed in $dest, sha512 matches"
"$dest/opencode" --version
