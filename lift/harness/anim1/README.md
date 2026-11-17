# lift/harness/anim1 — differential tests of the WebAssembly SIMD path of the packed-integer SSE helpers (decision 0225)

`lift/runtime/fle_sse.h`, `fle_sse_int2.h` and `fle_sse_more.c` implement the packed integer SSE2 instructions of the lifted code (PADDUSB, PSUBUSB, PMULHW, PMULHUW, PACKUSWB, PUNPCK*, the shifts, ...) as lane loops. When the file is compiled with `-msimd128` (the opt-in `FLE_WASM_SIMD=1` of `app/scripts/build-engine-bundle.sh`, or `SHIP_PRESET=F1xH3|F2xH3`) each of them is one WebAssembly SIMD instruction instead (`FLE_WASM_SIMD` is defined in `fle_sse.h`; `-DFLE_NO_WASM_SIMD` keeps the lane loops in a `-msimd128` build). Without `-msimd128` nothing changes.

The two programs here are built twice, from the same source: natively by gcc (the lane loops = the reference) and by emcc with `-msimd128` (the SIMD path), and print one hash per operation; the two outputs have to be identical (apart from the `# path:` line).

```
cd lift
gcc   -O1 -w -I runtime harness/anim1/sse_simd_difftest.c  runtime/fle_sse_more.c -o ref -lm        && ./ref 1500000 > ref.txt
emcc  -O2 -w -msimd128 -I runtime harness/anim1/sse_simd_difftest.c runtime/fle_sse_more.c -o new.js -sEXIT_RUNTIME=1 && node new.js 1500000 > new.txt
diff ref.txt new.txt          # only the "# path:" line differs
# the exhaustive part (97 s native, 16 s in node on the author's machine): sse_simd_exhaustive.c, same two commands; `quick` as the argument uses every 16th word
```

- `sse_simd_difftest.c`: 49 operations x 1,500,000 pseudo-random inputs (each byte 0x00, 0x7f, 0x80, 0xff or random, so that every saturation edge is hit).
- `sse_simd_exhaustive.c`: every 8-bit lane operation for all 65,536 byte pairs; every 16-bit lane operation for all 2^32 word pairs (PMULHW, PMULHUW, PADDSW, PSUBSW, PADDUSW, PSUBUSW, PMAXSW, PMINSW, PMULLW, PSUBW, PADDW, PCMPEQW, PCMPGTW); PACKUSWB / PACKSSWB for all 65,536 words; PACKSSDW for 2^26 strided doublewords plus the neighbourhood of every saturation edge; PMADDWD for every combination of 27 boundary words in four lanes; 32/64-bit lane operations for every combination of 27 boundary values; the shifts for all 65,536 words x counts 0..130 and the huge counts; PUNPCK for all byte pairs.
- A deliberately wrong SIMD expression (PADDSW as a saturating subtract, PACKUSWB as a signed pack) is caught by the random program.
