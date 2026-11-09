// Runs the wasm build of the VST3 harness under node (decision 0156): node run_vst_node.cjs <dir with vst.js> <plugin file (rebased PE)> [harness args...]
// NODERAWFS gives the guest the host file system (the plugin file is read by the harness; FLE_VFS_SEED is read from the environment).
const path = require('path');
const dir = process.argv[2];
const args = process.argv.slice(3);
require(path.resolve(dir, 'vst.js'))({ arguments: args, print: (s) => process.stdout.write(s + '\n'), printErr: (s) => process.stderr.write(s + '\n') }).catch((e) => { console.error('wasm run failed:', e && e.message ? e.message : e); process.exit(1); });
