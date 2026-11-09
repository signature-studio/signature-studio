// A small JSON-Schema-like validator (the subset the manifest needs). No dependency; errors carry a JSON-pointer path.
// Supported keywords: type (string or array), enum, const, properties, required, additionalProperties (boolean or schema), items, minItems, maxItems,
// minimum, maximum, minLength, maxLength, pattern, format. Unknown keywords are an error in the SCHEMA (caught by test/unit/manifest.test.ts), not ignored.

export type JsonType = 'object' | 'array' | 'string' | 'integer' | 'number' | 'boolean' | 'null';

export interface Schema {
  type?: JsonType | JsonType[];
  enum?: readonly unknown[];
  const?: unknown;
  properties?: Record<string, Schema>;
  required?: readonly string[];
  additionalProperties?: boolean | Schema;
  items?: Schema;
  minItems?: number;
  maxItems?: number;
  minimum?: number;
  maximum?: number;
  minLength?: number;
  maxLength?: number;
  pattern?: string;
  format?: 'same-origin-url' | 'relative-path';
}

export interface ValidationError { path: string; message: string }

const KNOWN = new Set(['type', 'enum', 'const', 'properties', 'required', 'additionalProperties', 'items', 'minItems', 'maxItems', 'minimum', 'maximum', 'minLength', 'maxLength', 'pattern', 'format']);

function typeOf(v: unknown): JsonType | 'undefined' | 'other' {
  if (v === null) return 'null';
  if (Array.isArray(v)) return 'array';
  switch (typeof v) {
    case 'string': return 'string';
    case 'boolean': return 'boolean';
    case 'number': return Number.isInteger(v) ? 'integer' : 'number';
    case 'object': return 'object';
    case 'undefined': return 'undefined';
    default: return 'other';
  }
}

const matchesType = (v: unknown, t: JsonType): boolean => {
  const actual = typeOf(v);
  return actual === t || (t === 'number' && actual === 'integer');
};

// URLs inside a bundle are paths on the page's own origin: no scheme, no host, no protocol-relative form.
export function isSameOriginPath(s: string): boolean {
  if (s.length === 0 || s.length > 1024) return false;
  if (/^[A-Za-z][A-Za-z0-9+.-]*:/.test(s)) return false;
  if (s.startsWith('//') || s.includes('\\') || /[\u0000-\u001f]/.test(s)) return false;
  return true;
}

// A path inside a pack: relative, '/'-separated, no '..' / '.' segments, no empty segments.
export function isRelativePackPath(s: string): boolean {
  if (s.length === 0 || s.length > 1024 || s.startsWith('/') || s.includes('\\') || /[\u0000-\u001f]/.test(s)) return false;
  return s.split('/').every((seg) => seg !== '' && seg !== '.' && seg !== '..');
}

export function validate(schema: Schema, value: unknown, path = '', out: ValidationError[] = []): ValidationError[] {
  for (const k of Object.keys(schema)) if (!KNOWN.has(k)) out.push({ path, message: `schema error: unknown keyword ${k}` });
  const here = path === '' ? '/' : path;
  if (schema.type !== undefined) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((t) => matchesType(value, t))) {
      out.push({ path: here, message: `expected ${types.join(' | ')}, got ${typeOf(value)}` });
      return out;
    }
  }
  if (schema.const !== undefined && value !== schema.const) out.push({ path: here, message: `must equal ${JSON.stringify(schema.const)}` });
  if (schema.enum && !schema.enum.includes(value)) out.push({ path: here, message: `must be one of ${schema.enum.map((e) => JSON.stringify(e)).join(', ')}` });
  if (typeof value === 'number') {
    if (schema.minimum !== undefined && value < schema.minimum) out.push({ path: here, message: `must be >= ${schema.minimum}` });
    if (schema.maximum !== undefined && value > schema.maximum) out.push({ path: here, message: `must be <= ${schema.maximum}` });
  }
  if (typeof value === 'string') {
    if (schema.minLength !== undefined && value.length < schema.minLength) out.push({ path: here, message: `length must be >= ${schema.minLength}` });
    if (schema.maxLength !== undefined && value.length > schema.maxLength) out.push({ path: here, message: `length must be <= ${schema.maxLength}` });
    if (schema.pattern !== undefined && !new RegExp(schema.pattern).test(value)) out.push({ path: here, message: `must match /${schema.pattern}/` });
    if (schema.format === 'same-origin-url' && !isSameOriginPath(value)) out.push({ path: here, message: 'must be a path on this origin (no scheme, no host, no //)' });
    if (schema.format === 'relative-path' && !isRelativePackPath(value)) out.push({ path: here, message: 'must be a relative path without "." / ".." segments' });
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) out.push({ path: here, message: `must have >= ${schema.minItems} items` });
    if (schema.maxItems !== undefined && value.length > schema.maxItems) out.push({ path: here, message: `must have <= ${schema.maxItems} items` });
    if (schema.items) value.forEach((item, i) => validate(schema.items as Schema, item, `${path}/${i}`, out));
  }
  if (typeOf(value) === 'object') {
    const obj = value as Record<string, unknown>;
    for (const r of schema.required ?? []) if (!(r in obj)) out.push({ path: `${path}/${r}`, message: 'is required' });
    for (const [k, v] of Object.entries(obj)) {
      const sub = schema.properties?.[k];
      if (sub) validate(sub, v, `${path}/${k}`, out);
      else if (schema.additionalProperties === false) out.push({ path: `${path}/${k}`, message: 'unknown property' });
      else if (typeof schema.additionalProperties === 'object') validate(schema.additionalProperties, v, `${path}/${k}`, out);
    }
  }
  return out;
}
