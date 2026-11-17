// Reads a Chrome net-log (--log-net-log) and reports every connection attempt, DNS lookup and URL request by host (decision 0124: the browser measurements must connect to 127.0.0.1 only).
// usage: node check_netlog.mjs <netlog.json> [allowed-host-prefix=127.0.0.1] [tolerate-ipv6-probe]
// `tolerate-ipv6-probe`: accept (and say so on the output line) exactly one thing that was NOT removed by any flag tried (docs/decisions/0124): UDP connect() calls to [2001:4860:4860::8888]:443 with no UDP_BYTES_SENT on their sockets.
// Prints one summary line:
//   netlog: events=<n> url_hosts=<host:count,...> tcp_connects=<addr:count,...> udp_connects=<addr:count,...> non_loopback_tcp_connects=<n> non_loopback_udp_connects=<n> udp_bytes_sent_on_them=<n> non_loopback_url_requests=<n> (net errors: <code:count,...>)
// A TCP connect = TCP_CONNECT_ATTEMPT with an address; a UDP connect = UDP_CONNECT with an address (a UDP connect() sends nothing: it only selects a route; UDP_BYTES_SENT events on the same source are
// counted separately). Exit status 0 only when non_loopback_tcp_connects = 0 AND non_loopback_udp_connects = 0 (the requirement of decision 0124: nothing but 127.0.0.1). non_loopback_url_requests =
// the requests Chrome itself started for other hosts (component updater, account manager ...), informational, with the net errors they ended with.
import fs from 'node:fs';
const file = process.argv[2];
const allow = process.argv[3] || '127.0.0.1';
const tolerate = process.argv[4] === 'tolerate-ipv6-probe';
let txt = fs.readFileSync(file, 'utf8').trim();
let log;
try { log = JSON.parse(txt); } catch {
  // a log that Chrome did not finish (killed): cut after the last complete event and close the document; the summary then says so
  const i = txt.lastIndexOf('},\n{');
  log = JSON.parse(txt.slice(0, i + 1) + ']}');
  console.log('netlog: the file is NOT a finished log (Chrome did not close it); events after the last complete one are missing');
}
const types = Object.fromEntries(Object.entries(log.constants.logEventTypes).map(([k, v]) => [v, k]));
const tcp = {}, udp = {}, errs = {}, urlHosts = {};
const bump = (o, k) => { o[k] = (o[k] || 0) + 1; };
const urlOf = new Map(), lastErr = new Map(), udpSrc = new Set(), udpSent = new Map();
let n = 0;
for (const e of log.events) {
  n++;
  const t = types[e.type], p = e.params || {}, id = e.source.id;
  if (t === 'URL_REQUEST_START_JOB' && p.url) urlOf.set(id, p.url);
  if (p.net_error && urlOf.has(id)) lastErr.set(id, p.net_error);
  if (t === 'TCP_CONNECT_ATTEMPT' && p.address) bump(tcp, p.address);
  if (t === 'UDP_CONNECT' && p.address) { bump(udp, p.address); udpSrc.add(id); }
  if (t === 'UDP_BYTES_SENT') udpSent.set(id, (udpSent.get(id) || 0) + 1);
}
const isLoop = (a) => a.startsWith(allow) || a.startsWith('[::1]') || a.startsWith('::1');
for (const [id, u] of urlOf) {
  let h; try { h = new URL(u).host; } catch { h = u.slice(0, 40); }
  bump(urlHosts, h);
  if (!isLoop(h)) bump(errs, String(lastErr.get(id) ?? 'none'));
}
const cnt = (o) => Object.entries(o).filter(([a]) => !isLoop(a)).reduce((s, [, c]) => s + c, 0);
const nTcp = cnt(tcp), nUdp = cnt(udp), nUrl = cnt(urlHosts);
let sent = 0; for (const id of udpSrc) sent += udpSent.get(id) || 0;
const f = (o) => Object.entries(o).map(([k, v]) => `${k}:${v}`).join(',') || '-';
const probeOnly = Object.entries(udp).filter(([a]) => !isLoop(a)).every(([a]) => a === '[2001:4860:4860::8888]:443');
const tolerated = tolerate && nUdp > 0 && probeOnly && sent === 0;
console.log(`netlog: events=${n} url_hosts=${f(urlHosts)} tcp_connects=${f(tcp)} udp_connects=${f(udp)} non_loopback_tcp_connects=${nTcp} non_loopback_udp_connects=${nUdp} udp_bytes_sent_on_them=${sent} non_loopback_url_requests=${nUrl} (net errors: ${f(errs)})${tolerated ? ' [UDP connect() to 2001:4860:4860::8888:443, 0 bytes sent, TOLERATED by option, not removed]' : ''}`);
process.exit(nTcp === 0 && (nUdp === 0 || tolerated) ? 0 : 1);
