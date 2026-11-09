#!/usr/bin/env python3
"""ci/lib/r2_lifecycle.py CURRENT.json NEW.json DAYS PREFIX RULE_ID - the lifecycle configuration of a bucket with one expiry rule for PREFIX.
CURRENT is the output of `aws s3api get-bucket-lifecycle-configuration` (or {"Rules": []}). Exit 0: a rule for PREFIX that is enabled and expires after DAYS days is there (NEW.json is not written).
Exit 10: NEW.json holds CURRENT with that rule added (an existing rule for PREFIX, or with the id RULE_ID, is replaced; every other rule is kept as it is)."""
import json, sys

def prefix_of(rule):
    f = rule.get('Filter')
    if isinstance(f, dict):
        return f.get('Prefix')
    return rule.get('Prefix')

def has_rule(rules, days, prefix):
    for r in rules:
        exp = r.get('Expiration') or {}
        if r.get('Status') == 'Enabled' and prefix_of(r) == prefix and exp.get('Days') == days and set(r.keys()) <= {'ID', 'Status', 'Filter', 'Prefix', 'Expiration'}:
            return True
    return False

def merged(rules, days, prefix, rule_id):
    keep = [r for r in rules if prefix_of(r) != prefix and r.get('ID') != rule_id]
    keep.append({'ID': rule_id, 'Status': 'Enabled', 'Filter': {'Prefix': prefix}, 'Expiration': {'Days': days}})
    return keep

def main(a):
    cur = json.load(open(a[0]))
    days, prefix, rule_id = int(a[2]), a[3], a[4]
    rules = cur.get('Rules', [])
    if has_rule(rules, days, prefix):
        return 0
    json.dump({'Rules': merged(rules, days, prefix, rule_id)}, open(a[1], 'w'), indent=1)
    return 10

if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
