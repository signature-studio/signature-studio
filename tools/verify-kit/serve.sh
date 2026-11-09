#!/bin/bash
# serve.sh PORT BUNDLE PAGEREF: vite preview of the page built from PAGEREF with FL_ASSETS_DIR = the assets of BUNDLE (run it in the background with exec, one per port)
. /home/ubuntu/work/signature-studio-develop/lift/out-ci1/kit/env.sh
cd $K/page-$3/app || exit 1
export FL_ASSETS_DIR=$K/assets-$2 FL_PERF_LOG=$K/out/perf-$1.jsonl
exec node_modules/.bin/vite preview --config vite.kit.config.ts --host 127.0.0.1 --port $1 --strictPort
