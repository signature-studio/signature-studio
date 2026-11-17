const KERNEL_SP = [0xE0000, 0xF0000];
self.onmessage = (ev) => {
  const { module, memory, idx } = ev.data;
  const t0 = performance.now();
  const inst = new WebAssembly.Instance(module, { env: { memory } });
  const ex = inst.exports;
  ex.__wasm_init_tls(ex.tls_block_addr(idx));
  ex.__stack_pointer.value = KERNEL_SP[idx - 1];
  self.postMessage({ ready: true, instantiate_ms: performance.now() - t0 });
  ex.helper_loop(idx - 1);
};
