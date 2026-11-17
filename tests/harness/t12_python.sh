#!/usr/bin/env bash
# t12: the python rule of scripts/scan-forbidden.sh (the standing order of 2026-11-17, AGENTS.md §6: no Python in the production path or the product, except investigation and verification).
#  (a) in the production scope a line that runs python or python3 (any letter case; a path such as /usr/bin/python3; a version such as python3.11; a backslash at the end of the line)
#      is a finding when the word is followed by a character that is not part of a name, or by the end of the line, or is quoted; a path that names a .py file is a finding in a file
#      that is not data; a .py name that starts a line is a finding in a script or a configuration; a .py file is a finding;
#  (b) a comment line, a Markdown file, a path outside the production scope, a data file (.txt) that names a .py file, a C member named py, a C string that holds python\\,
#      a host name (docs.python.org), an identifier that holds python (pythonic, PYTHONPATH_OK), and a verification tool named by an allowlist directory entry are not findings;
#  (c) a deploy input directory (lift/harness/web) is not exempt by a directory entry;
#  (d) the tracked production scope of this repository has no finding of any rule, and the scan exits 0 there.
#  The workflows: the python lines of .github/workflows are checked by (a). A script that a workflow calls is not followed here: the allowlist names each of them line by line (evidence/R6-37.md §6).
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"
set -o pipefail

tab=$'\t'
t=$(new_sandbox t12)/tree
mkdir -p "$t/.github/workflows" "$t/ci" "$t/app/src" "$t/app/scripts" "$t/app/test/unit" "$t/lift/harness/web" "$t/lift/harness/parity" "$t/lift/crates/fe-lift/src" "$t/lift/runtime" "$t/tests" "$t/docs" "$t/tools"
cat > "$t/.github/workflows/bad.yml" <<'EOF'
jobs:
  x:
    steps:
      - run: python3 ci/lib/tool.py --go
EOF
cat > "$t/ci/comment-only.sh" <<'EOF'
#!/usr/bin/env bash
# python3 was used before, not any more
echo ok
EOF
cat > "$t/ci/delimiter.sh" <<'EOF'
echo ok; python3; echo done
EOF
cat > "$t/ci/Makefile" <<'EOF'
build:
	python3 tools/x.py
EOF
cat > "$t/ci/run-a-py.sh" <<'EOF'
./tools/measure.py --go
EOF
cat > "$t/ci/new-tool.py" <<'EOF'
print("a new python file in the production scope")
EOF
cat > "$t/ci/data.txt" <<'EOF'
Made by tools/measure.py (see README).
EOF
cat > "$t/app/src/bad.ts" <<'EOF'
const r = spawnSync('python3', [x]);
EOF
cat > "$t/app/src/data-name.ts" <<'EOF'
const v = opts.py ?? 0;
EOF
cat > "$t/app/scripts/run.mjs" <<'EOF'
tools/x.py --go
EOF
cat > "$t/app/config.json" <<'EOF'
{ "run": "tools/x.py --go" }
EOF
cat > "$t/ci/usr-bin.sh" <<'EOF'
/usr/bin/python3 ci/tool.sh --go
EOF
cat > "$t/ci/version.sh" <<'EOF'
python3.11 ci/tool.sh --go
EOF
cat > "$t/ci/continued.sh" <<'EOF'
run: python3\
  ci/tool.sh
EOF
cat > "$t/ci/upper.sh" <<'EOF'
PYTHON3 ci/tool.sh
EOF
cat > "$t/ci/shebang-tool" <<'EOF'
#!/usr/bin/env python3
print("a shebang that runs python in a file without an extension")
EOF
cat > "$t/ci/shebang-sh" <<'EOF'
#!/usr/bin/env bash
echo ok
EOF
cat > "$t/app/src/host.ts" <<'EOF'
const docs = "https://docs.python.org/3/";
const pythonic = pythonicName + PYTHONPATH_OK;
EOF
cat > "$t/app/test/unit/verification-test.sh" <<'EOF'
python3 ../tools/x.py
EOF
cat > "$t/lift/harness/web/verification.sh" <<'EOF'
python3 "$HERE/check.py" "$OUT"
EOF
cat > "$t/lift/harness/parity/verification.sh" <<'EOF'
python3 "$HERE/check.py" "$OUT"
EOF
cat > "$t/lift/harness/parity/measure.py" <<'EOF'
print("a verification tool named by a directory entry")
EOF
cat > "$t/lift/crates/fe-lift/src/comment.rs" <<'EOF'
/// the reference is python3 based (see lift/harness/parity/measure.py)
fn f() {}
EOF
cat > "$t/lift/runtime/struct-member.c" <<'EOF'
    tab[idx].py = (int32_t)y;
    static const char *const absent[] = { "python\\python312.dll" };
EOF
cat > "$t/docs/note.md" <<'EOF'
Run `python3 tools/x.py` by hand.
EOF
cat > "$t/tests/outside-scope.sh" <<'EOF'
python3 tools/y.py
EOF
cat > "$t/tools/outside.py" <<'EOF'
print("outside the production scope")
EOF

scan_out=$(cd "$t" && bash "$ROOT/scripts/scan-forbidden.sh" . 2> /dev/null); rc=$?
expect_eq "the scan exits 1 on the synthetic violations" "1" "$rc"
expect_contains "a workflow that runs python3 is a finding" "$scan_out" "python${tab}.github/workflows/bad.yml:4"
expect_contains "python3 followed by a semicolon is a finding (the word ends at a delimiter)" "$scan_out" "python${tab}ci/delimiter.sh:1"
expect_contains "python3 at a tab-indented command of a Makefile is a finding" "$scan_out" "python${tab}ci/Makefile:2"
expect_contains "app code that names the python3 command is a finding" "$scan_out" "python${tab}app/src/bad.ts:1"
expect_contains "a .py named at the start of a line in a script is a finding" "$scan_out" "python${tab}ci/run-a-py.sh:1"
expect_contains "a .py named at the start of a line in an .mjs file is a finding" "$scan_out" "python${tab}app/scripts/run.mjs:1"
expect_contains "a .py path in a .json configuration is a finding" "$scan_out" "python${tab}app/config.json:1"
expect_contains "a .py file in the production scope is a finding (not named in the allowlist)" "$scan_out" "python${tab}ci/new-tool.py:0"
expect_contains "a path such as /usr/bin/python3 is a finding" "$scan_out" "python${tab}ci/usr-bin.sh:1"
expect_contains "python3.11 (a version after the word) is a finding" "$scan_out" "python${tab}ci/version.sh:1"
expect_contains "python3 at the end of a line with a backslash (a continued command) is a finding" "$scan_out" "python${tab}ci/continued.sh:1"
expect_contains "the word in upper case (PYTHON3) is a finding" "$scan_out" "python${tab}ci/upper.sh:1"
expect_contains "a shebang that runs python3 (#!/usr/bin/env python3) is a finding" "$scan_out" "python${tab}ci/shebang-tool:1"
expect_not_contains "a shebang that runs bash is not a finding" "$scan_out" "python${tab}ci/shebang-sh:1"
expect_not_contains "a host name (docs.python.org) is not a finding" "$scan_out" "python${tab}app/src/host.ts:1"
expect_not_contains "identifiers that hold python (pythonic, PYTHONPATH_OK) are not findings" "$scan_out" "python${tab}app/src/host.ts:2"
expect_contains "a deploy input directory is not exempt: python3 in lift/harness/web is a finding" "$scan_out" "python${tab}lift/harness/web/verification.sh:1"
expect_contains "a test file under app/test is in the scope and is a finding" "$scan_out" "python${tab}app/test/unit/verification-test.sh:1"
expect_not_contains "a comment line is not a finding" "$scan_out" "python${tab}ci/comment-only.sh"
expect_not_contains "a verification directory named by a directory entry (lift/harness/parity) is not a finding" "$scan_out" "python${tab}lift/harness/parity/verification.sh"
expect_not_contains "a .py file of a verification directory named by a directory entry is not a finding" "$scan_out" "python${tab}lift/harness/parity/measure.py"
expect_not_contains "a .py word in a TypeScript data name is not a finding" "$scan_out" "python${tab}app/src/data-name.ts"
expect_not_contains "a Rust comment that names a .py file is not a finding" "$scan_out" "python${tab}lift/crates/fe-lift/src/comment.rs"
expect_not_contains "a data file (.txt) that names a .py file is not a finding" "$scan_out" "python${tab}ci/data.txt"
expect_not_contains "a C struct member named py and a C string with python\\ are not findings" "$scan_out" "python${tab}lift/runtime/struct-member.c"
expect_not_contains "a Markdown file is not a finding" "$scan_out" "python${tab}docs/note.md"
expect_not_contains "a path outside the production scope (tests/, tools/) is not a finding" "$scan_out" "python${tab}tests/outside-scope.sh"
expect_not_contains "a .py file outside the production scope (tools/) is not a finding" "$scan_out" "python${tab}tools/outside.py"

# the tracked files of the production scope (an ignored output directory such as lift/out-* is not part of the repository): every rule, and the exit status
real_err="$t/real.err"
real=$(cd "$ROOT" && git ls-files -z .github ci app scripts lift | tr '\0' '\n' | grep -v -E '\.md$' | tr '\n' '\0' | xargs -0 bash scripts/scan-forbidden.sh 2> "$real_err"); real_rc=$?
expect_eq "the tracked production scope of this repository: scan-forbidden.sh exits 0 (no finding of any rule)" "0" "$real_rc"
expect_eq "the scan of the tracked production scope printed no error" "0" "$(grep -c . "$real_err")"
expect_eq "the tracked production scope of this repository has no python finding (count)" "0" "$(printf '%s\n' "$real" | grep -c "^python${tab}")"

finish
