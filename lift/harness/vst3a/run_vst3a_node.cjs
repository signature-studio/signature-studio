// Decision 0177 (srm1): runs the wasm build of the Serum 2 harness under node: node run_vst3a_node.cjs <dir with vst.js> <plugin file (rebased PE)> [harness args...]
// NODERAWFS gives the guest the host file system (the plugin file, the state files and the VFS seed list FLE_VFS_SEED are read through it). Same as vst/run_vst_node.cjs.
const path = require('path');
const dir = process.argv[2];
const args = process.argv.slice(3);
require(path.resolve(dir, 'vst.js'))({ arguments: args, print: (s) => process.stdout.write(s + '\n'), printErr: (s) => process.stderr.write(s + '\n') }).catch((e) => { console.error('wasm run failed:', e && e.message ? e.message : e); process.exit(1); });
