#!/bin/bash
# usage: suite.sh <name> <measure.mjs args...>   run one case and print the one-line summary
export PATH=/home/ubuntu/.local/bin:$PATH
cd "$(dirname "$0")"
n=$1; shift
node measure.mjs run name=$n root=/tmp/opencode/web/dist/data "$@" > /tmp/opencode/web/runs/$n.summary 2>&1
python3 - "$n" <<'PY'
import json,sys
n=sys.argv[1]
try:
    d=json.load(open('/tmp/opencode/web/runs/%s.json'%n))
except Exception as e:
    print(n,'NO JSON',e); sys.exit()
r=d.get('result') or {}
m=r.get('marks') or {}
w=r.get('wasmDetail') or {}
b=d['rss']['busiestRenderer'] or {}
c=d.get('compare_with_node') or {}
def g(k): return m.get(k)
print(n,'done=%s exit=%s err=%s'%(d['completed'],r.get('exitCode'),(r.get('error') or '')[:80]),
 'lines=%s diff=%s'%(c.get('browser_lines'),c.get('differing_lines')),
 'wasmstart=%s inst=%s mainstart=%s mainend=%s'%(g('wasm_start'),g('wasm_instantiated'),g('main_start'),g('main_end')),
 'main_s=%s'%(round((g('main_end')-g('main_start'))/1000,2) if g('main_end') and g('main_start') else None),
 'detail=%s'%w, 'renderer_peak_MB=%s pss_MB=%s tree_MB=%s'%(round(b.get('peakRssKB',0)/1024),round(b.get('peakPssKB',0)/1024),round(d['rss']['peakTreeRssKB']/1024)),
 'cpu_s=%s thr=%s'%(round((b.get('cpuMs') or 0)/1000,1),b.get('threadsMax')),'srv=%s'%[(s['enc'],s['ms']) for s in d['server_log'] if 'wasm' in s['url']], 'crash=%d timeout=%s'%(len(d['crashed']),d['timed_out']))
PY
