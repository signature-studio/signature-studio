// The engine bundle manifest: what the app needs to start the engine (the wasm, the glue, the guest's virtual files, the DLL data, the arguments).
// Schema version 1. Types and a JSON-Schema-like description live side by side; validateManifest() checks a parsed JSON value against the schema and returns every error.
// Everything in a bundle is addressed by a path on the page's own origin (no external URL can be named: see isSameOriginPath).

import { validate, type Schema, type ValidationError } from './schema.ts';

export const MANIFEST_SCHEMA_VERSION = 1;

export type Compression = 'identity' | 'gzip' | 'br';
/** the format tag of the engine (one engine: the lifted FL engine built with emscripten) */
export type EngineKind = 'emscripten-fruity-v1';

export interface AssetRef {
  /** path on this origin, e.g. /assets/fl-Oz/engine.wasm */
  url: string;
  /** size in bytes of the DECODED content (what the progress bar counts and what is hashed) */
  size: number;
  /** SHA-256 (lower case hex) of the decoded content */
  sha256: string;
  /**
   * How the object stored at `url` is encoded. If the server sends Content-Encoding the browser decodes it and this only documents it;
   * if the server sends no Content-Encoding and this is gzip / br, the loader decodes with DecompressionStream.
   */
  compression: Compression;
  /** size of the stored (encoded) object, informational */
  encodedSize?: number;
}

/** A pack of files: the guest's virtual files (seed pack) or DLL data (DLL pack). Formats: see pack.ts (flpack1; flseedp1 = the seed pack of decision 0134, pack_seed.py). */
export interface PackRef extends AssetRef { format: 'flpack1' | 'flseedp1' }

/** One note for a known way the engine stops: when the engine's stop message contains `contains`, the UI adds `note` to what it says. */
export interface StopNote { contains: string; note: string }

/** What the bundle says about how long its engine takes (measured on the build machine; only a hint for the progress display). */
export interface FruityHints {
  /** seconds from the start of the engine (main) to the first non-background pixel, on the machine that built the bundle */
  firstPixelSeconds: number;
  /** seconds from the start of the engine to its stop, when it stops by itself */
  stopSeconds?: number;
  /** a label of the build variant (Oz, O2, H), shown in the UI */
  variant?: string;
}

/**
 * Options of the engine (lift/harness/two/build_nmod_wasm.sh FRUITY=1 + lift/harness/web/fruity/link-web.sh).
 *  - engine.wasm is flengine6.wasm, `glue` the emscripten JS module (-sMODULARIZE=1 -sEXPORT_ES6=1) that instantiates it; both are named by SHA-256 in the manifest.
 *  - seedPack and dllPack are the files the engine reads from the host (open / stat / pread / fopen) under `root`: a pack path P is the file `root/P`. engine.args are the engine's argv (paths under `root`),
 *    engine.env its environment. The glue was linked with -sFILESYSTEM=0 by the Emscripten named in `hostFilesAbi` and the page answers the file calls from the pack bytes (src/engine/hostfiles.ts).
 */
export interface FruityOptions {
  glue: AssetRef;
  root: string;
  /** the Emscripten that linked the glue ("emscripten-6.0.12"); the page refuses a version whose system-call layer it was not written for */
  hostFilesAbi: string;
  hints?: FruityHints;
  stopNotes?: StopNote[];
}

export interface Manifest {
  schema: 1;
  bundle: { name: string; version: string; created?: string; description?: string };
  engine: {
    kind: EngineKind;
    wasm: AssetRef;
    /** the engine's argv after the program name */
    args: string[];
    /** the engine's environment */
    env: Record<string, string>;
    options: FruityOptions;
  };
  /** the guest's virtual files */
  seedPack: PackRef;
  /** the engine and module DLLs, and their data */
  dllPack: PackRef;
}

const sha256Schema: Schema = { type: 'string', pattern: '^[0-9a-f]{64}$' };
const nonNegInt = (max = 0xffffffff): Schema => ({ type: 'integer', minimum: 0, maximum: max });

const assetProps: Record<string, Schema> = {
  url: { type: 'string', format: 'same-origin-url' },
  size: nonNegInt(2 ** 32),
  sha256: sha256Schema,
  compression: { type: 'string', enum: ['identity', 'gzip', 'br'] },
  encodedSize: nonNegInt(2 ** 32),
};

const assetSchema: Schema = { type: 'object', required: ['url', 'size', 'sha256', 'compression'], additionalProperties: false, properties: assetProps };
const packSchema: Schema = {
  type: 'object', required: ['url', 'size', 'sha256', 'compression', 'format'], additionalProperties: false,
  properties: { ...assetProps, format: { type: 'string', enum: ['flpack1', 'flseedp1'] } },
};

export const MANIFEST_SCHEMA: Schema = {
  type: 'object',
  required: ['schema', 'bundle', 'engine', 'seedPack', 'dllPack'],
  additionalProperties: false,
  properties: {
    schema: { type: 'integer', const: MANIFEST_SCHEMA_VERSION },
    bundle: {
      type: 'object', required: ['name', 'version'], additionalProperties: false,
      properties: {
        name: { type: 'string', minLength: 1, maxLength: 200, pattern: '^[A-Za-z0-9._-]+$' },
        version: { type: 'string', minLength: 1, maxLength: 100 },
        created: { type: 'string', maxLength: 64 },
        description: { type: 'string', maxLength: 2000 },
      },
    },
    engine: {
      type: 'object', required: ['kind', 'wasm', 'args', 'env', 'options'], additionalProperties: false,
      properties: {
        kind: { type: 'string', enum: ['emscripten-fruity-v1'] },
        wasm: assetSchema,
        args: { type: 'array', items: { type: 'string', maxLength: 4096 }, maxItems: 256 },
        env: { type: 'object', additionalProperties: { type: 'string', maxLength: 8192 } },
        options: { type: 'object' },
      },
    },
    seedPack: packSchema,
    dllPack: packSchema,
  },
};

const fruityAssetSchema: Schema = { type: 'object', required: ['url', 'size', 'sha256', 'compression'], additionalProperties: false, properties: assetProps };

export const FRUITY_OPTIONS_SCHEMA: Schema = {
  type: 'object', required: ['glue', 'root', 'hostFilesAbi'], additionalProperties: false,
  properties: {
    glue: fruityAssetSchema,
    hostFilesAbi: { type: 'string', pattern: '^emscripten-[0-9]+\\.[0-9]+\\.[0-9]+$', maxLength: 32 },
    root: { type: 'string', pattern: '^/(?!\\.{1,2}(/|$))[A-Za-z0-9._-]+(/(?!\\.{1,2}(/|$))[A-Za-z0-9._-]+)*$', maxLength: 256 },
    hints: {
      type: 'object', required: ['firstPixelSeconds'], additionalProperties: false,
      properties: { firstPixelSeconds: { type: 'number', minimum: 0, maximum: 86400 }, stopSeconds: { type: 'number', minimum: 0, maximum: 86400 }, variant: { type: 'string', maxLength: 32 } },
    },
    stopNotes: {
      type: 'array', maxItems: 32,
      items: { type: 'object', required: ['contains', 'note'], additionalProperties: false, properties: { contains: { type: 'string', minLength: 1, maxLength: 512 }, note: { type: 'string', minLength: 1, maxLength: 2000 } } },
    },
  },
};

export type ManifestResult = { ok: true; manifest: Manifest } | { ok: false; errors: ValidationError[] };

export function validateManifest(input: unknown): ManifestResult {
  const errors = validate(MANIFEST_SCHEMA, input);
  if (errors.length === 0) {
    const m = input as Manifest;
    validate(FRUITY_OPTIONS_SCHEMA, m.engine.options, '/engine/options', errors);
    if (m.engine.args.length === 0) errors.push({ path: '/engine/args', message: 'must name the engine DLL (and the module list)' });
    else if (errors.length === 0) {
      const root = m.engine.options.root;
      if (!(m.engine.args[0] as string).startsWith(`${root}/`)) errors.push({ path: '/engine/args/0', message: `must be a path under the file root ${root}/ (the engine DLL)` });
    }
  }
  return errors.length === 0 ? { ok: true, manifest: input as Manifest } : { ok: false, errors };
}

export function formatErrors(errors: readonly ValidationError[]): string {
  return errors.map((e) => `${e.path}: ${e.message}`).join('\n');
}
