#!/usr/bin/env bash
# R4-5e (0031): the search of "the same type" in the tree of one commit. git grep reads the commit, not the working tree, so the result is the same whenever it is run. Usage: bash tools/verify-kit/tools/r45e-search0031.sh <commit>. Prints the commands, their output in full, and the counts.
set -u
c=$(git rev-parse --verify "${1:?usage: r45e-search0031.sh <commit>}^{commit}") || exit 2
echo "# commit $c"
echo "## 1. files with GIL (docs/ and evidence/, *.md, archive excluded)"
echo "\$ git grep -l -e GIL $c -- 'docs/*.md' 'evidence/*.md' | grep -v '/archive/'"
git grep -l -e GIL "$c" -- 'docs/*.md' 'evidence/*.md' | grep -v '/archive/' | sed "s#^$c:##"
echo "count: $(git grep -l -e GIL "$c" -- 'docs/*.md' 'evidence/*.md' | grep -v '/archive/' | wc -l)"
echo
echo "## 2. GIL lines with 同時|並行|一度に|1 スレッド|1 本しか|同じ時 (archive excluded)"
echo "\$ git grep -n -e GIL $c -- 'docs/*.md' 'evidence/*.md' | grep -v '/archive/' | grep -E '同時|並行|一度に|1 スレッド|1 本しか|同じ時'"
git grep -n -e GIL "$c" -- 'docs/*.md' 'evidence/*.md' | grep -v '/archive/' | grep -E '同時|並行|一度に|1 スレッド|1 本しか|同じ時'
echo "count: $(git grep -n -e GIL "$c" -- 'docs/*.md' 'evidence/*.md' | grep -v '/archive/' | grep -E '同時|並行|一度に|1 スレッド|1 本しか|同じ時' | wc -l)"
echo
echo "## 3. the old sentence 同時に 1 スレッド (no filter: archive included)"
echo "\$ git grep -n '同時に 1 スレッド' $c -- docs evidence scripts tools"
git grep -n "同時に 1 スレッド" "$c" -- docs evidence scripts tools
echo "count: $(git grep -n "同時に 1 スレッド" "$c" -- docs evidence scripts tools | wc -l)"
