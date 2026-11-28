// page/beacon-no-personal-data: the beacon carries no address, no token and no cookie: its fields are the record's own, and its text has no IP address, no JWT and no host name (the page's kind of host, "hostKind", is a class word and is allowed).
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconNow } from '../../lib/page.mjs';

const FIELDS = ['v', 'seq', 'sid', 'reason', 'pageMs', 'visibility', 'browser', 'bundle', 'engine', 'error', 'timeline', 'startup', 'ui', 'input', 'audio', 'memory', 'client', 'display', 'hz', 'rafIntervalMsP95', 'guestScreen', 'warns', 'assets', 'desk', 'engineRun', 'uiWindows', 'files'];

export default defineScenario({
  id: 'page/beacon-no-personal-data',
  covers: ['page/beacon/no-personal-data'],
  async run({ guest, page }) {
    const b = await beaconNow(page);
    const parsed = JSON.parse(b.body);
    const extra = Object.keys(parsed).filter((k) => !FIELDS.includes(k));
    const text = b.body;
    const ipv4 = /\b(?:\d{1,3}\.){3}\d{1,3}\b/.test(text);
    const jwt = /eyJ[A-Za-z0-9_-]{10,}/.test(text);
    const local = /127\.0\.0\.1|:\/\/localhost|localhost:\d/.test(text);
    const cookie = /cookie/i.test(text) || Object.keys(b.headers).some((k) => k === 'cookie');
    const state = (await guest.state())?.state ?? null;
    const ok = extra.length === 0 && !ipv4 && !jwt && !local && !cookie;
    return { ok, engineState: state, notes: [`fields outside the record: ${extra.length}; IPv4 ${ipv4}; a token ${jwt}; a local host ${local}; a cookie ${cookie}`], results: { extra, ipv4, jwt, local, cookie } };
  },
});
