// tools/plugin-run/flp.mjs : a FL Studio project (.flp) read into its events and written back, for scene (d) of run.mjs (Issue #1524).
// The bytes of a project that is not changed come back the same (spec/plugin-run.test.mjs checks the templates).
// The file: the "FLhd" chunk (its uint32 length, then the data), the "FLdt" chunk (its uint32 length, then the events), and what follows.
// An event: its ID (1 byte), then its data: 1 byte for an ID below 64, 2 bytes below 128, 4 bytes below 192; from 192 the data is preceded by its length,
// a varint (7 bits a byte, the lowest group first, the high bit set on every byte but the last).
// The mixer insert's plug-in: its name is the event 201 (UTF-16LE with a NUL end) and its state the event 212 right after it;
// the insert's name (204) and the event 236 come before the name (the layout of the templates).
// The event 213 after the state (the plug-in's data, 66 or 792 bytes) is kept as it is.

export const EV_INSERT_NAME = 204;
export const EV_SLOT = 236;
export const EV_PLUGIN_NAME = 201;
export const EV_PLUGIN_STATE = 212;

const fixedLength = (id) => (id < 64 ? 1 : id < 128 ? 2 : 4);

function readVarint(buf, at) {
  let value = 0;
  let scale = 1;
  let i = at;
  for (;;) {
    if (i >= buf.length) throw new Error(`varint at ${at} runs past the end of the file`);
    const b = buf[i++];
    value += (b & 0x7f) * scale;
    scale *= 128;
    if ((b & 0x80) === 0) return { value, next: i };
  }
}

function writeVarint(n) {
  const out = [];
  let v = n;
  do {
    let b = v % 128;
    v = Math.floor(v / 128);
    if (v > 0) b |= 0x80;
    out.push(b);
  } while (v > 0);
  return Buffer.from(out);
}

/** the project's bytes split up: `head` (the bytes up to the FLdt data; its length is the last 4 bytes), the events, and `tail` (the bytes after the FLdt data) */
export function parseFlp(buf) {
  if (buf.toString('latin1', 0, 4) !== 'FLhd') throw new Error('not a FL Studio project: no FLhd chunk');
  const dt = 8 + buf.readUInt32LE(4);
  if (buf.toString('latin1', dt, dt + 4) !== 'FLdt') throw new Error('no FLdt chunk after FLhd');
  const start = dt + 8;
  const end = start + buf.readUInt32LE(dt + 4);
  if (end > buf.length) throw new Error('the FLdt chunk runs past the end of the file');
  const events = [];
  let at = start;
  while (at < end) {
    const id = buf[at];
    let len;
    let dataAt;
    if (id >= 192) {
      const v = readVarint(buf, at + 1);
      len = v.value;
      dataAt = v.next;
    } else {
      len = fixedLength(id);
      dataAt = at + 1;
    }
    if (dataAt + len > end) throw new Error(`event ${id} at ${at} runs past the FLdt chunk`);
    events.push({ id, data: buf.subarray(dataAt, dataAt + len) });
    at = dataAt + len;
  }
  return { head: buf.subarray(0, start), events, tail: buf.subarray(end) };
}

/** the bytes of a parsed project, its FLdt length written again */
export function encodeFlp({ head, events, tail }) {
  const data = Buffer.concat(events.flatMap((e) => (e.id >= 192 ? [Buffer.from([e.id]), writeVarint(e.data.length), e.data] : [Buffer.from([e.id]), e.data])));
  const h = Buffer.from(head);
  h.writeUInt32LE(data.length, h.length - 4);
  return Buffer.concat([h, data, tail]);
}

/** the name in a plug-in name's data (UTF-16LE, NUL-terminated, printable ASCII), or null when the data is not one */
export function nameOf(data) {
  if (data.length < 4 || data.length % 2 !== 0 || data.readUInt16LE(data.length - 2) !== 0) return null;
  const s = data.toString('utf16le', 0, data.length - 2);
  return /^[\x20-\x7e]+$/.test(s) ? s : null;
}

/** the data of a plug-in name: UTF-16LE, NUL-terminated */
export const nameData = (name) => Buffer.from(`${name}\0`, 'utf16le');

/** the indexes of the mixer insert plug-ins: a name event (201, see nameOf) right after the insert's name (204) and its 236, and followed by the state (212) */
export function insertPluginsOf(events) {
  const out = [];
  for (let i = 2; i + 1 < events.length; i++) {
    if (events[i].id === EV_PLUGIN_NAME && events[i - 2].id === EV_INSERT_NAME && events[i - 1].id === EV_SLOT && events[i + 1].id === EV_PLUGIN_STATE && nameOf(events[i].data) !== null) out.push(i);
  }
  return out;
}

/** the project with its first mixer insert plug-in renamed to `name`. Its state (the event 212 after the name) keeps its place and length but is zeroed:
 *  the state is the other plug-in's, and a slot without its state event makes no plug-in (measured: the guest asked for nothing) */
export function renamePlugin(parsed, name) {
  const [at] = insertPluginsOf(parsed.events);
  if (at === undefined) throw new Error('no mixer insert plug-in');
  const events = parsed.events.map((e, i) => {
    if (i === at) return { id: EV_PLUGIN_NAME, data: nameData(name) };
    if (i === at + 1) return { id: EV_PLUGIN_STATE, data: Buffer.alloc(e.data.length) };
    return e;
  });
  return { ...parsed, events };
}
