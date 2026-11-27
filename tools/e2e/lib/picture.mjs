// The guest's picture as the page gives it (deskDumpBase64: RGBA bytes, base64; the rows may be longer than width x 4) (Issue #1485). Pure.
import crypto from 'node:crypto';

/** the picture: { w, h, stride, bytes } from the base64 text and the size (the stride is the row length in bytes) */
export function decodePicture({ base64, width, height }) {
  const bytes = Buffer.from(base64, 'base64');
  const stride = height > 0 ? bytes.length / height : 0;
  if (!Number.isInteger(stride) || stride < width * 4) throw new Error(`the picture has ${bytes.length} bytes for ${width}x${height}`);
  return { w: width, h: height, stride, bytes };
}

/** the rectangle clipped to the picture: [x0, y0, x1, y1) (x1 and y1 exclusive); null when nothing is left */
export function clipRect(pic, r) {
  const x0 = Math.max(0, Math.floor(r.x)), y0 = Math.max(0, Math.floor(r.y));
  const x1 = Math.min(pic.w, Math.ceil(r.x + r.w)), y1 = Math.min(pic.h, Math.ceil(r.y + r.h));
  return x1 > x0 && y1 > y0 ? [x0, y0, x1, y1] : null;
}

/** the SHA-256 (hex) of the RGB bytes of a rectangle of the picture: the same rectangle with the same pixels gives the same hash */
export function regionHash(pic, r) {
  const c = clipRect(pic, r);
  const h = crypto.createHash('sha256');
  if (c === null) return h.update('empty').digest('hex');
  const [x0, y0, x1, y1] = c;
  const row = Buffer.alloc((x1 - x0) * 3);
  for (let y = y0; y < y1; y++) {
    let k = 0;
    for (let x = x0; x < x1; x++) {
      const i = y * pic.stride + 4 * x;
      row[k++] = pic.bytes[i]; row[k++] = pic.bytes[i + 1]; row[k++] = pic.bytes[i + 2];
    }
    h.update(row);
  }
  return h.digest('hex');
}

/**
 * the number of tiles of `tile` x `tile` guest px whose pixels are all black (R, G and B 0) over the whole picture: a black rectangle of the picture
 * (a tile is counted once; a partial tile at the right or bottom edge is counted with the pixels it has).
 */
export function blackTiles(pic, tile = 64) {
  let n = 0;
  for (let ty = 0; ty < pic.h; ty += tile) {
    for (let tx = 0; tx < pic.w; tx += tile) {
      const x1 = Math.min(pic.w, tx + tile), y1 = Math.min(pic.h, ty + tile);
      let black = true;
      scan: for (let y = ty; y < y1; y++) {
        for (let x = tx; x < x1; x++) {
          const i = y * pic.stride + 4 * x;
          if (pic.bytes[i] | pic.bytes[i + 1] | pic.bytes[i + 2]) { black = false; break scan; }
        }
      }
      if (black) n++;
    }
  }
  return n;
}

// The black regions of a picture (Issue #1483): the pixels whose R, G and B are all 0 (pure black; the desktop's own colour is #05070a, which is not counted), in 4-connected groups.
// A group is big when its bounding box is at least `min` pixels wide and at least `min` pixels high. `pic` is decodePicture's { w, h, stride, bytes } (the rows may be longer than w x 4).
export function blackRegions(pic, min = 100) {
  const { w: width, h: height } = pic;
  const n = width * height;
  const black = new Uint8Array(n);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const i = y * pic.stride + 4 * x;
    black[y * width + x] = pic.bytes[i] === 0 && pic.bytes[i + 1] === 0 && pic.bytes[i + 2] === 0 ? 1 : 0;
  }
  const seen = new Uint8Array(n);
  const stack = new Int32Array(n);
  let groups = 0, blackPixels = 0;
  const big = [];
  for (let start = 0; start < n; start++) {
    if (!black[start] || seen[start]) continue;
    let top = 0, count = 0;
    let x0 = width, y0 = height, x1 = -1, y1 = -1;
    stack[top++] = start;
    seen[start] = 1;
    while (top > 0) {
      const p = stack[--top];
      const x = p % width, y = (p - x) / width;
      count++;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
      if (x > 0 && black[p - 1] && !seen[p - 1]) { seen[p - 1] = 1; stack[top++] = p - 1; }
      if (x < width - 1 && black[p + 1] && !seen[p + 1]) { seen[p + 1] = 1; stack[top++] = p + 1; }
      if (y > 0 && black[p - width] && !seen[p - width]) { seen[p - width] = 1; stack[top++] = p - width; }
      if (y < height - 1 && black[p + width] && !seen[p + width]) { seen[p + width] = 1; stack[top++] = p + width; }
    }
    groups++;
    blackPixels += count;
    if (x1 - x0 + 1 >= min && y1 - y0 + 1 >= min) big.push({ x0, y0, x1, y1, w: x1 - x0 + 1, h: y1 - y0 + 1, pixels: count });
  }
  big.sort((a, b) => b.pixels - a.pixels);
  return { groups, blackPixels, big: big.slice(0, 8), bigCount: big.length };
}
