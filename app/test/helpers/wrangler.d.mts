// types for the two helpers the Worker tests call from TypeScript (decision 0152); the implementation is wrangler.mjs
export function putObject(workDir: string, key: string, file: string, opts?: { contentType?: string; contentEncoding?: string; cacheControl?: string }): Promise<void>;
export function deleteObject(workDir: string, key: string): Promise<void>;
// decision 0224: the beacon bucket of the local R2 (wrangler dev --local)
export const PERF_BUCKET: string;
export function listLocalKeys(workDir: string, prefix?: string): { key: string; size: number; contentType: string | null }[];
export function getPerfObject(workDir: string, key: string): Promise<string | null>;
