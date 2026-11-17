// mt1: shared read-only VFS prototype (decision 0173). 'shared': the Worker reads ranges straight from one SharedArrayBuffer into a small private buffer (what a host-side ReadFile import would do
// for a file whose bytes are not in the wasm linear memory). 'private': the Worker first copies the whole blob into its own ArrayBuffer (what the current harness does: vfs_seed puts the file
// bytes into the instance's own memory), then reads the same ranges from it.
self.onmessage = (e) => {
  const m = e.data;
  const src = new Uint8Array(m.sab);
  const t0 = performance.now();
  let blob = src;
  if (m.mode === 'private') { blob = new Uint8Array(src.length); blob.set(src); }
  const tCopy = performance.now() - t0;
  const chunk = new Uint8Array(m.chunk);          // the "wasm side" destination
  let x = (m.seed >>> 0) || 1, sum = 0;
  const t1 = performance.now();
  for (let i = 0; i < m.reads; i++) {
    x ^= x << 13; x >>>= 0; x ^= x >>> 17; x ^= x << 5; x >>>= 0;
    const off = (x % (blob.length - m.chunk)) | 0;
    chunk.set(blob.subarray(off, off + m.chunk));
    sum = (sum + chunk[0] + chunk[m.chunk - 1]) | 0;
  }
  const tRead = performance.now() - t1;
  postMessage({ type: 'done', id: m.id, tCopy, tRead, mbps: (m.reads * m.chunk / 1048576) / (tRead / 1000), sum });
  if (m.hold) { /* keep the memory alive until terminated */ self.__keep = blob; }
};
