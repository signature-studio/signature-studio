import { describe, expect, it } from 'vitest';
import { fileRecord, RECENT_KEY, RECENT_MAX, recentAfterOpen, recentListOf, recordBytes, utf16Bytes } from '../../src/engine/recent.ts';
import { parseRecord, RegJournal, type RegRecord, REG_OP } from '../../src/engine/reg-journal.ts';

const whole = (b: Uint8Array): RegRecord => {
  const r = parseRecord(b, 0);
  if (r === null) throw new Error('not a record');
  return r;
};
const journalOf = (records: Uint8Array[]): RegJournal => {
  const j = new RegJournal();
  for (const r of records) j.add(whole(r));
  return j;
};

describe('recent: the records of the journal', () => {
  it('writes a value record that the journal reads back (key, name, type and data)', () => {
    const r = recordBytes(REG_OP.value, RECENT_KEY, '1', 1, utf16Bytes('C:\\a.flp\u0000'));
    const p = whole(r);
    expect(p.op).toBe(REG_OP.value);
    expect(p.key).toBe(RECENT_KEY);
    expect(p.name).toBe('1');
    expect(p.type).toBe(1);
    expect(p.end).toBe(r.length);
  });

  it('writes a file record (op 5): the path is the key, the content is the data', () => {
    const p = whole(fileRecord('C:\\Users\\user\\Downloads\\x.flp', Uint8Array.from([1, 2, 3, 4])));
    expect(p.op).toBe(REG_OP.filePut);
    expect(p.key).toBe('C:\\Users\\user\\Downloads\\x.flp');
    expect(p.name).toBe('');
    expect(Array.from(p.bytes.subarray(p.bytes.length - 4))).toEqual([1, 2, 3, 4]);
  });
});

describe('recent: the list FL reads', () => {
  it('reads the opened paths back, newest first, from the saved records', () => {
    const records: Uint8Array[] = [];
    let list: string[] = [];
    for (const path of ['C:\\a.flp', 'C:\\b.flp', 'C:\\c.flp']) {
      const next = recentAfterOpen(list, path);
      records.push(...next.records);
      list = next.list;
    }
    expect(recentListOf(journalOf(records).image())).toEqual(['C:\\c.flp', 'C:\\b.flp', 'C:\\a.flp']);
  });

  it('a path that is listed again moves to the top, and the list is capped', () => {
    let list: string[] = [];
    for (let i = 0; i < RECENT_MAX + 5; i++) list = recentAfterOpen(list, `C:\\p${i}.flp`).list;
    expect(list.length).toBe(RECENT_MAX);
    expect(list[0]).toBe(`C:\\p${RECENT_MAX + 4}.flp`);
    const again = recentAfterOpen(list, `C:\\P${RECENT_MAX + 3}.FLP`);
    expect(again.list[0]).toBe(`C:\\P${RECENT_MAX + 3}.FLP`);
    expect(again.list.length).toBe(RECENT_MAX);
    expect(again.list.filter((p) => p.toLowerCase() === `c:\\p${RECENT_MAX + 3}.flp`).length).toBe(1);
  });

  it('writes only the values whose text changed', () => {
    const first = recentAfterOpen([], 'C:\\a.flp');
    expect(first.records.length).toBe(1);
    const second = recentAfterOpen(first.list, 'C:\\b.flp');
    expect(second.records.length).toBe(2);
    expect(second.list).toEqual(['C:\\b.flp', 'C:\\a.flp']);
  });

  it('ignores the other keys, the names that are not 1 to 49, the other types, and a deleted value', () => {
    const j = journalOf([
      recordBytes(REG_OP.value, RECENT_KEY, '1', 1, utf16Bytes('C:\\a.flp\u0000')),
      recordBytes(REG_OP.value, 'Software\\Other', '1', 1, utf16Bytes('C:\\x.flp\u0000')),
      recordBytes(REG_OP.value, RECENT_KEY, '01', 1, utf16Bytes('C:\\y.flp\u0000')),
      recordBytes(REG_OP.value, RECENT_KEY, '2', 4, utf16Bytes('C:\\z.flp\u0000')),
      recordBytes(REG_OP.value, RECENT_KEY, '3', 1, utf16Bytes('C:\\w.flp\u0000')),
      recordBytes(REG_OP.delValue, RECENT_KEY, '3', 1, new Uint8Array(0)),
    ]);
    expect(recentListOf(j.image())).toEqual(['C:\\a.flp']);
  });
});
