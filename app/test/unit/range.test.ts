import { describe, expect, it } from 'vitest';
import { ifNoneMatchHit, ifRangeAllows, parseRange } from '../../shared/range.ts';

describe('parseRange', () => {
  it('start-end, inclusive', () => expect(parseRange('bytes=0-99', 1000)).toEqual({ kind: 'partial', start: 0, end: 99 }));
  it('the last byte', () => expect(parseRange('bytes=999-999', 1000)).toEqual({ kind: 'partial', start: 999, end: 999 }));
  it('open end', () => expect(parseRange('bytes=500-', 1000)).toEqual({ kind: 'partial', start: 500, end: 999 }));
  it('end past the size is clamped', () => expect(parseRange('bytes=900-5000', 1000)).toEqual({ kind: 'partial', start: 900, end: 999 }));
  it('suffix', () => expect(parseRange('bytes=-100', 1000)).toEqual({ kind: 'partial', start: 900, end: 999 }));
  it('suffix longer than the object is the whole object', () => expect(parseRange('bytes=-5000', 1000)).toEqual({ kind: 'partial', start: 0, end: 999 }));
  it('start at or past the size is unsatisfiable', () => {
    expect(parseRange('bytes=1000-', 1000)).toEqual({ kind: 'unsatisfiable' });
    expect(parseRange('bytes=1000-1001', 1000)).toEqual({ kind: 'unsatisfiable' });
    expect(parseRange('bytes=5000-6000', 1000)).toEqual({ kind: 'unsatisfiable' });
  });
  it('suffix of zero bytes, or any range of an empty object, is unsatisfiable', () => {
    expect(parseRange('bytes=-0', 1000)).toEqual({ kind: 'unsatisfiable' });
    expect(parseRange('bytes=0-0', 0)).toEqual({ kind: 'unsatisfiable' });
    expect(parseRange('bytes=-5', 0)).toEqual({ kind: 'unsatisfiable' });
  });
  it('ignores what it does not serve: no header, another unit, several ranges, garbage, a reversed range', () => {
    for (const h of [undefined, null, '', 'items=0-1', 'bytes=0-1,5-6', 'bytes=abc', 'bytes=-', 'bytes=5-2', 'bytes 0-1']) expect(parseRange(h, 1000), String(h)).toEqual({ kind: 'none' });
  });
  it('huge numbers do not lose precision into a wrong answer', () => {
    expect(parseRange('bytes=99999999999999999999-', 1000)).toEqual({ kind: 'unsatisfiable' });
    expect(parseRange('bytes=0-99999999999999999999', 1000)).toEqual({ kind: 'unsatisfiable' });
  });
  it('is case-insensitive in the unit and tolerant of spaces', () => expect(parseRange('Bytes = 10 - 19', 1000)).toEqual({ kind: 'partial', start: 10, end: 19 }));
});

describe('ETag helpers', () => {
  it('If-None-Match: strong and weak forms, lists, *', () => {
    expect(ifNoneMatchHit('"abc"', '"abc"')).toBe(true);
    expect(ifNoneMatchHit('W/"abc"', '"abc"')).toBe(true);
    expect(ifNoneMatchHit('"x", "abc"', '"abc"')).toBe(true);
    expect(ifNoneMatchHit('*', '"abc"')).toBe(true);
    expect(ifNoneMatchHit('"other"', '"abc"')).toBe(false);
    expect(ifNoneMatchHit(null, '"abc"')).toBe(false);
  });
  it('If-Range: only an equal strong validator keeps the range', () => {
    expect(ifRangeAllows(null, '"abc"')).toBe(true);
    expect(ifRangeAllows('"abc"', '"abc"')).toBe(true);
    expect(ifRangeAllows('"zzz"', '"abc"')).toBe(false);
    expect(ifRangeAllows('W/"abc"', '"abc"')).toBe(false);
  });
});
