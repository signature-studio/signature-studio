# R4-5e: one line per defined function of a wasm module from `wasm-objdump -d` (stdin): index, name (<...> when the module has one), instructions, memory.atomic.wait32, memory.atomic.notify, atomic cmpxchg, calls, calls to imports (index < the import count, given as -v nimp=N).
# Usage: wasm-objdump -d <module.wasm> | awk -v nimp=48 -f tools/verify-kit/tools/r45e-wasmfuncs.awk > <table.tsv>
BEGIN { OFS = "\t"; print "index", "name", "instr", "wait32", "notify", "cmpxchg", "calls", "import_calls"; cur = "" }
/^[0-9a-f]+ func\[[0-9]+\]/ {
  if (cur != "") emit()
  s = $0
  match(s, /func\[[0-9]+\]/)
  cur = substr(s, RSTART + 5, RLENGTH - 6)
  name = ""
  if (match(s, /<[^>]*>/)) name = substr(s, RSTART + 1, RLENGTH - 2)
  n = 0; w = 0; no = 0; cx = 0; c = 0; ic = 0
  next
}
cur != "" && /\| / {
  n++
  if ($0 ~ /memory\.atomic\.wait32/) w++
  if ($0 ~ /memory\.atomic\.notify/) no++
  if ($0 ~ /atomic\.rmw\.cmpxchg/) cx++
  if ($0 ~ /\| call /) {
    t = $0
    sub(/.*\| call /, "", t)
    split(t, a, " ")
    c++
    if (a[1] + 0 < nimp) ic++
  }
}
END { if (cur != "") emit() }
function emit() { print cur, name, n, w, no, cx, c, ic }
