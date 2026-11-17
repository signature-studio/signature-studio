#!/usr/bin/env bash
# usage: scripts/setup-repo.sh        (no argument; safe to run again)
# Repository-local git settings only: core.hooksPath=.githooks, user.name, user.email. Nothing global is touched.
# The identity is a neutral one (not an owner's, not a bot of the owner's): the commits are the agent's work.
set -o pipefail

die() { echo "setup-repo: $*" >&2; exit 1; }

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || die "cannot resolve the repository root"
cd "$root" || die "cannot enter $root"
git rev-parse --git-dir > /dev/null 2>&1 || die "$root is not a git repository"

git config --local core.hooksPath .githooks || die "cannot set core.hooksPath"
git config --local user.name "signature-studio agent" || die "cannot set user.name"
git config --local user.email "signature-studio-agent@users.noreply.github.com" || die "cannot set user.email"
chmod +x .githooks/pre-commit .githooks/pre-push 2> /dev/null || die "cannot make .githooks/pre-commit and .githooks/pre-push executable (missing?)"

echo "setup-repo: core.hooksPath=$(git config --local core.hooksPath) user.name=$(git config --local user.name) user.email=$(git config --local user.email)"
