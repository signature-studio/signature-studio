#!/bin/bash
# ci/stage-app.sh install|check|dryrun - the stage "Workers build" (the front end and the Cloudflare Worker in app/).
#   install: npm ci (the lockfile decides the versions; wrangler is a devDependency of app/)
#   check:   npm run build (type check of the page, the engine Worker, the tests and the Cloudflare Worker; then vite build into app/dist), the unit tests (vitest), and the Worker tests
#            (npm run test:worker: the Worker under `wrangler dev --local`: Range, ETag/304, Content-Encoding, headers, the Access JWT check, the default-manifest refusal). The browser tests (test:e2e)
#            need Chrome and are not run here.
#   dryrun:  `wrangler deploy --dry-run --outdir`: the bundle the Worker deploy would upload, without a credential. The configuration is the one stage-deploy.sh config writes.
set -u -o pipefail
. "$(dirname "$0")/lib/common.sh"
export WRANGLER_SEND_METRICS=false CI=1
cd "$REPO/app" || exit 2
case "${1:-}" in
  install) npm ci --no-audit --no-fund ;;
  check)   npm run build && npm test && npm run test:worker ;;
  dryrun)
    bash "$CI_DIR/stage-deploy.sh" config
    npx wrangler deploy --config wrangler.deploy.json --dry-run --outdir "$CI_WORK/worker-dry-run" ;;
  *) die "usage: stage-app.sh install|check|dryrun" ;;
esac
