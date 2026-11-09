#!/usr/bin/env python3
"""Mutation checks of the fast path (decision 0143): each mutation edits one expression of a copy of
runtime/fle_mxcsr_flags.h, the differential test is built against the copy and run; the mutation is "caught" when the test
exits nonzero (a mismatch with the reference, or a guard violation). usage: mutate.py WORKDIR [NAME...]"""
import os, subprocess, sys, shutil, concurrent.futures as cf
HERE = os.path.dirname(os.path.abspath(__file__))
LIFT = os.path.abspath(os.path.join(HERE, '..', '..'))
RT = os.path.join(LIFT, 'runtime')
M = [
 ('M34-np-ignores-pm', '#define FLE_MXF_NP(mx) (((mx) & (FLE_MX_PE | 0x1000u)) != (FLE_MX_PE | 0x1000u))', '#define FLE_MXF_NP(mx) (((mx) & (FLE_MX_PE)) != (FLE_MX_PE))'),
 ('M35-np-always-skip', '#define FLE_MXF_NP(mx) (((mx) & (FLE_MX_PE | 0x1000u)) != (FLE_MX_PE | 0x1000u))', '#define FLE_MXF_NP(mx) 0'),
 ('M36-np-wrong-bit', '#define FLE_MXF_NP(mx) (((mx) & (FLE_MX_PE | 0x1000u)) != (FLE_MX_PE | 0x1000u))', '#define FLE_MXF_NP(mx) (((mx) & (FLE_MX_PE | 0x0800u)) != (FLE_MX_PE | 0x0800u))'),

 ('M29-twosum-drops-b-term', 'return (a - (s - v)) + (b - v);', 'return (a - (s - v));'),
 ('M30-twosum-drops-a-term', 'return (a - (s - v)) + (b - v);', 'return (b - v);'),
 ('M31-sqrt-pe-eq', 'FLE_MXF_PE(mq * mq != (((bb & FLE_MXF_M52) | FLE_MXF_H52) << s));', 'FLE_MXF_PE(mq * mq == (((bb & FLE_MXF_M52) | FLE_MXF_H52) << s));'),
 ('M09-mul-c1-uses-mask52', '} else if (c == 1) {\n            FLE_MXF_PE((lo & ((UINT64_C(1) << 53) - 1)) != 0);', '} else if (c == 1) {\n            FLE_MXF_PE((lo & ((UINT64_C(1) << 52) - 1)) != 0);'),
 ('M32-mul-c-swapped', 'if (c == 0) {\n            FLE_MXF_PE((lo & ((UINT64_C(1) << 52) - 1)) != 0);', 'if (c == 1) {\n            FLE_MXF_PE((lo & ((UINT64_C(1) << 52) - 1)) != 0);'),
 ('M33-div-s-relation', 's = 1075u + ((uint32_t)(ab >> 52) & 0x7ffu) - ((uint32_t)(bb >> 52) & 0x7ffu) - ((uint32_t)(qb >> 52) & 0x7ffu);', 's = 1075u + ((uint32_t)(ab >> 52) & 0x7ffu) - ((uint32_t)(bb >> 52) & 0x7ffu) - ((uint32_t)(qb >> 52) & 0x7ffu) + 1u;'),

 ('M01-twosum-err-sign', 'return (a - (s - v)) + (b - v);', 'return (a - (s - v)) - (b - v);'),
 ('M02-add-pe-eq', 'FLE_MXF_PE(fle_mxf_twosum_err(a, b, s) != 0.0);', 'FLE_MXF_PE(fle_mxf_twosum_err(a, b, s) == 0.0);'),
 ('M03-res64-boundary', '< (FLE_MXF_INF64 - (FLE_MXF_MINN64 + 1));', '<= (FLE_MXF_INF64 - (FLE_MXF_MINN64 + 1));'),
 ('M04-rc-guard-bit13-removed', '#define FLE_MX_FAST_CTL 0x6000u', '#define FLE_MX_FAST_CTL 0x2000u'),
 ('M05-rc-guard-bit14-removed', '#define FLE_MX_FAST_CTL 0x6000u', '#define FLE_MX_FAST_CTL 0x4000u'),
 ('M06-guard-removed-entirely', '#define FLE_MX_FAST_CTL 0x6000u', '#define FLE_MX_FAST_CTL 0x0000u'),
 ('M07-div-pe-eq', '''    FLE_MXF_PE((((qb & FLE_MXF_M52) | FLE_MXF_H52) * ((bb & FLE_MXF_M52) | FLE_MXF_H52)) !=''', '''    FLE_MXF_PE((((qb & FLE_MXF_M52) | FLE_MXF_H52) * ((bb & FLE_MXF_M52) | FLE_MXF_H52)) =='''),
 ('M08-mul-mask-52-51', 'FLE_MXF_PE((lo & ((UINT64_C(1) << 52) - 1)) != 0);', 'FLE_MXF_PE((lo & ((UINT64_C(1) << 51) - 1)) != 0);'),
 ('M10-sqrt-exponent-relation', 's = 1075u + ((uint32_t)(bb >> 52) & 0x7ffu) - 2u * ((uint32_t)(qb >> 52) & 0x7ffu);', 's = 1075u + ((uint32_t)(bb >> 52) & 0x7ffu) - 1u * ((uint32_t)(qb >> 52) & 0x7ffu);'),
 ('M11-f32-add-pe-no-rounding-term', 'pe = np && (fle_mxf_twosum_err(x, y, d) != 0.0 || (double)f != d);', 'pe = np && (fle_mxf_twosum_err(x, y, d) != 0.0);'),
 ('M12-f32-div-pe-eq', 'pe = np && (double)f * y != x;', 'pe = np && (double)f * y == x;'),
 ('M13-f2i-trunc-range', '} else if (trunc) {\n        if (!(x > -2147483648.0 && x < 2147483648.0)) {', '} else if (trunc) {\n        if (!(x > -2147483648.0 && x < 2147483649.0)) {'),
 ('M14-comi-lt-le', 'lt = p < q;\n                eq = p == q;\n            } else {\n                float', 'lt = p <= q;\n                eq = p == q;\n            } else {\n                float'),
 ('M15-mul-zero-sign-lost', '''    if ((ab << 1) == 0 || (bb << 1) == 0) {
        *r = pb;''', '''    if ((ab << 1) == 0 || (bb << 1) == 0) {
        *r = pb & ~UINT64_C(0x8000000000000000);'''),
 ('M16-cvtsi2ss32-pe-eq', 'FLE_MXF_PE(t != (double)s);', 'FLE_MXF_PE(t == (double)s);'),
 ('M17-min-lt-le', '*r = (is_max ? a > b : a < b) ? ab : bb;\n    return 1;\n}\n\nstatic inline int fle_mxf_minmax32', '*r = (is_max ? a >= b : a <= b) ? ab : bb;\n    return 1;\n}\n\nstatic inline int fle_mxf_minmax32'),
 ('M18-round-sign', 't = x < 0 ? -t : t;', 't = t;'),
 ('M19-guard-in-lane-removed', '''fle_mx_lane(int op, int f64, uint64_t a, uint64_t b, uint32_t mx, uint32_t *fl) {
#ifdef FLE_MX_FAST
    if (!(mx & FLE_MX_FAST_CTL)) {''', '''fle_mx_lane(int op, int f64, uint64_t a, uint64_t b, uint32_t mx, uint32_t *fl) {
#ifdef FLE_MX_FAST
    if (1) {'''),
 ('M20-sub-negation-removed', '''    if (sub) {
        b = -b;
    }''', '''    if (0) {
        b = -b;
    }'''),
 ('M21-div-zero-divisor-allowed', 'if (!(fle_mxf_zn64(ab, 1u, 0x7feu) && fle_mxf_n64(bb, 1u, 0x7feu))) {\n        return 0;\n    }\n    q =', 'if (!(fle_mxf_zn64(ab, 1u, 0x7feu) && fle_mxf_zn64(bb, 1u, 0x7feu))) {\n        return 0;\n    }\n    q ='),
 ('M22-denormal-operand-allowed', 'static inline int fle_mxf_zn64(uint64_t x, uint32_t lo, uint32_t hi) {\n    return fle_mxf_n64(x, lo, hi) || (x << 1) == 0;', 'static inline int fle_mxf_zn64(uint64_t x, uint32_t lo, uint32_t hi) {\n    return fle_mxf_n64(x, lo, hi) || (x << 1) == 0 || (x >> 52 & 0x7ff) == 0;'),
 ('M23-f32-mul-pe-eq', 'pe = np && (double)f != d;\n    } else if (op == FLE_MXO_DIV) {', 'pe = np && (double)f == d;\n    } else if (op == FLE_MXO_DIV) {'),
 ('M24-i2f-64-pe-boundary', 'FLE_MXF_PE(t >= 9223372036854775808.0 || (int64_t)t != s);\n        }\n    } else {', 'FLE_MXF_PE((int64_t)t != s);\n        }\n    } else {'),
 ('M25-cvtsd2ss-pe-eq', 'FLE_MXF_PE((double)f != d);\n        }\n        *r = fb;', 'FLE_MXF_PE((double)f == d);\n        }\n        *r = fb;'),
 ('M26-res32-min-normal-allowed', 'return ((x & 0x7fffffffu) - (FLE_MXF_MINN32 + 1)) < (FLE_MXF_INF32 - (FLE_MXF_MINN32 + 1));', 'return ((x & 0x7fffffffu) - (FLE_MXF_MINN32 + 1 - 1)) < (FLE_MXF_INF32 - (FLE_MXF_MINN32 + 1 - 1));'),
 ('M27-add-window-7fd-7fe', 'if (!(fle_mxf_zn64(ab, 1u, 0x7fdu) && fle_mxf_zn64(bb, 1u, 0x7fdu))) {', 'if (!(fle_mxf_zn64(ab, 1u, 0x7feu) && fle_mxf_zn64(bb, 1u, 0x7feu))) {'),
 ('M28-res64-min-normal-allowed', 'return ((x & UINT64_C(0x7fffffffffffffff)) - (FLE_MXF_MINN64 + 1)) < (FLE_MXF_INF64 - (FLE_MXF_MINN64 + 1));', 'return ((x & UINT64_C(0x7fffffffffffffff)) - (FLE_MXF_MINN64)) < (FLE_MXF_INF64 - (FLE_MXF_MINN64));'),
]
def run(m, work, cases):
    name, old, new = m
    d = os.path.join(work, name); rt = os.path.join(d, 'runtime')
    shutil.rmtree(d, ignore_errors=True); shutil.copytree(RT, rt)
    p = os.path.join(rt, 'fle_mxcsr_flags.h'); s = open(p).read()
    if s.count(old) != 1:
        return name, 'NOT APPLIED (count %d)' % s.count(old), ''
    open(p, 'w').write(s.replace(old, new))
    env = dict(os.environ, RT=rt, OPT=os.environ.get('OPT', 'O2'))
    b = subprocess.run([os.path.join(HERE, 'build_diff.sh'), d], env=env, capture_output=True, text=True)
    if b.returncode:
        return name, 'BUILD FAILED', b.stderr[-300:]
    r = subprocess.run([os.path.join(d, 'mxfp_diff'), 'rand', '*', str(cases), '11', '1'], capture_output=True, text=True)
    if not r.returncode:
        # a mutation that only matters with the precision exception unmasked: the abort has to be the same
        u = subprocess.run([os.path.join(d, 'mxfp_diff'), 'unmasked', str(max(cases // 3, 1000)), '12'], capture_output=True, text=True)
        if u.returncode:
            r = u
    lines = [l for l in r.stdout.splitlines() if l.startswith('MISMATCH') or l.startswith('TOTAL')]
    ops = sorted(set(l.split()[2] for l in r.stdout.splitlines() if l.startswith('MISMATCH')))
    viol = [l.split()[2] for l in r.stdout.splitlines() if l.startswith('OP rand') and ' guard_viol 0' not in l]
    return name, ('CAUGHT' if r.returncode else 'SURVIVED'), 'rc=%d first-ops=%s guard-viol-ops=%s %s' % (r.returncode, ','.join(ops[:6]), ','.join(viol[:6]), lines[-1] if lines else '')
if __name__ == '__main__':
    work = sys.argv[1]; names = sys.argv[2:]
    cases = int(os.environ.get('CASES', '300000'))
    os.makedirs(work, exist_ok=True)
    sel = [m for m in M if not names or m[0] in names or m[0].split('-')[0] in names]
    with cf.ThreadPoolExecutor(int(os.environ.get('PAR', '6'))) as ex:
        for name, res, info in ex.map(lambda m: run(m, work, cases), sel):
            print('%-36s %-9s %s' % (name, res, info), flush=True)
