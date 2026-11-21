#!/usr/bin/env node
// tools/task-accept.mjs : タスクの本文(標準入力)から、受け入れコマンドを取り出し、「accept: <コマンド>」の 1 行を標準出力に出す。
// 本文に ```accept のコードブロックが 1 つあり、中身が 1 行のコマンドのときだけ成功する(0)。それ以外は、理由を標準エラーに出して 1 で終わる。
import { readFileSync } from 'node:fs';

const lines = readFileSync(0, 'utf8').replace(/\r/g, '').split('\n');
const fence = /^```accept[ \t]*$/;
const close = /^```[ \t]*$/;
const blocks = [];
for (let i = 0; i < lines.length; i++) {
  if (!fence.test(lines[i])) continue;
  let j = i + 1;
  while (j < lines.length && !close.test(lines[j])) j++;
  blocks.push(j < lines.length ? lines.slice(i + 1, j) : null);
  i = j;
}

function fail(message) {
  process.stderr.write(`task-accept: ${message}\n`);
  process.exit(1);
}

if (blocks.length === 0) fail('accept のブロックが無い(本文に、accept と書いたコードブロックを 1 つ置く)');
if (blocks.length > 1) fail(`accept のブロックが ${blocks.length} 個ある(1 つだけにする)`);
if (blocks[0] === null) fail('accept のブロックが閉じていない');
const cmds = blocks[0].map((l) => l.trim()).filter((l) => l !== '');
if (cmds.length !== 1) fail(`accept のブロックの中身が ${cmds.length} 行(1 行のコマンドにする)`);
process.stdout.write(`accept: ${cmds[0]}\n`);
