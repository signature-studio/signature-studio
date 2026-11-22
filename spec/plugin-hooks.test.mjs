#!/usr/bin/env node
// spec/plugin-hooks.test.mjs : tools/plugin-hooks/detect.mjs の数え方((組)と(候補))を、手で作った小さなバイト列で確かめ、表 inputs/plugin-hooks.tsv の形を確かめる。実物の DLL は使わない。
// 終了コード 0 = 全部通った。
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TOOL = path.join(ROOT, 'tools/plugin-hooks/detect.mjs');
const TABLE = path.join(ROOT, 'inputs/plugin-hooks.tsv');
const HEADER = ['パス', 'hook 1 RVA', 'hook 2 RVA', 'hook 3 RVA', 'hook 1 先頭 5 バイト', 'hook 2 先頭 5 バイト', 'hook 3 先頭 5 バイト', '検出の根拠', '確信', '備考'];
const ROWS = 132;
let failed = 0;
const ok = (name, got, want) => {
  if (JSON.stringify(got) === JSON.stringify(want)) console.log(`ok   ${name}`);
  else {
    failed++;
    console.log(`FAIL ${name}: expected ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
  }
};

let m;
try {
  m = await import(pathToFileURL(TOOL).href);
} catch (e) {
  console.log(`FAIL import: ${e.message}`);
  process.exit(1);
}

// 手で作る節の中身。命令の位置は RVA で数え、rip 相対の d32 は「先の RVA − 命令の次の RVA」にする。
const SEC_VA = 0x1000;
const SEC = { va: SEC_VA, ptr: 0, raw: 0, chars: 0x2000_0000 };
const i32 = (v) => {
  const b = Buffer.alloc(4);
  b.writeInt32LE(v);
  return [...b];
};
class Asm {
  constructor() {
    this.b = [];
  }
  get rva() {
    return SEC_VA + this.b.length;
  }
  bytes(...xs) {
    this.b.push(...xs);
    return this;
  }
  lea8(t) {
    const at = this.rva;
    return this.bytes(0x4c, 0x8d, 0x05, ...i32(t - (at + 7)));
  }
  lea9(t) {
    const at = this.rva;
    return this.bytes(0x4c, 0x8d, 0x0d, ...i32(t - (at + 7)));
  }
  call(t) {
    const at = this.rva;
    return this.bytes(0xe8, ...i32(t - (at + 5)));
  }
  nop(n) {
    return this.bytes(...new Array(n).fill(0x90));
  }
  // Balance の形: lea r8, lea r9, mov byte [rsp+0x20],0, call
  triple(target, replacement, callee) {
    return this.lea8(target).lea9(replacement).bytes(0xc6, 0x44, 0x24, 0x20, 0x00).call(callee);
  }
  // rva(s) of the triplets, grouped by callee: [[callee, [r8...]]] with the candidates (exactly three) only
  candidates(imageBase = 0x10000000) {
    const buf = Buffer.from(this.b);
    const list = m.triplets({ d: buf, imageBase }, [{ ...SEC, raw: buf.length }]);
    return [...m.groupByCallee(list)].filter(([, v]) => v.length === 3).map(([c, v]) => [c, v.map((t) => t.r8)]);
  }
  counts(imageBase = 0x10000000) {
    const buf = Buffer.from(this.b);
    const list = m.triplets({ d: buf, imageBase }, [{ ...SEC, raw: buf.length }]);
    return list.length;
  }
}

const T1 = 0x17bc0, T2 = 0x31d70, T3 = 0x31d90, R1 = 0x5bd4b0, R2 = 0x5bd3a0, R3 = 0x5bd410;
const C = 0x1c2d70, D = 0x1c2c90;

// (組)(候補): three triplets of one callee, given out of order, are one candidate sorted by the target
ok('3 組を 1 候補にし、対象を昇順に並べる', new Asm().triple(T3, R3, C).nop(40).triple(T1, R1, C).nop(40).triple(T2, R2, C).candidates(), [[C, [T1, T2, T3]]]);
// two triplets, or four, are not a candidate
ok('2 組は候補にならない', new Asm().triple(T1, R1, C).nop(40).triple(T2, R2, C).candidates(), []);
ok('4 組は候補にならない', new Asm().triple(T1, R1, C).nop(40).triple(T2, R2, C).nop(40).triple(T3, R3, C).nop(40).triple(T1, R1, C).candidates(), []);
// a callee with three triplets is a candidate; another callee with one triplet is not
ok('別の呼び出し先の 1 組は候補にならない', new Asm().triple(T1, R1, C).nop(40).triple(T2, R2, C).nop(40).triple(T3, R3, C).nop(40).triple(T1, R1, D).candidates(), [[C, [T1, T2, T3]]]);
// the window of lea r9: 16 bytes after the end of lea r8 is in, 17 is out
ok('lea r9 は lea r8 の終わりから 16 バイトまで含む', new Asm().lea8(T1).nop(16).lea9(R1).call(C).counts(), 1);
ok('lea r9 が 17 バイト先なら数えない', new Asm().lea8(T1).nop(17).lea9(R1).call(C).counts(), 0);
// the window of call: 48 bytes from the start of lea r8 is in, 49 is out (lea r8 and lea r9 take 14 bytes)
ok('call は lea r8 の先頭から 48 バイトまで含む', new Asm().lea8(T1).lea9(R1).nop(34).call(C).counts(), 1);
ok('call が 49 バイト先なら数えない', new Asm().lea8(T1).lea9(R1).nop(35).call(C).counts(), 0);
// lea r8 without a lea r9 after it is not a triplet
ok('lea r9 の無い lea r8 は数えない', new Asm().lea8(T1).nop(20).call(C).counts(), 0);
// the RVAs do not depend on the ImageBase: the header value is used for the VA, and the RVA is VA minus ImageBase
ok('ImageBase が違っても RVA は同じ(0x400000)', new Asm().triple(T1, R1, C).nop(40).triple(T2, R2, C).nop(40).triple(T3, R3, C).candidates(0x400000), [[C, [T1, T2, T3]]]);
ok('ImageBase が違っても RVA は同じ(0x10000000)', new Asm().triple(T1, R1, C).nop(40).triple(T2, R2, C).nop(40).triple(T3, R3, C).candidates(0x10000000), [[C, [T1, T2, T3]]]);

// (表) inputs/plugin-hooks.tsv: the header, the rows (one per DLL), the columns, and the confidence values
const text = fs.existsSync(TABLE) ? fs.readFileSync(TABLE, 'utf8') : '';
const lines = text.split('\n');
if (lines.at(-1) === '') lines.pop();
const [head, ...body] = lines.map((l) => l.split('\t'));
ok('表の見出し', head, HEADER);
ok('表の行数', body.length, ROWS);
ok('表の列数', body.every((r) => r.length === HEADER.length), true);
ok('確信は 高・中・低 のどれか', body.every((r) => ['高', '中', '低'].includes(r[8])), true);
ok('確信が 高・中 の行は hook 点の 3 つを持つ', body.filter((r) => r[8] !== '低').every((r) => r.slice(1, 4).every((v) => /^0x[0-9a-f]+$/.test(v)) && r.slice(4, 7).every((v) => /^[0-9a-f]{2}( [0-9a-f]{2}){4}$/.test(v))), true);
ok('確信が 低 の行は hook 点を空にする', body.filter((r) => r[8] === '低').every((r) => r.slice(1, 7).every((v) => v === '')), true);

if (failed) console.log(`plugin-hooks test: ${failed} 件が失敗`);
else console.log('plugin-hooks test: 全部通った');
process.exit(failed ? 1 : 0);
