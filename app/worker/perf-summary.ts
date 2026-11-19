// GET /__perf/summary (decision 0224): the measurement beacons the Worker stored in R2 (shared/perf.ts: perf/<UTC date>/<sid>-<seq>.json), read back and summarised by the same library as the command line
// (shared/perf-summary.mjs: summarize + format), as text for a person who is signed in through Access. It knows nothing about the engine or its windows: it counts the records the page sent.
// Bounded on purpose: a request lists the objects of the last `days` UTC days (1 list call per 1,000 keys) and reads at most PERF_SUMMARY_MAX_OBJECTS of them, the newest by upload time, with at most 6 reads at once (a Worker invocation
// may wait for 6 connections at a time). The answer says how many objects the window holds and how many were read.
import { PERF_PREFIX } from '../shared/perf.ts';
import { format, joinStoredObjects, summarize } from '../shared/perf-summary.mjs';

export const PERF_SUMMARY_MAX_OBJECTS = 600;
export const PERF_SUMMARY_DEFAULT_DAYS = 2;
export const PERF_SUMMARY_MAX_DAYS = 180;
export const PERF_SUMMARY_DEFAULT_ROWS = 30;
const READ_AT_ONCE = 6;

/** the part of R2Bucket this file uses (R2Bucket satisfies it; a test can pass a few lines of its own) */
export interface PerfReadBucket {
  list(options: { prefix: string; cursor?: string }): Promise<{ objects: { key: string; uploaded: Date }[]; truncated: boolean; cursor?: string }>;
  get(key: string): Promise<{ text(): Promise<string> } | null>;
}
export interface PerfSummaryEnv { PERF_BUCKET?: PerfReadBucket }
export interface PerfSummaryAnswer { status: 200 | 400 | 503; body: string; json?: boolean }

const MACHINES = ['all', 'tailnet', 'loopback', 'lan', 'other', 'workers'];

/** the UTC dates (YYYY-MM-DD) of the last `days` days, today first */
export function windowDays(now: Date, days: number): string[] {
  const out: string[] = [];
  for (let i = 0; i < days; i++) out.push(new Date(now.getTime() - i * 86_400_000).toISOString().slice(0, 10));
  return out;
}

export async function perfSummary(env: PerfSummaryEnv, params: URLSearchParams, now: Date = new Date()): Promise<PerfSummaryAnswer> {
  const bucket = env.PERF_BUCKET;
  if (!bucket) return { status: 503, body: 'このWorkerには計測の保存先(R2 バインディング PERF_BUCKET)がありません。\n' };
  const daysParam = params.get('days');
  const days = daysParam === null ? PERF_SUMMARY_DEFAULT_DAYS : /^\d{1,3}$/.test(daysParam) ? Number(daysParam) : 0;
  if (days < 1 || days > PERF_SUMMARY_MAX_DAYS) return { status: 400, body: `days は 1〜${PERF_SUMMARY_MAX_DAYS} の整数です(UTC の日数。既定 ${PERF_SUMMARY_DEFAULT_DAYS})。\n` };
  const rowsParam = params.get('rows');
  const rows = rowsParam === null ? PERF_SUMMARY_DEFAULT_ROWS : rowsParam === 'all' ? undefined : /^\d{1,5}$/.test(rowsParam) ? Number(rowsParam) : -1;
  if (rows !== undefined && rows < 0) return { status: 400, body: 'rows は 0〜99999 の整数か all です(各記録の 1 行表示の件数。既定 30)。\n' };
  const machine = params.get('machine') ?? 'all';
  if (!MACHINES.includes(machine)) return { status: 400, body: `machine は ${MACHINES.join(' / ')} のどれかです。\n` };
  const asJson = params.get('format') === 'json';

  const dates = windowDays(now, days);
  const listed: { key: string; uploaded: number }[] = [];
  for (const d of dates) {
    let cursor: string | undefined;
    do {
      const page = await bucket.list({ prefix: `${PERF_PREFIX}${d}/`, ...(cursor ? { cursor } : {}) });
      for (const o of page.objects) listed.push({ key: o.key, uploaded: o.uploaded.getTime() });
      cursor = page.truncated ? page.cursor : undefined;
    } while (cursor);
  }
  const chosen = listed.length > PERF_SUMMARY_MAX_OBJECTS ? [...listed].sort((a, b) => b.uploaded - a.uploaded || (a.key < b.key ? -1 : 1)).slice(0, PERF_SUMMARY_MAX_OBJECTS) : listed;

  const texts: string[] = [];
  let missing = 0;
  let next = 0;
  const reader = async (): Promise<void> => {
    for (;;) {
      const i = next++;
      if (i >= chosen.length) return;
      const o = await bucket.get((chosen[i] as { key: string }).key);
      if (o === null) { missing++; continue; }
      texts.push(await o.text());
    }
  };
  await Promise.all(Array.from({ length: Math.min(READ_AT_ONCE, chosen.length) }, reader));

  const s = summarize(joinStoredObjects(texts));
  if (asJson) return { status: 200, json: true, body: JSON.stringify({ window: { dates, objects: listed.length, read: texts.length, missing }, summary: s }, null, 1) + '\n' };
  const head = [
    `計測の集計(Workers が R2 の ${PERF_PREFIX} に保存したビーコン。保存するのはこのWorkerが受け取った記録だけで、IP アドレス・メール・Cookie は保存していません)`,
    `範囲: UTC ${dates[dates.length - 1]} 〜 ${dates[0]}(${days} 日)。範囲内のオブジェクト ${listed.length} 件、読んだ ${texts.length} 件${listed.length > chosen.length ? `(多すぎるため、アップロードの新しい ${PERF_SUMMARY_MAX_OBJECTS} 件だけ。古い記録は数えていません。?days= を小さくしてください)` : ''}${missing ? `、読む前に消えていた ${missing} 件` : ''}`,
    `?days=N(1〜${PERF_SUMMARY_MAX_DAYS})  ?machine=${MACHINES.join('|')}  ?rows=N|all  ?format=json`,
    '',
  ];
  return { status: 200, body: head.join('\n') + format(s, { machine, ...(rows !== undefined ? { rows } : {}) }) + '\n' };
}
