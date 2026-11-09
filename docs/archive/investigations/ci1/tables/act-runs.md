# Workflow runs with nektos/act 0.2.96 (host execution mode: `-P ubuntu-24.04=-self-hosted`, this machine, no container) against the real R2 buckets

GitHub itself refuses to dispatch (`HTTP 403 Resource not accessible by integration`), so these are NOT GitHub runs. `act` runs the real workflow file, the real composite action `ci/actions/r2-cache`, the real
`runs-on/cache/restore|save` v5.0.7 and `cloudflare/wrangler-action` v4.1.3; the runner is this workstation instead of a GitHub / RunsOn runner. Variables and secrets came from files (`--var-file`, `--secret-file`; values not recorded).
In `act-ws` (a clone of the branch) the Rust step is skipped and `RUST_TOOLCHAIN=stable` (the host has 1.99.0 under that name; no toolchain is installed or changed on the host). `GITHUB_REPOSITORY` is a local path under act, so the cache entries are
namespaced `cache/<local path>/…` and are never read by a real run (they were deleted afterwards).

| job | what ran | result |
|---|---|---|
| plan | static checks (`bash -n`, shellcheck, py_compile, `artifacts.lock` format), `ci/check-config.sh`, `ci/plan.sh`, runner JSON, 10 `lookup` steps against R2, `ci/needs.sh`, cache table | succeeded (twice: cold = 10 misses) |
| vst3 | 3 installers from the artifact bucket (read-only token, 1.36 GB), ETag/size/SHA-256 check, VST3 extraction, `save` + verify | succeeded, 20 s + 6 s |
| app | npm ci, type check, vite build, vitest (1,290 passed + 7 skipped), `npm run test:worker`, `save` | 1st run failed 3 Worker tests (my rename of the bucket in `app/wrangler.jsonc`, reverted); 2nd run failed on a port collision with my own leftover `wrangler dev`; 3rd run succeeded |
| build | (1) cold: extract, felift, b2d, lift, objects (7 groups), link, seed, bundle all `run -> save -> verify` (`logs/act-build1.summary.txt`); (2) second `plan`: every lookup hit, `need_build=false`; (3) partial rebuild (`act-build2`, the cache entries of `bundle`, `link` and the `dsp` objects removed from R2): `plan` finds bundle/link/dsp miss and everything else hit, the build job restores the hits, compiles only the 12 `dsp` units, relinks and rebundles (`logs/act-build2.summary.txt`) | 1 and 3 succeeded. (3): dsp objects 3m54s (234 s), link 8m19s (wasm-opt included), bundle 7m16s, every save verified; the wasm SHA-256 `1efa6a97...` and the sizes of all `.br` files are identical to the cold run (the same inputs gave the same output after a partial rebuild). Cold run: objects 22m19s (CI_JOBS=2), link 11m25s |
| wrangler-action | `deploy --config wrangler.deploy.json --dry-run --outdir …` without a token | succeeded: "using pre-installed wrangler version 4.156.0", bindings listed, "--dry-run: exiting now" |
