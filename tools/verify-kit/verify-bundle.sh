#!/bin/bash
# verify-bundle.sh MODE ...   (decision 0277: the check of a new production bundle; every value is written with the load and the I/O pressure of its moment)
#   fetch BUNDLE            the bundle from R2 (read only), sha256 and size of every asset against the manifest, NAME.br next to each, assets-BUNDLE/
#   page REF                the page of git ref REF (= the Worker's app) built in page-REF/ (the vite cache is not in app/node_modules)
#   pagedir NAME DIR        the page of a working tree DIR (its app/, e.g. a page with a change that is not in git) built as page-NAME/ (same build as `page`); serve with serve.sh PORT BUNDLE NAME
#   proj                    kit/proj = the project files served at /assets/proj/ (reference project, its zip, Sampler.flp + SHP_Sawing Synth.wav)
#   preflight               the project files are served (200 and the right size; a 404 body would be opened as a project, 0277); DISPLAY=:77 + WebGL: one Firefox first idle on the new bundle; the beacon must say webglRenderer != null and desk.mode = gl (else exit 2 and stop)
#   scenes chrome|firefox [SCENE]   the 11 scenes (+ more plugins, add menu fade, picker fades) on the new bundle; File x100 is d8
#   alpha7                  the main window geometry + a screenshot (Chrome, guest 1920x1080)
#   minrestore              E8 only (alpha10 / alpha11-app): the main window minimized with its own button, #taskbar-restore shown and pressed, the main window back (Chrome); window tables and pictures at the three moments
#   title                   E8/E9: the repeat of minimize -> restore in 5 variants (DOM click after 0.5 / 3 / 15 s, Tab+Enter, a real CDP mouse click on #taskbar-restore) with the window titles before / minimized / after (the playlist title '(none)' question)
#   logo [new|prev [N]]     E9: the window table (sizes of lid 8, the logo window) and pictures just after the first idle with the welcome wizard open, 2.5 s later, and after closeWizard (Chrome)
#   idle chrome|firefox [N] N (6) rounds of single first idles, the previous and the new bundle alternating (cold, a new profile each), load and I/O pressure at the start and the end of each, the beacon's webglRenderer and desk.mode
#   export                  Chrome: the reference project opened, Ctrl+R, Start; the WAV's size and sha256 (expected 22863828 B, 381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979)
#   play chrome|firefox [new|prev [SUFFIX]]  the reference project (zip) played 60 s: no stop, Worker busy ms/s, underrun episodes / longest run
#   p3|pure chrome|firefox [R]   (env P3START=N starts the round numbering at N, e.g. for extra pairs; pure = the same mode under the name of the E9 check: pure playback with no input, E8 vs E9 alternating: UI idles/s, presents/s, lagCuts/s, virtual s per real s, write rate, underrun, Worker CPU, in 10 s windows; every run waits for a quiet window: load <=8 and I/O pressure 0, up to MAXWAIT (240) min; the wait is written to wait.txt of the run and shown in the table)
#   p3 chrome|firefox [R]   the P3 decider (0277): R (6) rounds of (previous, new) alternating, each run = the reference project (zip) played 60 s, audio counters every 10 s + Unlock / consumed rates; then: python3 tools/p3-win.py BROWSER  -> out/P3-BROWSER.md (a run counts only if the load at its start is <=8 and the I/O pressure is 0)
#   lat chrome|firefox [R]  E9: the input latency T (page sent -> presented) of the File click and Esc during playback and at idle, previous and new alternating R (3) rounds (gpu7's scenario scn-lat-ff.json); then python3 tools/lat-ab.py BROWSER -> out/LAT-BROWSER.md
#   long [chrome|firefox [new|prev [N]]]  a 5 minute playback (the reference zip, Space, 300 s) with the audio counters every 10 s and the pulses / Unlocks; analyse with: P3PREFIX=long P3WINS=30 python3 tools/p3-win.py BROWSER
#   ab chrome|firefox [R]   the Mixer header drag (1920x1080) Worker ms per move, the previous bundle and the new one alternating R (2) rounds
#   report                  out/RESULTS.md from out/*
#   clean                   removes bundles, builds and profiles (keeps out/)
# order: (Xvfb: exec ./start-xvfb.sh in the background) fetch NEW; fetch PREV; page REF; (exec ./serve.sh 5040 NEW REF; exec ./serve.sh 5041 PREV REF in the background); preflight; scenes chrome / scenes firefox; alpha7; idle chrome / idle firefox; play; ab; export; report; clean
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
NEW=${NEW:-}; PREV=${PREV:-fl-engine-ci-ff5ee429}; export NEW PREV REF WORKER   # NEW, REF (git ref of the page), WORKER (version) are set by the operator
mode=${1:-help}; shift
log() { echo "$(date +%T) $* load $(cut -d' ' -f1 /proc/loadavg) io $(io)" | tee -a $K/out/steps.log; }
need_x() { [ -S /tmp/.X11-unix/X77 ] || { echo "NO X SERVER :77: start it first (exec ./start-xvfb.sh in the background). Firefox without WebGL gives other values: stop."; exit 2; }; }
case "$mode" in
fetch)
  b=$1; d=$K/bundles/$b; mkdir -p $d; rm -rf $K/assets-$b; mkdir -p $K/assets-$b
  wrangler r2 object get "fl-web-assets/$b/manifest.json" --remote --file $d/manifest.json 2>&1 | grep -E "complete|rror"
  for f in $(python3 $K/tools/bundle-files.py $d); do wrangler r2 object get "fl-web-assets/$b/$f" --remote --file $d/$f 2>&1 | grep -E "rror"; done
  python3 $K/tools/bundle-files.py $d verify | tee $K/out/fetch-$b.txt; [ ${PIPESTATUS[0]} -eq 0 ] || { echo "MANIFEST MISMATCH"; exit 1; }
  node $K/tools/mkbr.mjs $d
  ln -sfn $d $K/assets-$b/$b; cp $d/manifest.json $K/assets-$b/manifest.json; ln -sfn $K/proj $K/assets-$b/proj
  log "fetched $b ($(du -sh $d | cut -f1))" ;;
page)
  ref=$1; p=$K/page-$ref; rm -rf $p; mkdir -p $p; (cd $REPO && git archive $ref app | tar -x -C $p)
  ln -sfn $REPO/app/node_modules $p/app/node_modules
  cat > $p/app/vite.kit.config.ts <<'CFG'
import { defineConfig, mergeConfig } from 'vite';
import base from './vite.config.ts';
export default defineConfig(async (env) => { const b = typeof base === 'function' ? await base(env) : await base; return mergeConfig(b, { cacheDir: process.env.KIT_VITE_CACHE }); });
CFG
  (cd $p/app && node_modules/.bin/vite build --config vite.kit.config.ts 2>&1 | tail -n 2)
  log "built the page of $ref" ;;
pagedir)
  name=$1; src=$2; p=$K/page-$name; rm -rf $p; mkdir -p $p/app; (cd $src/app && tar -c --exclude=node_modules --exclude=dist . | tar -x -C $p/app)
  ln -sfn $REPO/app/node_modules $p/app/node_modules
  cat > $p/app/vite.kit.config.ts <<'CFG'
import { defineConfig, mergeConfig } from 'vite';
import base from './vite.config.ts';
export default defineConfig(async (env) => { const b = typeof base === 'function' ? await base(env) : await base; return mergeConfig(b, { cacheDir: process.env.KIT_VITE_CACHE }); });
CFG
  (cd $p/app && node_modules/.bin/vite build --config vite.kit.config.ts 2>&1 | tail -n 2)
  log "built the page of the tree $src as page-$name" ;;
proj)
  rm -f $K/proj 2>/dev/null; [ -d $K/proj ] && find $K/proj -maxdepth 1 -type l -delete; mkdir -p $K/proj
  for f in $REPO/lift/out-gpu2/refproj/*; do ln -sf "$f" $K/proj/; done
  ln -sf $REPO/lift/out-ci1/c5/assets/proj/serina1-serum2.zip $K/proj/
  ln -sf "$REPO/docs/investigations/crash1/tools/Sampler-project/Sampler.flp" "$REPO/docs/investigations/crash1/tools/Sampler-project/SHP_Sawing Synth.wav" $K/proj/
  log "proj: $(ls $K/proj | wc -l) files" ;;
preflight)
  need_x
  for t in "Sampler.flp:81831" "SHP_Sawing%20Synth.wav:8336" "serina1-serum2.zip:53489451" "serina1-serum2.flp:1322619"; do n=${t%%:*}; sz=${t##*:}; r=$(curl -s -o /dev/null -w "%{http_code} %{size_download}" http://127.0.0.1:$NEWPORT/assets/proj/$n); [ "$r" = "200 $sz" ] || { log "PREFLIGHT FAILED: /assets/proj/$n gives '$r' (expected 200 $sz): the input files of the scenes are missing (a 404 body would be opened as a project): stop"; exit 3; }; done
  out=$K/out/preflight; rm -rf $out; mkdir -p $out; skip=$(cat $K/out/perf-$NEWPORT.jsonl 2>/dev/null | wc -l)
  node $K/tools/entry-check.mjs --browser firefox --url http://127.0.0.1:$NEWPORT/ --app $K/page-$REF/app --beacon-wait-s 3 --out $out > $out.log 2>&1
  python3 $K/tools/beacon.py $K/out/perf-$NEWPORT.jsonl $skip | tee $K/out/preflight.txt
  rc=${PIPESTATUS[0]}
  [ $rc -eq 0 ] && log "preflight ok" || { log "PREFLIGHT FAILED: no WebGL in Firefox (rc $rc): stop and report"; exit 2; } ;;
scenes)
  b=$1; only=${2:-}; out=$K/out/scenes-$b; mkdir -p $out; [ $b = firefox ] && { need_x; vp=2260x1202; } || vp=2260x1116
  run() { d=$out/$1; rm -rf $d; mkdir -p $d; shift; log "scenes $b $1 ${2:-}"; node $K/tools/sdrive-a1.mjs --browser $b --url http://127.0.0.1:$NEWPORT/ --out $d --app $K/page-$REF/app --scenarios $K/tools/$1 ${2:+--only $2} --viewport $vp > $d.log 2>&1; }
  if [ $b = firefox ]; then
    for s in ${only:-d1-sampler-envelope d2-browser-folder d3-settings-all-tabs d4-playlist-middle-drag d5-mixer-front d6-playlist-close d7-main-close d8-file-menu-100 d9-keys d10-pianoroll-f12 d11-menu-hover-escape}; do run ff-$s scn-client.json $s; done
    [ -z "$only" ] && { run plus scn-plus.json; run alpha scn-alpha1.json; }
  else
    if [ -n "$only" ]; then for s in $only; do run ch-$s scn-client.json $s; done; else run client scn-client.json; run plus scn-plus.json; run alpha scn-alpha1.json; fi
  fi ;;
alpha7)
  d=$K/out/alpha7; rm -rf $d; mkdir -p $d; log "alpha7"
  node $K/tools/sdrive-a1.mjs --browser chrome --url http://127.0.0.1:$NEWPORT/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-n1.json --viewport 2260x1116 > $d.log 2>&1 ;;
minrestore)
  d=$K/out/minrestore; rm -rf $d; mkdir -p $d; log "minrestore"
  node $K/tools/sdrive-a1.mjs --browser chrome --url http://127.0.0.1:$NEWPORT/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-e8-minrestore.json --viewport 2260x1116 > $d.log 2>&1; log "minrestore end" ;;
title)
  d=$K/out/title; rm -rf $d; mkdir -p $d; log "title"
  node $K/tools/sdrive-a1.mjs --browser chrome --url http://127.0.0.1:$NEWPORT/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-e8-title.json --viewport 2260x1116 > $d.log 2>&1; log "title end" ;;
logo)
  b=${1:-new}; p=$NEWPORT; [ "$b" = prev ] && p=$PREVPORT; d=$K/out/logo-$b-${2:-1}; rm -rf $d; mkdir -p $d; log "logo $b ${2:-1}"
  node $K/tools/sdrive-a1.mjs --browser chrome --url http://127.0.0.1:$p/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-e9-logo.json --viewport 2260x1116 > $d.log 2>&1; log "logo $b ${2:-1} end" ;;
idle)
  b=$1; n=${2:-6}; [ $b = firefox ] && need_x; f=$K/out/idle-$b.tsv; [ -n "${IDLESTART:-}" ] && f=$K/out/idle-$b-more.tsv; : > $f
  for i in $(seq ${IDLESTART:-1} $(( ${IDLESTART:-1} + n - 1 ))); do
    if [ $((i % 2)) = 1 ]; then order="prev:$PREVPORT new:$NEWPORT"; else order="new:$NEWPORT prev:$PREVPORT"; fi
    for v in $order; do n2=${v%%:*}; p=${v##*:}; d=$K/out/idle-$b-$n2-$i; q=$(node $K/tools/wait-quiet.mjs ${MAXLOAD:-8} ${MAXWAIT:-15}) || q="REFERENCE (start load over): $q"; log "idle $b $n2 $i ($q)"; rm -rf $d; mkdir -p $d; echo "$q" > $d/wait.txt; s="$(cut -d' ' -f1 /proc/loadavg) $(io)"; skip=$(cat $K/out/perf-$p.jsonl 2>/dev/null | wc -l)
      node $K/tools/entry-check.mjs --browser $b --url http://127.0.0.1:$p/ --app $K/page-$REF/app --beacon-wait-s 3 --out $d > $d.log 2>&1
      e="$(cut -d' ' -f1 /proc/loadavg) $(io)"
      python3 $K/tools/beacon.py $K/out/perf-$p.jsonl $skip tsv "$b" "$n2" "$i" "$s" "$e" $d >> $f
    done
  done; log "idle $b x$n" ;;
export)
  d=$K/out/export; rm -rf $d; mkdir -p $d; log "export start"
  node $K/tools/sdriveg2-export.mjs --browser chrome --url http://127.0.0.1:$NEWPORT/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-export.json --viewport 2260x1116 > $d.log 2>&1; log "export end" ;;
play)
  b=$1; p=$NEWPORT; tag=$b; [ "${2:-}" = prev ] && { p=$PREVPORT; tag=$b-prev; }; [ -n "${3:-}" ] && tag=$tag-$3; d=$K/out/play-$tag; rm -rf $d; mkdir -p $d; [ $b = firefox ] && { need_x; vp=2260x1202; } || vp=2260x1116; log "play $tag"
  node $K/tools/sdrive-a1.mjs --browser $b --url http://127.0.0.1:$p/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-play60.json --viewport $vp > $d.log 2>&1; log "play $tag end" ;;
p3|pure)
  b=$1; r=${2:-6}; [ $b = firefox ] && { need_x; vp=2260x1202; } || vp=2260x1116; : > $K/out/p3-$b$([ -n "${P3START:-}" ] && echo -more).tsv
  for i in $(seq ${P3START:-1} $(( ${P3START:-1} + r - 1 ))); do
    if [ $((i % 2)) = 1 ]; then order="prev:$PREVPORT new:$NEWPORT"; else order="new:$NEWPORT prev:$PREVPORT"; fi
    for v in $order; do n2=${v%%:*}; p=${v##*:}; d=$K/out/p3-$b-$n2-$i; q=$(node $K/tools/wait-quiet.mjs ${MAXLOAD:-8} ${MAXWAIT:-15}) || q="REFERENCE (start load over): $q"; log "p3 $b $n2 $i ($q)"; rm -rf $d; mkdir -p $d; echo "$q" > $d/wait.txt
      curl -sf -o /dev/null http://127.0.0.1:$p/ || { log "p3 $b $n2 $i: THE SERVER ON PORT $p IS DOWN: stop (restart it with serve.sh)"; exit 3; }
      node $K/tools/sdriveg2-lat.mjs --browser $b --url http://127.0.0.1:$p/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-p3.json --viewport $vp > $d.log 2>&1
      rm -rf $d/profile-sdrive $d/netlog-sdrive.json; log "p3 $b $n2 $i end"
    done
  done; log "pure end $b (all $((2 * r)) runs of the mode finished)" ;;
lat)
  b=$1; r=${2:-3}; [ $b = firefox ] && { need_x; vp=2260x1202; } || vp=2260x1116
  for i in $(seq 1 $r); do
    if [ $((i % 2)) = 1 ]; then order="prev:$PREVPORT new:$NEWPORT"; else order="new:$NEWPORT prev:$PREVPORT"; fi
    for v in $order; do n2=${v%%:*}; p=${v##*:}; d=$K/out/lat-$b-$n2-$i; q=$(node $K/tools/wait-quiet.mjs ${MAXLOAD:-8} ${MAXWAIT:-15}) || q="REFERENCE (start load over): $q"; log "lat $b $n2 $i ($q)"; rm -rf $d; mkdir -p $d; echo "$q" > $d/wait.txt
      curl -sf -o /dev/null http://127.0.0.1:$p/ || { log "lat $b $n2 $i: THE SERVER ON PORT $p IS DOWN: stop (restart it with serve.sh)"; exit 3; }
      node $K/tools/sdriveg2-lat.mjs --browser $b --url http://127.0.0.1:$p/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-lat-ff.json --viewport $vp > $d.log 2>&1
      rm -rf $d/profile-sdrive $d/netlog-sdrive.json; log "lat $b $n2 $i end"
    done
  done ;;
long)
  b=${1:-chrome}; v=${2:-new}; i=${3:-1}; p=$NEWPORT; [ "$v" = prev ] && p=$PREVPORT; [ $b = firefox ] && { need_x; vp=2260x1202; } || vp=2260x1116; d=$K/out/long-$b-$v-$i
  q=$(node $K/tools/wait-quiet.mjs ${MAXLOAD:-8} ${MAXWAIT:-15}) || q="REFERENCE (start load over): $q"; log "long $b $v $i ($q)"; rm -rf $d; mkdir -p $d; echo "$q" > $d/wait.txt
  curl -sf -o /dev/null http://127.0.0.1:$p/ || { log "long: THE SERVER ON PORT $p IS DOWN: stop"; exit 3; }
  node $K/tools/sdriveg2-lat.mjs --browser $b --url http://127.0.0.1:$p/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-p3-300.json --viewport $vp > $d.log 2>&1; rm -rf $d/profile-sdrive $d/netlog-sdrive.json; log "long $b $v $i end" ;;
ab)
  b=$1; r=${2:-2}; [ $b = firefox ] && { need_x; vp=2260x1202; } || vp=2260x1116; : > $K/out/ab-$b.tsv
  for i in $(seq 1 $r); do for v in prev:$PREVPORT new:$NEWPORT; do n=${v%%:*}; p=${v##*:}; d=$K/out/ab-$b-$n-$i; rm -rf $d; mkdir -p $d; s="$(cut -d' ' -f1 /proc/loadavg) $(io)"
    node $K/tools/sdrive-a1.mjs --browser $b --url http://127.0.0.1:$p/ --out $d --app $K/page-$REF/app --scenarios $K/tools/scn-move1920.json --only move-1920 --viewport $vp > $d.log 2>&1
    python3 - <<PY >> $K/out/ab-$b.tsv
import json
r=json.load(open('$d/report.json'))[0]; m=[x for x in (r.get('measures') or []) if x.get('workerMsPerMove') is not None]
print('\t'.join(map(str,['$b','$n',$i,m[0]['workerMsPerMove'] if m else None,'start $s'])))
PY
  done; done; log "ab $b x$r" ;;
report) python3 $K/tools/report.py ;;
clean)
  for t in bundles page-* assets-* ffprof tmp appwork vite-cache; do for x in $K/$t; do r=$(readlink -f "$x"); case "$r" in $K/*) rm -rf "$r"; echo "removed $t";; esac; done; done; mkdir -p $K/ffprof $K/tmp $K/appwork ;;
*) sed -n 2,17p $0 ;;
esac
