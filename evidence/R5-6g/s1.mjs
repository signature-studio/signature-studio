// R5-6g: the census sums and the S1 estimate (evidence/R5-6.md §5 formula). Read only.
// usage: node evidence/R5-6g/s1.mjs FFMPEG_DIR HOST_LIFT_DIR
//   FFMPEG_DIR: the 8 extracted DLLs (installer 26.1.7.5653). HOST_LIFT_DIR: the lift/ of the r56e host (the 10 current real modules).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const [ffDir, hostLift] = process.argv.slice(2);
const dlls = ['ILVideo_x64', 'avutil-56', 'avcodec-58', 'avformat-58', 'swresample-3', 'swscale-5', 'avdevice-58', 'avfilter-7'];
const outVal = (text, key) => Number(text.match(new RegExp(`^${key}\\s+(\\d+)`, 'm'))[1]);
const sizeOfImage = (f) => {
  const b = fs.readFileSync(f);
  return b.readUInt32LE(b.readUInt32LE(0x3c) + 24 + 56);
};

let nat = 0, c = 0, soi8 = 0, unsup = 0;
console.log('8 DLLs: file bytes, c_bytes (census), SizeOfImage, unsupported_total');
for (const n of dlls) {
  const bytes = fs.statSync(path.join(ffDir, `${n}.dll`)).size;
  const cb = outVal(fs.readFileSync(path.join(here, 'census-out', `${n}.out`), 'utf8'), 'c_bytes');
  const ut = outVal(fs.readFileSync(path.join(here, 'census-out', `${n}.out`), 'utf8'), 'unsupported_total');
  const so = sizeOfImage(path.join(ffDir, `${n}.dll`));
  nat += bytes; c += cb; soi8 += so; unsup += ut;
  console.log(`  ${n}.dll ${bytes} ${cb} ${so} ${ut}`);
}
console.log(`sum native ${nat}  sum c_bytes ${c}  sum SizeOfImage ${soi8}  sum unsupported ${unsup}`);

const cur = ['out-ext/dsp_ippv2_x64.dll', 'out-blend2d/Blend2D.dll', 'out-qfc/quickfontcache_x64.dll', 'out-wallQ/freetype/freetype_x64.dll',
  'out-inventory/extracted/ILTools_x64.dll', 'out-plugins/fruity_balance_x64.dll', 'out-plugins/fruity_filter_x64.dll',
  'out-plugins/fruity_delay2_x64.dll', 'out-sampledec/elastique_x64.dll', 'out-sampledec/oggio_x64.dll'];
let socur = 0;
for (const p of cur) socur += sizeOfImage(path.join(hostLift, p));
console.log(`current 10 real modules: sum SizeOfImage ${socur}`);
console.log(`after adding the 8: ${socur + soi8} (${((socur + soi8) / 1048576).toFixed(1)} MiB); current ${(socur / 1048576).toFixed(1)} MiB`);

// S1 (R5-6 §5): wasm / (FLEngine + real module native sum). Inputs: R5-6a wasm, decision 0408 denominator, R5-6 §5 coefficients.
const wasm = 165776779, denom = 88674176;
const kC = 15375721 / (36346414 + 37078504), kN = 15375721 / 7499264;
const ratio = (w, d) => (w / d).toFixed(4);
const dsp0 = outVal(fs.readFileSync(path.join(here, 'census-out', 'dsp-noprune.out'), 'utf8'), 'c_bytes');
const dsp1 = outVal(fs.readFileSync(path.join(here, 'census-out', 'dsp-prune.out'), 'utf8'), 'c_bytes');
const keep = dsp1 / dsp0;
const D = denom + nat;
console.log(`k_C ${kC.toFixed(6)}  k_N ${kN.toFixed(4)}  base ratio ${ratio(wasm, denom)}  denominator with the 8: ${D}`);
console.log(`A  C-type   increment ${Math.round(kC * c)} wasm ${Math.round(wasm + kC * c)} ratio ${ratio(wasm + kC * c, D)}`);
console.log(`A' native-type increment ${Math.round(kN * nat)} wasm ${Math.round(wasm + kN * nat)} ratio ${ratio(wasm + kN * nat, D)}`);
console.log(`C  break-even C-type: C <= ${Math.round((2 * D - wasm) / kC)} (cut ${(100 * (1 - (2 * D - wasm) / kC / c)).toFixed(2)}%)`);
console.log(`B  dsp prune retention ${keep.toFixed(6)} (${dsp1}/${dsp0}); C-type increment ${Math.round(kC * c * keep)} wasm ${Math.round(wasm + kC * c * keep)} ratio ${ratio(wasm + kC * c * keep, D)}`);
const c7 = c - outVal(fs.readFileSync(path.join(here, 'census-out', 'avfilter-7.out'), 'utf8'), 'c_bytes');
const n7 = nat - fs.statSync(path.join(ffDir, 'avfilter-7.dll')).size;
console.log(`D  without avfilter-7: C ${c7} C-type increment ${Math.round(kC * c7)} wasm ${Math.round(wasm + kC * c7)} ratio ${ratio(wasm + kC * c7, denom + n7)}`);
