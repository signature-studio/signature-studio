#!/usr/bin/env python3
"""PROVISIONAL の一覧を lift/runtime から機械生成する(R7-1、ROADMAP L25)。
標準ライブラリだけ。環境変数・旗は読まない。

usage: python3 tools/provisional-census.py [ROOT [OUT]]
  ROOT 既定: <このリポジトリ>/lift/runtime
  OUT  既定: 標準出力(生成物は evidence/ に置く。docs/ には置かない)

走査: ROOT の下の *.c と *.h だけ(.inc は含めない)。行番号は 1 始まり(grep -n と同じ数え方)。
出力: TSV 1 行 1 件。列は file(ROOT からの相対パス)・line・decision・excerpt・deadline。
  decision: 同じ行の、決定番号らしい 0NNN の最初の一致。無ければ -。
  excerpt : 該当行の先頭 160 文字(タブ・CR は空白)。
  deadline: 同じ行と直後 2 行の中にある、期限の明示だけ。明示が無ければ 未設定。
    - until の直後の決定番号(0NNN)または日付(YYYY-MM-DD)。その文字列を入れる
    - deadline・due・expires・expiry・期限 の語から 40 文字以内の日付
    - by の直後の日付
    参照日・更新日・取得日は、上の語が無いので入れない。日付を推測で作らない。
行番号は実行時の値。lift/runtime が変わると変わる(仕様)。
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_ROOT = os.path.join(ROOT, "lift", "runtime")
MARK = "PROVISIONAL"
DATE = r"[0-9]{4}-[0-9]{2}-[0-9]{2}"
DECISION = re.compile(r"(?<![0-9A-Za-z_])0[0-9]{3}(?![0-9A-Za-z_])")
UNTIL = re.compile(r"\buntil\s+(?:decision\s+)?(?:0[0-9]{3}|" + DATE + r")(?![0-9])", re.I)
DATE_RE = re.compile(r"(?<![0-9])" + DATE + r"(?![0-9])")
DEADLINE_WORD = re.compile(r"\b(?:deadline|due|expires|expiry)\b|期限", re.I)
BY_DATE = re.compile(r"\bby\s+" + DATE + r"(?![0-9])", re.I)


def deadline_in(line):
    m = UNTIL.search(line)
    if m:
        return m.group(0)
    m = BY_DATE.search(line)
    if m:
        return re.search(DATE, m.group(0)).group(0)
    for m in DATE_RE.finditer(line):
        near = line[max(0, m.start() - 40):m.end() + 40]
        if DEADLINE_WORD.search(near):
            return m.group(0)
    return None


def deadline(lines, i):
    for j in range(i, min(i + 3, len(lines))):
        found = deadline_in(lines[j])
        if found:
            return found
    return "未設定"


def sources(root):
    found = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in filenames:
            if name.endswith(".c") or name.endswith(".h"):
                full = os.path.join(dirpath, name)
                found.append(os.path.relpath(full, root).replace(os.sep, "/"))
    return sorted(found)


def census(root):
    rows = []
    for rel in sources(root):
        with open(os.path.join(root, rel), encoding="utf-8", errors="replace", newline="") as f:
            text = f.read()
        lines = text.split("\n")
        if text.endswith("\n"):
            lines.pop()
        for i, s in enumerate(lines):
            if MARK not in s:
                continue
            m = DECISION.search(s)
            decision = m.group(0) if m else "-"
            excerpt = s.strip().replace("\t", " ").replace("\r", " ")[:160]
            rows.append((rel, i + 1, decision, excerpt, deadline(lines, i)))
    return rows


def main(argv):
    if len(argv) > 3:
        sys.stderr.write("usage: provisional-census.py [ROOT [OUT]]\n")
        return 2
    root = argv[1] if len(argv) > 1 else DEFAULT_ROOT
    out = ["file\tline\tdecision\texcerpt\tdeadline"]
    for row in census(root):
        out.append("\t".join(str(x) for x in row))
    data = ("\n".join(out) + "\n").encode("utf-8")
    if len(argv) > 2:
        with open(argv[2], "wb") as f:
            f.write(data)
    else:
        sys.stdout.buffer.write(data)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
