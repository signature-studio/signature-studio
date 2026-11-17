# runs-on/cache on Cloudflare R2: primary sources and what was tried (2026-11-04)

## Sources (read on 2026-11-04)

* https://github.com/runs-on/cache (README, tag v5.0.7 = commit 88d90644011a3a9957fd141a106f5a94f9794203, 2026-05-13). Text that matters:
  * "It will automatically store your caches in a dedicated RunsOn S3 bucket … If no S3 bucket is provided, it will also transparently switch to the default behaviour." -> an empty bucket variable means GitHub's cache, silently.
  * "Usage outside RunsOn: setup your AWS credentials … `RUNS_ON_S3_BUCKET_CACHE: name-of-your-bucket`".
  * "`RUNS_ON_S3_BUCKET_ENDPOINT`: … a non-AWS S3-compatible service." "`RUNS_ON_RUNNER_NAME`: when running on RunsOn, where this environment variable is non-empty, existing AWS credentials from the environment will be discarded. If you want to preserve existing environment variables, set this to the empty string." "`RUNS_ON_S3_FORCE_PATH_STYLE` or `AWS_S3_FORCE_PATH_STYLE` … `"true"`".
* `src/custom/backend.ts` of the same tag (read): region = `RUNS_ON_AWS_REGION || AWS_REGION || AWS_DEFAULT_REGION` (a RunsOn runner sets its own region first: the wrapper sets `RUNS_ON_AWS_REGION=auto`); `new S3Client({region, forcePathStyle, endpoint})`; entry lookup = `ListObjectsV2` with the prefix `cache/<GITHUB_REPOSITORY>/<sha256 of paths>/<key>` (a prefix match: the keys of this repository have one fixed length per stage, so none is the prefix of another);
  upload = `@aws-sdk/lib-storage` `Upload` (multipart, 32 MiB parts, 4 in flight); download = presigned `GetObject` URL + ranged GETs of 16 MiB (8 in flight), 5 retries of 30 s per segment.
  `restore` with `lookup-only: true` returns after the listing (no download).
* https://developers.cloudflare.com/r2/platform/limits/: object size 5 TiB; single upload 5 GiB; multipart parts max 10,000; "maximum concurrent writes to the same object name: 1 per second".
* https://developers.cloudflare.com/r2/examples/aws/aws-cli/: endpoint `https://<ACCOUNT_ID>.r2.cloudflarestorage.com`; "The `region` value can be set to `auto`".
* https://developers.cloudflare.com/r2/pricing/: Standard $0.015/GB-month; Class A $4.50/M; Class B $0.36/M; egress free; free 10 GB-month / 1 M / 10 M.

## What was run (action `dist/` of v5.0.7 outside Actions with the Actions environment faked, then inside `act` in host mode with the real composite action)

Token: an account-owned R2 token limited to `fl-web-cache` (Object Read & Write), created with `cf accounts tokens create` for the test, 3-hour expiry, revoked afterwards. Endpoint `https://<account>.r2.cloudflarestorage.com`, region `auto`, path style.

| test | result |
|---|---|
| restore of an absent key | clean miss (no error) |
| save 3 MB (one request) | saved |
| save 150 MB incompressible (5 multipart parts of 32 MiB) | saved |
| restore of it into an empty directory | restored, SHA-256 of all files equal to the originals |
| lookup-only of an existing key | `cache-hit=true`, no download |
| the composite action (save, verify by lookup, summary row), 17 MB `vst3` entry, inside act | "saved + verified" |
| the plan job: 10 lookups of absent keys | 10 clean misses |
| AWS SDK checksum headers (`x-amz-checksum-*`) | accepted by R2 in all of the above; no `requestChecksumCalculation` override was needed |

Observed once and not explained: for about ten minutes every ranged GET of the last 16 MiB segment of the 150 MB entry stalled after 6-8 MB (curl over HTTP/2 and the action's node client both; the action then reports `Failed to restore … downloadSegmentRetry failed due to timeout` as a warning and treats it as a miss). Afterwards the same URLs returned in 0.2 s and the restore worked; a full-object GET over HTTP/1.1 took 1.3 s. The machine was at load 45 meanwhile, so a local cause is possible. Consequence in the workflow: a failed restore is a counted miss (`ci/summary.sh` prints the number), not a silent rebuild.
