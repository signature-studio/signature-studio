#!/usr/bin/env python3
"""A5 oracle for dsp_ippv2_x64.dll: a pool (pure.json format) of every pure function that contains one of the given
decision-0075 mnemonics (xmm form, dsp_pool.py classification). run_diff2.py --count N with N >= the pool size runs all of them.
usage: dsp_pool_mn.py PURE.json CLASSES.json OUT.json MNEMONIC[,MNEMONIC...]     (OUT.json must not exist)
"""
import json
import sys


def main():
    pure, classes, out, mns = sys.argv[1:5]
    want = set(mns.split(","))
    p = json.load(open(pure))
    cl = json.load(open(classes))
    fs = [f for f in p["functions"] if want & set(cl[f["va"]]["d0075"])]
    res = dict(p)
    res["functions"] = fs
    res["counts"] = dict(p["counts"], pure=len(fs), pool="functions with " + mns)
    with open(out, "x") as fh:
        json.dump(res, fh, indent=1)
    print("functions", len(fs), {m: sum(1 for f in fs if m in cl[f["va"]]["d0075"]) for m in sorted(want)})


if __name__ == "__main__":
    main()
