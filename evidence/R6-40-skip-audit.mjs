// R6-40 §6: counts the references that scripts/check-consistency.sh skips by a rule and that the git index does not have.
// It copies the rules of scripts/check-consistency.sh (same words, same sentence scope) and reads the index only.
// usage (repository root): node evidence/R6-40-skip-audit.mjs   -> prints the count per reason, then one line per reference.
import { execFileSync } from "node:child_process";
import path from "node:path";

const git = (...args) => execFileSync("git", args, { encoding: "utf8", maxBuffer: 1 << 28 });
const files = new Set(git("ls-files").split("\n").filter(Boolean));
const dirs = new Set();
for (const p of files) {
  const parts = p.split("/").slice(0, -1);
  while (parts.length) { dirs.add(parts.join("/")); parts.pop(); }
}
const top = new Set([...files].map((p) => p.split("/")[0]));
const gitignore = files.has(".gitignore") ? git("show", ":.gitignore") : "";
for (const line of gitignore.split("\n")) {
  const name = line.trim().replace(/^\/+/, "");
  if (name.endsWith("/") && !name.startsWith("#") && !name.startsWith("!") && name.length > 1 && !name.slice(0, -1).includes("/")) top.add(name.slice(0, -1));
}
const exists = (p) => files.has(p) || dirs.has(p);

const EXCLUDED = ["docs/archive/", "archive/", "evidence/", "patches/", "tools/", "lift/", "artifacts/", "binaries/"];
const OUTSIDE = ["~/work/handover", "~/.agents", "旧ツリー", "docs/archive"];
const PLANNED = ["ビルド", "作る", "作成", "出力", "ロールバック", "退避", "置く", "前に", "ls", "ロード", "build", "出来"];
const PLANNED_RE = PLANNED.map((w) => new RegExp("(?<![A-Za-z\\u30a0-\\u30ff])" + w.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + (/^[\x00-\x7f]+$/.test(w) ? "(?![A-Za-z])" : ""), "u"));
const CODE = /`[^`\n]*`|\]\([^)\s]*\)/g;
const BAD = /[<>*$…{}()\[\]|\\#~=?%]/;
const TICK = /`([^`\n]+)`/g;
const LINK = /\]\(([^)\s]+)\)/g;
const SUFFIX = /:\d[\d,\-]*$/;
const COMMIT = /^[0-9a-f]{7,40}\^?:\S+$/;

const isDoc = (p) => {
  if (EXCLUDED.some((e) => p.startsWith(e))) return false;
  if (["AGENTS.md", "README.md", "tests/harness/README.md"].includes(p)) return true;
  return p.endsWith(".md") && ["docs/", "app/", "prompts/"].some((d) => p.startsWith(d));
};

const counts = { BAD: 0, OUTSIDE: 0, PLANNED: 0, TOPNOT: 0, MISSING: 0, COMMIT: 0 };
const out = [];
const docs = [...files].filter(isDoc).sort();
for (const doc of docs) {
  const lines = git("show", ":" + doc).split("\n");
  lines.forEach((line, idx) => {
    const where = `${doc}:${idx + 1}`;
    if (doc === "docs/ROADMAP.md" && /\| R\d+-\d+ \| todo \|/.test(line)) return;
    const raws = [...[...line.matchAll(TICK)].map((m) => m[1]), ...[...line.matchAll(LINK)].map((m) => m[1]).filter((t) => !/^(http|#)/.test(t))];
    for (const raw of raws) {
      const words = raw.trim().includes(" ") ? raw.trim().split(/\s+/) : [raw];
      for (const w of words) {
        const sentence = [...line.split(/[。|]/)].filter((s) => s.includes(w)).join(" ") || line;
        const prose = sentence.replace(CODE, " ");
        const tok = w.replace(SUFFIX, "").trim();
        if (!tok || /^(-|\/|http|git@|#)/.test(tok)) continue;
        if (COMMIT.test(tok)) { counts.COMMIT++; continue; }
        if (BAD.test(tok)) { if (tok.includes("/")) { counts.BAD++; out.push(`BAD\t${where}\t${tok}`); } continue; }
        if (!tok.includes("/")) continue;
        if (OUTSIDE.some((o) => sentence.includes(o) || tok.includes(o))) { if (!exists(tok.replace(/\/+$/, ""))) { counts.OUTSIDE++; out.push(`OUTSIDE\t${where}\t${tok}`); } continue; }
        if (PLANNED_RE.some((r) => r.test(prose))) { if (!exists(tok.replace(/\/+$/, ""))) { counts.PLANNED++; out.push(`PLANNED\t${where}\t${tok}`); } continue; }
        if (!top.has(tok.split("/")[0])) { if (!exists(tok.replace(/\/+$/, ""))) { counts.TOPNOT++; out.push(`TOPNOT\t${where}\t${tok}`); } continue; }
        const here = path.posix.normalize(path.posix.join(path.posix.dirname(doc), tok.replace(/\/+$/, "")));
        if (!exists(tok.replace(/\/+$/, "")) && !exists(here)) { counts.MISSING++; out.push(`MISSING\t${where}\t${tok}`); }
      }
    }
  });
}
console.log(JSON.stringify(counts));
console.log(out.join("\n"));
