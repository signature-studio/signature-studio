// Net-log summary. The counting rules are those of lift/harness/web/threads/check_netlog.mjs (decision 0124), copied here as a function:
// a TCP connect = TCP_CONNECT_ATTEMPT with an address; a UDP connect = UDP_CONNECT with an address (it sends nothing; UDP_BYTES_SENT on the same source is counted separately).
// Only 127.0.0.1 / ::1 may be connected to.
import fs from 'node:fs';

export function summarizeNetlog(file, allow = '127.0.0.1') {
  const txt = fs.readFileSync(file, 'utf8').trim();
  let log, complete = true;
  try { log = JSON.parse(txt); } catch { const i = txt.lastIndexOf('},\n{'); log = JSON.parse(txt.slice(0, i + 1) + ']}'); complete = false; }
  const types = Object.fromEntries(Object.entries(log.constants.logEventTypes).map(([k, v]) => [v, k]));
  const tcp = {}, udp = {}, urlHosts = {}, errs = {};
  const bump = (o, k) => { o[k] = (o[k] || 0) + 1; };
  const urlOf = new Map(), lastErr = new Map(), udpSrc = new Set(), udpSent = new Map();
  let events = 0;
  for (const e of log.events) {
    events++;
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
  let sent = 0; for (const id of udpSrc) sent += udpSent.get(id) || 0;
  const udpSentAll = [...udpSent.values()].reduce((s, c) => s + c, 0);
  const f = (o) => Object.entries(o).map(([k, v]) => `${k}:${v}`).join(',') || '-';
  return {
    complete, events, nonLoopbackTcp: cnt(tcp), nonLoopbackUdpConnects: cnt(udp), udpBytesSentEvents: udpSentAll, udpBytesSentOnNonLoopback: sent, nonLoopbackUrlRequests: cnt(urlHosts),
    line: `netlog: events=${events} url_hosts=${f(urlHosts)} tcp_connects=${f(tcp)} udp_connects=${f(udp)} non_loopback_tcp_connects=${cnt(tcp)} non_loopback_udp_connects=${cnt(udp)} udp_bytes_sent_events=${udpSentAll} non_loopback_url_requests=${cnt(urlHosts)} (net errors: ${f(errs)})${complete ? '' : ' [log not closed by Chrome]'}`,
  };
}
