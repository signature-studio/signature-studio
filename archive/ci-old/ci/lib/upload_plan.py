#!/usr/bin/env python3
"""The R2 upload plan of an engine bundle: from the bundle directory's upload-commands.txt (written by app/scripts/pack-assets.mjs) to the exact `wrangler r2 object put` commands for the delivery bucket.

  upload_plan.py BUNDLE_DIR BUCKET [--commands FILE] [--table FILE] [--json FILE]
                 [--execute --mode s3 --endpoint https://<account>.r2.cloudflarestorage.com]   (aws CLI; <CREDENTIAL-NAME>/<CREDENTIAL-NAME> of a token scoped to BUCKET)
                 [--execute --mode wrangler --wrangler "npx wrangler"]                         (wrangler r2 object put --remote; needs an account-wide R2 write permission or a wrangler login)

What it checks before it prints anything: every file the commands name is in BUNDLE_DIR, its SHA-256 equals SHA256SUMS, its size is below wrangler's 300 MiB limit for `r2 object put`,
the manifest names only keys that this plan uploads, and the engine kind is emscripten-fruity-v1 (the default manifest is for the real engine only: decision 0152). The bundle's own manifest.json is
also planned as the bucket's DEFAULT manifest (key manifest.json) and goes LAST, so the site never points at objects that are not there yet.
Why s3 is the mode of the workflow: `wrangler r2 object put --remote` calls the REST API, which a bucket-scoped R2 token is refused for (403, tested 2026-11-04); only an account-wide "Workers R2 Storage Write"
token works, and that one can write every bucket of the account. The S3 API accepts a token that is limited to one bucket.
Without --execute nothing is uploaded and no credential is needed. Local file paths in upload-commands.txt (the machine that built the bundle) are replaced by BUNDLE_DIR.
"""
import hashlib
import json
import os
import shlex
import sys

LIMIT = 300 * 1024 * 1024 - 1


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for c in iter(lambda: f.read(1 << 20), b""):
            h.update(c)
    return h.hexdigest()


def parse(bundle):
    sums = {}
    with open(os.path.join(bundle, "SHA256SUMS")) as f:
        for ln in f:
            if ln.strip():
                d, n = ln.rstrip("\n").split("  ", 1)
                sums[n] = d
    items = []
    with open(os.path.join(bundle, "upload-commands.txt")) as f:
        for ln in f:
            if not ln.startswith("wrangler r2 object put "):
                continue
            a = shlex.split(ln)
            target = a[4]
            key = target.split("/", 1)[1]
            opt = {}
            i = 5
            while i < len(a):
                if a[i] == "--remote":
                    i += 1
                else:
                    opt[a[i]] = a[i + 1]; i += 2
            fname = os.path.basename(opt["--file"])
            items.append(dict(key=key, file=fname, ct=opt["--content-type"], ce=opt.get("--content-encoding"), cc=opt["--cache-control"]))
    return items, sums


def main(argv):
    args = [a for a in argv[1:]]
    if len(args) < 2:
        print(__doc__); return 64
    bundle, bucket = args[0], args[1]
    def opt(name, default=None):
        return args[args.index(name) + 1] if name in args else default
    items, sums = parse(bundle)
    if not items:
        print("upload_plan: no `wrangler r2 object put` line in %s/upload-commands.txt" % bundle, file=sys.stderr); return 2
    man = json.load(open(os.path.join(bundle, "manifest.json")))
    if man.get("engine", {}).get("kind") != "emscripten-fruity-v1":
        print("upload_plan: engine.kind is %r, not emscripten-fruity-v1: refusing to plan a default manifest for it" % man.get("engine", {}).get("kind"), file=sys.stderr); return 2
    keys = {it["key"] for it in items}
    mkey = [it for it in items if it["file"] == "manifest.json"]
    if len(mkey) != 1:
        print("upload_plan: expected exactly one manifest.json line", file=sys.stderr); return 2
    prefix = mkey[0]["key"].rsplit("/", 1)[0]
    # every url in the manifest must be a key of this plan
    def urls(o):
        if isinstance(o, dict):
            for k, v in o.items():
                if k == "url" and isinstance(v, str):
                    yield v
                else:
                    yield from urls(v)
        elif isinstance(o, list):
            for v in o:
                yield from urls(v)
    for u in urls(man):
        k = u[len("/assets/"):] if u.startswith("/assets/") else None
        if k is None or k not in keys:
            print("upload_plan: the manifest names %s which this plan does not upload" % u, file=sys.stderr); return 2
    rows = []
    for it in items:
        p = os.path.join(bundle, it["file"])
        size = os.path.getsize(p)
        if size > LIMIT:
            print("upload_plan: %s is %d bytes, over wrangler's 300 MiB limit" % (it["file"], size), file=sys.stderr); return 2
        want = sums.get(it["file"])
        got = sha256_file(p)
        if want != got:
            print("upload_plan: %s has SHA-256 %s, SHA256SUMS says %s" % (it["file"], got, want), file=sys.stderr); return 2
        rows.append(dict(it, path=p, size=size, sha256=got))
    mrow = [r for r in rows if r["file"] == "manifest.json"][0]
    rows = [r for r in rows if r["file"] != "manifest.json"] + [mrow, dict(mrow, key="manifest.json", note="default manifest (the bucket's entry point)")]
    cmds = []
    for r in rows:
        c = "aws s3 cp %s s3://%s/%s --content-type %s --cache-control %s" % (shlex.quote(r["path"]), bucket, r["key"], r["ct"], shlex.quote(r["cc"]))
        if r["ce"]:
            c += " --content-encoding %s" % r["ce"]
        cmds.append(c)
    if opt("--commands"):
        open(opt("--commands"), "w").write("\n".join(cmds) + "\n")
    if opt("--json"):
        json.dump(dict(bucket=bucket, prefix=prefix, rows=rows), open(opt("--json"), "w"), indent=1)
    lines = ["| key | bytes | SHA-256 (stored file) | Content-Encoding |", "|---|---:|---|---|"]
    for r in rows:
        lines.append("| `%s` | %d | `%s` | %s |" % (r["key"], r["size"], r["sha256"][:16], r["ce"] or "-"))
    if opt("--table"):
        open(opt("--table"), "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))
    print("\nupload_plan: %d objects, %d bytes into %s (bundle %s, engine wasm %s)" % (len(rows), sum(r["size"] for r in rows), bucket, prefix, man["engine"]["wasm"]["sha256"][:16]))
    if "--execute" in args:
        import subprocess
        mode = opt("--mode", "s3")
        # --phase objects: everything except the DEFAULT manifest (the bundle's files and its own manifest under the new prefix: nothing points to them yet);
        # --phase switch: only the default manifest (the entry point) - the last thing, after the Worker. No --phase: all, default manifest last.
        phase = opt("--phase", "all")
        if phase not in ("all", "objects", "switch"):
            print("upload_plan: --phase must be objects, switch or all", file=sys.stderr); return 2
        todo = [r for r in rows if phase == "all" or (phase == "switch") == (r["key"] == "manifest.json")]
        print("upload_plan: phase %s: %d of %d objects" % (phase, len(todo), len(rows)), flush=True)
        for r in todo:
            if mode == "s3":
                a = ["aws", "s3", "cp", r["path"], "s3://%s/%s" % (bucket, r["key"]), "--endpoint-url", opt("--endpoint"), "--only-show-errors",
                     "--content-type", r["ct"], "--cache-control", r["cc"]] + (["--content-encoding", r["ce"]] if r["ce"] else [])
            else:
                a = shlex.split(opt("--wrangler", "npx wrangler")) + ["r2", "object", "put", "%s/%s" % (bucket, r["key"]), "--remote", "--file", r["path"], "--content-type", r["ct"], "--cache-control", r["cc"]] + (["--content-encoding", r["ce"]] if r["ce"] else [])
            print("+ upload %s (%d bytes)" % (r["key"], r["size"]), flush=True)
            subprocess.run(a, check=True, env=dict(os.environ, AWS_DEFAULT_REGION="auto", AWS_EC2_METADATA_DISABLED="true"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
