import json,sys,glob
names={'0':'EnterCS','1':'WaitSO','2':'AcqSRW','3':'entry','4':'LoadLibW','5':'ReadFile','6':'CreateFileW'}
for f in sys.argv[1:]:
    d=json.load(open(f))
    e=(d['error'] or {}).get('message')
    line=f"{f.split('/runs/')[-1][:-5]:28s} exit={d['exitCode']} err={e} rec={d.get('records')}"
    for k,v in sorted((d.get('byKind') or {}).items()):
        t=v['total_us']; j=v['js_us']
        line+=f" | {names[k]} n={v['n']} total p50/p95/p99/max={t['p50']:.1f}/{t['p95']:.1f}/{t['p99']:.1f}/{t['max']:.1f} js p50={j['p50']:.1f} nland={v['nland']} cb={v['cbdepth']}"
    print(line)
