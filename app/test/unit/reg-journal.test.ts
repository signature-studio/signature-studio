import { describe, expect, it } from 'vitest';
import { parseRecord, RegJournal, REG_OP } from '../../src/engine/reg-journal.ts';

// the records as lift/runtime/fle_reg.c writes them (reg_journal): u8 op | u32 key bytes | key | u32 name bytes | name | u32 type | u32 data bytes | data
const u16 = (s: string): number[] => {
  const out: number[] = [];
  for (const c of s) out.push(c.charCodeAt(0) & 0xff, c.charCodeAt(0) >> 8);
  return out;
};
const u32 = (v: number): number[] => [v & 0xff, (v >> 8) & 0xff, (v >> 16) & 0xff, (v >>> 24) & 0xff];
const rec = (op: number, key: string, name = '', type = 0, data: number[] = []): Uint8Array => {
  const k = u16(key), n = u16(name);
  return Uint8Array.from([op, ...u32(k.length), ...k, ...u32(n.length), ...n, ...u32(type), ...u32(data.length), ...data]);
};
const cat = (...parts: Uint8Array[]): Uint8Array => {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) { out.set(p, at); at += p.length; }
  return out;
};
const ops = (b: Uint8Array): number[] => {
  const out: number[] = [];
  for (let at = 0; at < b.length;) {
    const r = parseRecord(b, at);
    if (!r) throw new Error(`not a record at ${at}`);
    out.push(r.op);
    at = r.end;
  }
  return out;
};

describe('reg-journal: the record format', () => {
  it('reads a value record: its key, name, type and bytes', () => {
    const r = rec(REG_OP.value, 'Software\\Example', 'Sample', 1, [...u16('abc'), 0, 0]);
    const p = parseRecord(r, 0);
    expect(p).not.toBeNull();
    expect(p?.op).toBe(REG_OP.value);
    expect(p?.key).toBe('Software\\Example');
    expect(p?.name).toBe('Sample');
    expect(p?.type).toBe(1);
    expect(p?.end).toBe(r.length);
    expect(p?.bytes).toEqual(r);
  });

  it('refuses a record that is cut short, an unknown op and data on a key record', () => {
    const r = rec(REG_OP.value, 'A', 'b', 4, [1, 0, 0, 0]);
    expect(parseRecord(r.slice(0, r.length - 1), 0)).toBeNull();
    expect(parseRecord(Uint8Array.from([9, 0, 0, 0, 0]), 0)).toBeNull();
    expect(parseRecord(rec(REG_OP.key, 'A', '', 0, [1, 0]), 0)).toBeNull();
  });
});

describe('reg-journal: the state the records make', () => {
  it('keeps the last value of a name, without regard to ASCII case', () => {
    const j = new RegJournal();
    const first = rec(REG_OP.value, 'Software\\A', 'X', 4, [1, 0, 0, 0]);
    const last = rec(REG_OP.value, 'software\\a', 'x', 4, [2, 0, 0, 0]);
    j.load(cat(first, last));
    const img = j.image();
    expect(img).toEqual(last);
    expect(j.liveBytes).toBe(last.length);
  });

  it('keeps a deletion as a record, so that a seed value the deletion removed stays removed', () => {
    const j = new RegJournal();
    j.load(cat(rec(REG_OP.value, 'Software\\A', 'v', 1, [...u16('x'), 0, 0]), rec(REG_OP.delValue, 'Software\\A', 'v')));
    expect(ops(j.image())).toEqual([REG_OP.delValue]);
  });

  it('a deleted key takes what is under it out of the live set and keeps its own deletion', () => {
    const j = new RegJournal();
    j.load(cat(
      rec(REG_OP.key, 'A'),
      rec(REG_OP.key, 'A\\B'),
      rec(REG_OP.value, 'A\\B', 'v', 1, [0, 0]),
      rec(REG_OP.value, 'A', 'w', 1, [0, 0]),
      rec(REG_OP.delKey, 'A'),
    ));
    expect(ops(j.image())).toEqual([REG_OP.delKey]);
    expect(parseRecord(j.image(), 0)?.key).toBe('A');
  });

  it('a key or value keeps the place of its first record (keys are listed in the order they were made)', () => {
    const j = new RegJournal();
    const d2 = rec(REG_OP.value, 'K1', 'a', 1, [2, 0]);
    j.load(cat(rec(REG_OP.value, 'K1', 'a', 1, [1, 0]), rec(REG_OP.key, 'K2'), d2));
    const img = j.image();
    expect(ops(img)).toEqual([REG_OP.value, REG_OP.key]);
    expect(parseRecord(img, 0)?.bytes).toEqual(d2);
  });

  it('leaves out a record cut short at the end and says how many bytes that was', () => {
    const j = new RegJournal();
    const whole = rec(REG_OP.key, 'A');
    const part = rec(REG_OP.key, 'B').slice(0, 5);
    expect(j.load(cat(whole, part))).toEqual({ records: 1, dropped: 5 });
    expect(j.image()).toEqual(whole);
  });

  it('counts the live bytes as the image has them', () => {
    const j = new RegJournal();
    j.load(cat(rec(REG_OP.key, 'A'), rec(REG_OP.value, 'A', 'n', 1, [...u16('abc'), 0, 0]), rec(REG_OP.value, 'A', 'n', 1, [0, 0])));
    expect(j.liveBytes).toBe(j.image().length);
  });
});
