// The window table of the page's picture (deskDumpBase64's `windows`), read whole (Issue #1485): the top-level windows (`lid N at (x, y) WxH ... class "..." title "..."`)
// and the child windows (`child lid N parent lid M at (x, y) WxH vis V class "C" text "T"`; the child's (x, y) is in its parent's client area).
// Pure: text in, objects out. The absolute position of a child is the sum of its parents' offsets (absolutePosition).
const TOP = /^lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+)(.*)$/;
const TOP_NAMES = /class "([^"]*)" title "([^"]*)"\s*$/;
const CHILD = /^child lid (\d+) parent lid (\d+) at \((-?\d+), (-?\d+)\) (\d+)x(\d+) vis (\d) class "([^"]*)" text "(.*)"\s*$/;

/** the window table: { top: [{ lid, x, y, w, h, cls, title }], children: [{ lid, parent, x, y, w, h, vis, cls, text }] } (the child style lines are not kept) */
export function parseDump(text) {
  const top = [], children = [];
  for (const line of String(text).split('\n')) {
    let m = CHILD.exec(line);
    if (m) {
      children.push({ lid: Number(m[1]), parent: Number(m[2]), x: Number(m[3]), y: Number(m[4]), w: Number(m[5]), h: Number(m[6]), vis: m[7] === '1', cls: m[8], text: m[9] });
      continue;
    }
    m = TOP.exec(line);
    if (m) {
      const names = TOP_NAMES.exec(m[6]);
      top.push({ lid: Number(m[1]), x: Number(m[2]), y: Number(m[3]), w: Number(m[4]), h: Number(m[5]), cls: names ? names[1] : '', title: names ? names[2] : '' });
    }
  }
  return { top, children };
}

/** every window of the table (top-level and child) by lid */
function byLid(dump) {
  const m = new Map();
  for (const w of dump.top) m.set(w.lid, { ...w, child: false, parent: null });
  for (const w of dump.children) m.set(w.lid, { ...w, child: true });
  return m;
}

/** the absolute rectangle of a window: its offset plus the offsets of its parents (guest px); null when the chain is broken */
export function absoluteRect(dump, lid) {
  const all = byLid(dump);
  const w = all.get(lid);
  if (!w) return null;
  let x = w.x, y = w.y, cur = w, guard = 0;
  while (cur.child && guard++ < 64) {
    const p = all.get(cur.parent);
    if (!p) return null;
    x += p.x; y += p.y;
    cur = p;
  }
  return { x, y, w: w.w, h: w.h };
}

/** the top-level window a window belongs to (itself when it is top-level); null when the chain is broken */
export function rootOf(dump, lid) {
  const all = byLid(dump);
  let w = all.get(lid), guard = 0;
  while (w && w.child && guard++ < 64) w = all.get(w.parent);
  return w ?? null;
}

/** the descendants of a window (its child windows, at any depth), in table order */
export function subtree(dump, lid) {
  const out = [];
  const ids = new Set([lid]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const c of dump.children) if (!ids.has(c.lid) && ids.has(c.parent)) { ids.add(c.lid); out.push(c); grew = true; }
  }
  return out;
}

/** the child windows of the table whose class is one of `classes` (and, when given, whose text is `text`) */
export function childrenOfClass(dump, classes, text = undefined) {
  const set = new Set(classes);
  return dump.children.filter((c) => set.has(c.cls) && (text === undefined || c.text === text));
}
