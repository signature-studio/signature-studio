"""Evaluates opencode.jsonc the way opencode does (E2 level: a re-implementation, not the real tool path).
Used by tests/harness/t02_permissions.sh. t01_config.sh compares this construction with the rules `opencode debug agents` really resolves.

Order of the resolved rules (observed with `opencode debug agents`): built-in defaults, then the top-level "permissions", then the agent's "permissions".
The last matching rule wins; no match = ask. "policies" are evaluated after all of that: any matching `tool.use` deny turns the result into deny.
Wildcard: `*` = any characters including `/` and spaces, `?` = one character, and a pattern ending in " *" also matches the bare command (opencode: util/wildcard.ts).
For read/edit/external_directory a leading ~ or $HOME in a rule is expanded when the configuration loads; shell resources are raw command text.
"""
import json
import os
import re
import sys

HOME = os.path.expanduser("~")
PATH_ACTIONS = ("read", "edit", "external_directory")


def strip_jsonc(t):
    out, i, n, ins = [], 0, len(t), False
    while i < n:
        c = t[i]
        if ins:
            out.append(c)
            if c == "\\":
                out.append(t[i + 1]); i += 2; continue
            if c == '"': ins = False
            i += 1; continue
        if c == '"': ins = True; out.append(c); i += 1; continue
        if t.startswith("//", i):
            while i < n and t[i] != "\n": i += 1
            continue
        if t.startswith("/*", i):
            i = t.index("*/", i + 2) + 2; continue
        out.append(c); i += 1
    return re.sub(r",(\s*[}\]])", r"\1", "".join(out))


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.loads(strip_jsonc(f.read()))


def match(value, pattern):
    value = value.replace("\\", "/")
    p = pattern.replace("\\", "/")
    out = ""
    for ch in p:
        if ch in ".+^${}()|[]\\": out += "\\" + ch
        elif ch == "*": out += ".*"
        elif ch == "?": out += "."
        else: out += ch
    if out.endswith(" .*"): out = out[:-3] + "( .*)?"
    return re.fullmatch(out, value, re.S) is not None


def expand(rule):
    r = dict(rule)
    if r["action"] in PATH_ACTIONS:
        res = r["resource"]
        if res == "~" or res.startswith("~/"): res = HOME + res[1:]
        elif res == "$HOME" or res.startswith("$HOME/"): res = HOME + res[5:]
        r["resource"] = res
    return r


def agent_rules(cfg, agent):
    base = [{"action": "*", "resource": "*", "effect": "allow"}]
    rules = base + cfg.get("permissions", []) + cfg["agents"][agent].get("permissions", [])
    return [expand(r) for r in rules]


def config_rules(cfg, agent):
    """The part of the resolved list that comes from opencode.jsonc (for the comparison with `opencode debug agents`)."""
    return [expand(r) for r in cfg.get("permissions", []) + cfg["agents"][agent].get("permissions", [])]


def last_match(rules, action, resource):
    for r in reversed(rules):
        if match(action, r["action"]) and match(resource, r["resource"]):
            return r
    return None


def policy_deny(cfg, action, resource):
    hit = None
    for p in cfg.get("policies", []):
        if p["action"] == "tool.use" and match(action + ":" + resource, p["resource"]):
            hit = p
    return hit is not None and hit["effect"] == "deny"


def decide(cfg, agent, action, resource):
    """-> (effect, why). effect is allow / deny / ask."""
    r = last_match(agent_rules(cfg, agent), action, resource)
    effect = r["effect"] if r else "ask"
    if effect == "deny":
        return "deny", "permissions: " + r["action"] + " " + r["resource"]
    if policy_deny(cfg, action, resource):
        return "deny", "policy"
    return effect, ("permissions: " + r["action"] + " " + r["resource"]) if r else "no rule"


def decide_shell(cfg, agent, resources):
    """A command is split into parts; one deny denies the whole command."""
    if not resources:
        return "allow", "no command part"
    results = [decide(cfg, agent, "shell", res) for res in resources]
    for (effect, why), res in zip(results, resources):
        if effect == "deny": return "deny", "[" + res + "] " + why
    for (effect, why), res in zip(results, resources):
        if effect == "ask": return "ask", "[" + res + "] " + why
    return "allow", results[0][1]


if __name__ == "__main__":
    cfg = load(sys.argv[1])
    print(json.dumps(decide(cfg, sys.argv[2], sys.argv[3], sys.argv[4])))
