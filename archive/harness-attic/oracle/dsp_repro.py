#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: re-run single cases with a long alarm and compare (run_diff2.compare).

Used for cases that run_diff2.py classified mismatch_core with only `lift_timeout`: each case is run alone on the native
side and on the lift side with ORACLE_TIMEOUT raised, the wall-clock time of each run is measured, and the two results are
compared with the same function as the oracle (GPR, XMM, memory diffs, EFLAGS, MXCSR, x87).
usage: dsp_repro.py BINDIR PE CASEFILE.txt IDS.txt OUT.json [--native-alarm 60] [--lift-alarm 1800] [--jobs 8]
       [--redo-lift-timeouts]   (resumable: out files of an interrupted run are reused; OUT.json is written at the end)
CASEFILE.txt: the `CASE ...` lines (e.g. the work/cases-nat-NN.txt files concatenated); IDS.txt: one case id per line.
OUT.json must not exist. Per-case files are written next to OUT.json in OUT.json.d/.
"""
import argparse
import json
import os
import subprocess
import sys
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run_diff2 as rd  # noqa: E402


def read_out(of):
    o = None
    if os.path.exists(of) and os.path.getsize(of) > 0:
        with open(of) as fh:
            for ln in fh:
                if ln.strip():
                    o = json.loads(ln)
    return o


def run_one(bindir, pe, outdir, cid, line, nalarm, lalarm, redo_lift_timeout=False):
    """Resumable: a non-empty out file of an earlier (interrupted) run is reused (its seconds are estimated from the
    file mtimes: the lift run starts when the native run ends, the native run starts when the case file is written).
    With redo_lift_timeout a lift result of status timeout is run again with the (longer) alarm into
    out-lift-<id>.a<alarm>.jsonl; the earlier file is kept."""
    res = {"id": cid}
    cf = os.path.join(outdir, "case-%s.txt" % cid)
    if not os.path.exists(cf) or os.path.getsize(cf) == 0:   # an empty case file is what a full disk leaves behind
        with open(cf, "w") as fh:
            fh.write(line + "\n")
    outs = {}
    prev_end = os.path.getmtime(cf)
    for side, drv, alarm in (("native", "native_drv2", nalarm), ("lift", "lift_drv2", lalarm)):
        of = os.path.join(outdir, "out-%s-%s.jsonl" % (side, cid))
        o = read_out(of)
        if o is not None and side == "lift" and redo_lift_timeout and o["status"] == "timeout":
            res["lift_first_alarm_status"] = "timeout"
            of = os.path.join(outdir, "out-lift-%s.a%d.jsonl" % (cid, alarm))
            o = read_out(of)
        if o is not None:
            res[side + "_seconds"] = round(os.path.getmtime(of) - prev_end, 2)
            res[side + "_seconds_estimated_from_mtime"] = True
        else:
            env = dict(os.environ)
            env["ORACLE_TIMEOUT"] = str(alarm)
            t0 = time.monotonic()
            subprocess.run([os.path.join(bindir, drv), cf, of, pe], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
            res[side + "_seconds"] = round(time.monotonic() - t0, 2)
            o = read_out(of)
        prev_end = os.path.getmtime(of) if os.path.exists(of) else prev_end
        outs[side] = o
        res[side + "_status"] = o["status"] if o else "missing"
    n, l = outs["native"], outs["lift"]
    if n and l and n["status"] == "ok" and l["status"] == "ok":
        core, aux, memd = rd.compare(n, l, Counter())
        res["core"], res["aux"] = core, aux
        res["verdict"] = "bit_exact" if not core and not aux else "mismatch"
        if memd:
            res["mem"] = memd
    elif n and l and n["status"] == "ok":
        res["verdict"] = "lift_" + l["status"]
        res["lift_msg"] = l.get("msg", "")
    else:
        res["verdict"] = "native_" + (n["status"] if n else "missing")
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("bindir")
    ap.add_argument("pe")
    ap.add_argument("casefile")
    ap.add_argument("ids")
    ap.add_argument("out")
    ap.add_argument("--native-alarm", type=int, default=60)
    ap.add_argument("--lift-alarm", type=int, default=1800)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--redo-lift-timeouts", action="store_true",
                    help="run again, with --lift-alarm, the cases whose earlier lift result was status timeout")
    a = ap.parse_args()
    want = [x.strip() for x in open(a.ids) if x.strip()]
    lines = {}
    with open(a.casefile) as fh:
        for ln in fh:
            p = ln.split()
            if len(p) > 2 and p[0] == "CASE":
                lines[p[1]] = ln.rstrip("\n")
    outdir = a.out + ".d"
    os.makedirs(outdir, exist_ok=True)
    with ThreadPoolExecutor(a.jobs) as ex:
        rows = list(ex.map(lambda c: run_one(a.bindir, a.pe, outdir, c, lines[c], a.native_alarm, a.lift_alarm,
                                             a.redo_lift_timeouts), want))
    with open(a.out, "x") as fh:
        json.dump(rows, fh, indent=1)
    print(json.dumps(Counter(r["verdict"] for r in rows)))


if __name__ == "__main__":
    main()
