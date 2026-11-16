#!/usr/bin/env python3
"""Writes the deploy configuration of the Worker from app/wrangler.jsonc plus values given as environment variables. Nothing in the output is a credential.

  ASSETS_BUCKET_NAME (required) R2 bucket bound as ASSETS_BUCKET (the delivery bucket)
  PERF_BUCKET_NAME   (required) R2 bucket bound as PERF_BUCKET: where the Worker keeps the measurement beacons (perf/ prefix, decision 0224). It must NOT be the delivery bucket (nothing the page posts may sit where /assets/* is served from)
  ACCESS_TEAM_DOMAIN (required with PUBLIC_HOSTNAME) e.g. your-team.cloudflareaccess.com
  ACCESS_AUD         (required with PUBLIC_HOSTNAME) the AUD tag of the Access application that protects PUBLIC_HOSTNAME
  PUBLIC_HOSTNAME    (optional) the Worker's custom domain. Without it the Worker gets NO route and no URL (workers_dev stays false).

It refuses to produce a configuration that opens a door: a route without the Access settings, workers_dev / preview_urls on,
a wildcard or non-hostname pattern. The Worker itself fails closed too (403 for every request while ACCESS_TEAM_DOMAIN or ACCESS_AUD is empty).
usage: wrangler_config.py APP_WRANGLER_JSONC OUT_JSON
"""
import json
import os
import re
import sys


def strip_jsonc(text):
    out, i, n, in_str = [], 0, len(text), False
    while i < n:
        c = text[i]
        if in_str:
            out.append(c)
            if c == "\\":
                out.append(text[i + 1]); i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True; out.append(c)
        elif text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
            continue
        elif text.startswith("/*", i):
            i = text.index("*/", i) + 2
            continue
        else:
            out.append(c)
        i += 1
    s = "".join(out)
    return re.sub(r",(\s*[}\]])", r"\1", s)


def fail(msg):
    print("wrangler_config: " + msg, file=sys.stderr)
    sys.exit(2)


def main(argv):
    if len(argv) != 3:
        print(__doc__); return 64
    cfg = json.loads(strip_jsonc(open(argv[1], encoding="utf-8").read()))
    env = os.environ
    bucket = env.get("ASSETS_BUCKET_NAME", "").strip()
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{1,61}[a-z0-9]", bucket):
        fail("ASSETS_BUCKET_NAME is missing or not an R2 bucket name")
    bound = [b for b in cfg.get("r2_buckets", []) if b.get("binding") == "ASSETS_BUCKET"]
    if len(bound) != 1:
        fail("app/wrangler.jsonc must bind exactly one R2 bucket as ASSETS_BUCKET")
    bound[0]["bucket_name"] = bucket
    perf = env.get("PERF_BUCKET_NAME", "").strip()
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{1,61}[a-z0-9]", perf):
        fail("PERF_BUCKET_NAME is missing or not an R2 bucket name")
    if perf == bucket:
        fail("PERF_BUCKET_NAME must not be the delivery bucket ASSETS_BUCKET_NAME: beacons would sit where /assets/* is served from")
    perf_bound = [b for b in cfg.get("r2_buckets", []) if b.get("binding") == "PERF_BUCKET"]
    if len(perf_bound) != 1:
        fail("app/wrangler.jsonc must bind exactly one R2 bucket as PERF_BUCKET")
    perf_bound[0]["bucket_name"] = perf
    if cfg.get("workers_dev") is not False or cfg.get("preview_urls") is not False:
        fail("app/wrangler.jsonc must keep workers_dev and preview_urls false (Access protects only the hostnames it is given)")
    if cfg.get("routes") or cfg.get("route"):
        fail("app/wrangler.jsonc must not carry a route: the host name comes from PUBLIC_HOSTNAME")
    v = cfg.setdefault("vars", {})
    team, aud, host = env.get("ACCESS_TEAM_DOMAIN", "").strip(), env.get("ACCESS_AUD", "").strip(), env.get("PUBLIC_HOSTNAME", "").strip()
    if team and not re.fullmatch(r"[a-z0-9][a-z0-9-]*\.cloudflareaccess\.com", team):
        fail("ACCESS_TEAM_DOMAIN must look like <team>.cloudflareaccess.com")
    if aud and not re.fullmatch(r"[0-9a-f]{64}", aud):
        fail("ACCESS_AUD must be the 64 hex characters of an Access AUD tag")
    v["ACCESS_TEAM_DOMAIN"], v["ACCESS_AUD"] = team, aud
    if host:
        if not (team and aud):
            fail("PUBLIC_HOSTNAME is set but ACCESS_TEAM_DOMAIN / ACCESS_AUD are not: refusing to attach a route that Access does not protect")
        if not re.fullmatch(r"(?=.{4,253}$)([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,}", host) or host.startswith("*"):
            fail("PUBLIC_HOSTNAME must be one plain host name (no wildcard, no path)")
        cfg["routes"] = [{"pattern": host, "custom_domain": True}]
    cfg.pop("$schema", None)
    with open(argv[2], "w", encoding="utf-8") as f:
        json.dump(cfg, f, indent=2); f.write("\n")
    print("wrangler_config: wrote %s (worker %s, bucket %s, perf bucket %s, route %s, access %s)" % (argv[2], cfg["name"], bucket, perf, host or "NONE", "set" if team and aud else "EMPTY -> the Worker answers 403"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
