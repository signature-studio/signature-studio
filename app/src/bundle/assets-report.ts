// What the page tells about its start-up assets (decision 0220): per asset where the bytes came from (the network, the browser's HTTP cache, the page's own stored copy in OPFS), how many bytes crossed the wire, and where
// the time went (headers, read incl. the browser's decoding, hash, copy, write of the stored copy, compile). It is the beacon's optional `assets` block and the lines of the performance panel.
// Why: the person who reads the log cannot see the browser's network tab; "the download takes 4.5 s" says nothing about whether it was the network or the CPU (decision 0220 measured: with a warm HTTP cache it is the CPU).
// Fixed words and whole numbers only: no URL, no path, no name; the hash only as its first 8 hex digits.

import type { AssetOrigin, AssetTrace, HashMode, StoreState } from './loader.ts';
import type { CleanResult, StoreStats } from './assetstore.ts';
import type { PERF_ASSET_FAIL_CODES, PERF_ASSET_KINDS } from '../../shared/perf.ts';

export type AssetKind = (typeof PERF_ASSET_KINDS)[number];
export type AssetFail = (typeof PERF_ASSET_FAIL_CODES)[number];
export type ReportOrigin = AssetOrigin | 'http-cache';

export interface AssetItem {
  kind: AssetKind;
  /** the first 8 hex digits of the manifest's SHA-256 */
  sha: string;
  /** decoded bytes */
  bytes: number;
  /** PerformanceResourceTiming.encodedBodySize (the bytes as stored on the server); null: the stored copy was used and nothing was requested */
  wire: number | null;
  /** PerformanceResourceTiming.transferSize: 0 = no bytes crossed the network (a header-only revalidation is a few hundred); null as above */
  transfer: number | null;
  origin: ReportOrigin;
  store: StoreState;
  fail: AssetFail | null;
  ms: { headers: number; read: number; hash: number; copy: number; parse: number; store: number; total: number; compile: number | null };
}

export interface AssetsReport {
  v: 1;
  /** on: the page tried the stored copy; off: not tried; none: this browser has no OPFS */
  store: 'on' | 'off' | 'none';
  hash: HashMode;
  items: AssetItem[];
  storage: { usedMiB: number | null; quotaMiB: number | null; persisted: boolean | null } | null;
  clean: CleanResult | null;
}

const KIND_OF_LABEL: Record<string, AssetKind> = { 'engine wasm': 'wasm', 'DLL pack': 'dll', 'seed pack': 'seed', 'engine glue': 'glue' };
const ORDER: AssetKind[] = ['wasm', 'dll', 'seed', 'glue'];
const FAILS = new Set<string>(['unsupported', 'quota', 'security', 'locked', 'io', 'other', 'verify', 'busy']);
const ms = (n: number): number => Math.max(0, Math.round(n));

/** the resource timing of one asset URL, or null (the entry is gone, or the browser does not list it) */
export interface ResourceFacts { transfer: number; encoded: number; delivery: string }
export type ResourceLookup = (absUrl: string) => ResourceFacts | null;

export function resourceLookupFromPerformance(perf: Pick<Performance, 'getEntriesByName'> = performance): ResourceLookup {
  return (url) => {
    const e = perf.getEntriesByName(url, 'resource').pop() as (PerformanceResourceTiming & { deliveryType?: string }) | undefined;
    return e ? { transfer: e.transferSize, encoded: e.encodedBodySize, delivery: e.deliveryType ?? '' } : null;
  };
}

/** where a network-path asset really came from: the browser says 'cache' for a response served from its HTTP cache (also after a header-only revalidation); without that field, no transferred byte means the cache */
export function classifyNetwork(f: ResourceFacts | null): { origin: 'network' | 'http-cache'; wire: number | null; transfer: number | null } {
  if (!f) return { origin: 'network', wire: null, transfer: null };
  const cache = f.delivery === 'cache' || (f.delivery === '' && f.transfer === 0 && f.encoded > 0);
  return { origin: cache ? 'http-cache' : 'network', wire: f.encoded, transfer: f.transfer };
}

export class AssetsRecorder {
  private readonly traces = new Map<string, AssetTrace>();
  readonly store: 'on' | 'off' | 'none';
  readonly hash: HashMode;
  private storage: AssetsReport['storage'] = null;
  private clean: CleanResult | null = null;
  private readonly shaOf = new Map<string, string>();
  private readonly refOf = new Map<string, { url: string; size: number }>();
  /** true: the loader's original path ran (no stored copy, JS hash while the bytes arrive), which makes no trace; the report then lists the assets from the manifest and the browser's resource timing only */
  private readonly legacy: boolean;
  constructor(store: 'on' | 'off' | 'none', hash: HashMode, legacy = false) { this.store = store; this.hash = hash; this.legacy = legacy; }

  /** the loader's onTrace; a second call for the same asset (the background write ended) replaces the first */
  trace = (t: AssetTrace): void => { this.traces.set(t.label, t); for (const w of this.waiters.slice()) w(); };
  /** the manifest's SHA-256 of an asset, for the 8-digit prefix */
  expect(label: string, ref: { url: string; size: number; sha256: string }, base?: string): void {
    this.shaOf.set(label, ref.sha256);
    this.refOf.set(label, { url: base ? new URL(ref.url, base).href : ref.url, size: ref.size });
  }
  setStorage(s: StoreStats | null): void { this.storage = s; }
  setClean(c: CleanResult | null): void { this.clean = c; }
  get size(): number { return this.traces.size; }
  private waiters: (() => void)[] = [];
  private pending(): boolean { for (const t of this.traces.values()) if (t.store === 'writing') return true; return false; }
  /** resolves when no asset is being written to the stored copy any more (at most `maxMs`, so that a hung disk cannot hold the clean-up for ever) */
  writesSettled(maxMs = 120_000): Promise<void> {
    if (!this.pending()) return Promise.resolve();
    return new Promise((resolve) => {
      const t = setTimeout(done, maxMs);
      const self = this;
      function done(): void { clearTimeout(t); self.waiters = self.waiters.filter((w) => w !== check); resolve(); }
      const check = (): void => { if (!self.pending()) done(); };
      this.waiters.push(check);
    });
  }
  /** the stored copy of this asset exists: it was used or has been written */
  stored(ref: { sha256: string }): boolean {
    const label = [...this.shaOf.entries()].find(([, sha]) => sha === ref.sha256)?.[0];
    const t = label ? this.traces.get(label) : undefined;
    return !!t && (t.store === 'hit' || t.store === 'stored');
  }

  report(lookup: ResourceLookup): AssetsReport {
    const items: AssetItem[] = [];
    const traces = [...this.traces.values()];
    if (this.legacy) for (const [label, r] of this.refOf) if (!this.traces.has(label)) traces.push({ label, url: r.url, bytes: r.size, origin: 'network', hash: 'js-stream', store: 'off', fail: null, headersMs: 0, readMs: 0, hashMs: 0, copyMs: 0, storeMs: 0, totalMs: 0, compileMs: null, parseMs: 0 });
    for (const t of traces) {
      const kind = KIND_OF_LABEL[t.label];
      if (!kind) continue;
      const net = t.origin === 'network' ? classifyNetwork(lookup(t.url)) : { origin: 'storage-cache' as const, wire: null, transfer: null };
      const fail = t.fail !== null && FAILS.has(t.fail) ? (t.fail as AssetFail) : t.fail === null ? null : 'other';
      items.push({
        kind, sha: (this.shaOf.get(t.label) ?? '').slice(0, 8).padEnd(8, '0'), bytes: t.bytes, wire: net.wire, transfer: net.transfer, origin: net.origin, store: t.store, fail,
        ms: { headers: ms(t.headersMs), read: ms(t.readMs), hash: ms(t.hashMs), copy: ms(t.copyMs), parse: ms(t.parseMs), store: ms(t.storeMs), total: ms(t.totalMs), compile: t.compileMs === null ? null : ms(t.compileMs) },
      });
    }
    items.sort((a, b) => ORDER.indexOf(a.kind) - ORDER.indexOf(b.kind));
    return { v: 1, store: this.store, hash: this.hash, items, storage: this.storage, clean: this.clean };
  }
}

/** one line per asset for the panel and the log: "wasm 124.2 MiB from OPFS: read 180 ms, hash 140 ms, copy 0 ms, compile 1.6 s" */
export function assetLines(r: AssetsReport): string[] {
  const mib = (n: number): string => `${(n / 1048576).toFixed(1)} MiB`;
  const sec = (n: number): string => (n >= 1000 ? `${(n / 1000).toFixed(1)} s` : `${n} ms`);
  const from: Record<ReportOrigin, string> = { network: 'the network', 'http-cache': "the browser's HTTP cache", 'storage-cache': "the page's own stored copy (OPFS)" };
  return r.items.map((i) => {
    const wire = i.transfer === null ? '' : i.transfer === 0 ? ', no byte transferred' : `, ${mib(i.transfer)} transferred`;
    const st = i.store === 'off' ? '' : i.store === 'hit' ? '' : `; stored copy: ${i.store}${i.fail ? ` (${i.fail})` : ''}${i.ms.store ? ` ${sec(i.ms.store)}` : ''}`;
    return `${i.kind} ${mib(i.bytes)} from ${from[i.origin]}${wire}: read ${sec(i.ms.read)}, hash ${sec(i.ms.hash)} (${r.hash === 'subtle' ? 'WebCrypto' : 'JS'}), copy ${sec(i.ms.copy)}${i.ms.parse ? `, pack index ${sec(i.ms.parse)}` : ''}${i.ms.compile !== null ? `, compiled at ${sec(i.ms.compile)}` : ''}${st}`;
  });
}

/** the one-line summary for the Download stage: "from the network ×1, the browser's cache ×3" */
export function sourceSummary(r: AssetsReport): string {
  const n: Record<ReportOrigin, number> = { network: 0, 'http-cache': 0, 'storage-cache': 0 };
  for (const i of r.items) n[i.origin]++;
  const parts: string[] = [];
  if (n['storage-cache']) parts.push(`own stored copy ×${n['storage-cache']}`);
  if (n['http-cache']) parts.push(`HTTP cache ×${n['http-cache']}`);
  if (n.network) parts.push(`network ×${n.network}`);
  return parts.join(', ');
}
