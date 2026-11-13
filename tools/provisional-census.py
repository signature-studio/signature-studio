#!/usr/bin/env python3
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUNTIME = os.path.join(ROOT, "lift", "runtime")
MARK = "PROVISIONAL"
DECISION = re.compile(r"(?<![0-9])0[0-9]{3}(?![0-9])")
DATE = re.compile(r"(?<![0-9])[0-9]{4}-[0-9]{2}-[0-9]{2}(?![0-9])")
DEADLINE_WORD = re.compile(r"deadline|due|until|expires|expiry|期限", re.I)
BY_BEFORE = re.compile(r"\bby\s+$", re.I)


def comment_flags(lines):
    flags = []
    in_comment = False
    for s in lines:
        t = s.strip()
        flag = in_comment or t.startswith("//") or "/*" in s
        if in_comment:
            if "*/" in s:
                in_comment = False
        elif "/*" in s and "*/" not in s[s.index("/*"):]:
            in_comment = True
        flags.append(flag)
    return flags


def deadline(lines, flags, i):
    if flags[i]:
        a = i
        while a > 0 and flags[a - 1]:
            a -= 1
        b = i
        while b + 1 < len(lines) and flags[b + 1]:
            b += 1
        span = range(a, b + 1)
    elif i > 0 and flags[i - 1]:
        a = i - 1
        while a > 0 and flags[a - 1]:
            a -= 1
        span = range(a, i + 1)
    else:
        span = range(i, i + 1)
    for j in span:
        line = lines[j]
        for m in DATE.finditer(line):
            near = line[max(0, m.start() - 40):m.end() + 40]
            if DEADLINE_WORD.search(near) or BY_BEFORE.search(line[:m.start()]):
                return m.group(0)
    return "未設定"


def sources():
    found = []
    for dirpath, dirnames, filenames in os.walk(RUNTIME):
        dirnames.sort()
        for name in filenames:
            if name.endswith(".c") or name.endswith(".h"):
                full = os.path.join(dirpath, name)
                found.append(os.path.relpath(full, RUNTIME).replace(os.sep, "/"))
    return sorted(found)


def main():
    rows = []
    for rel in sources():
        with open(os.path.join(RUNTIME, rel), encoding="utf-8", errors="replace", newline="") as f:
            text = f.read()
        lines = text.split("\n")
        if text.endswith("\n"):
            lines.pop()
        flags = comment_flags(lines)
        for i, s in enumerate(lines):
            if MARK not in s:
                continue
            m = DECISION.search(s)
            decision = m.group(0) if m else "-"
            excerpt = s.strip().replace("\t", " ").replace("\r", " ")[:160]
            rows.append((rel, i + 1, decision, excerpt, deadline(lines, flags, i)))
    out = ["file\tline\tdecision\texcerpt\tdeadline"]
    for row in rows:
        out.append("\t".join(str(x) for x in row))
    sys.stdout.buffer.write(("\n".join(out) + "\n").encode("utf-8"))


if __name__ == "__main__":
    main()
