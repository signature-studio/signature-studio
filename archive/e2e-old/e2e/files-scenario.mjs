// The file manager's browser scenario (decision 0216, opfs1), written against a small driver so that the same steps run in Chrome (CDP) and in Firefox (WebDriver):
//   page.ev(expression)               evaluate a JS expression in the page (a promise is awaited) and return its JSON value
//   page.setFiles(selector, paths)    give an <input type=file> files, the way the browser's chooser would (CDP DOM.setFileInputFiles / WebDriver element send keys)
//   page.downloadDir                  where the browser puts downloads
// The OPFS is the browser's real one (no mock). The ground truth is read with the browser's own API in the page (navigator.storage.getDirectory()), not through the code under test.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { sha } from './files-testdata.mjs';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const J = JSON.stringify;

const HELPERS = `(() => {
  const q = (s) => document.querySelector(s);
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const T = {
    click(s) { const e = q(s); if (!e) throw new Error('no element ' + s); e.click(); return true; },
    exists: (s) => !!q(s),
    text: (s) => (q(s) ? q(s).textContent : null),
    rows: () => [...document.querySelectorAll('#fx-list .fx-row[data-name]')].map((r) => r.dataset.name + (r.dataset.kind === 'directory' ? '/' : '')),
    selectedNames: () => [...document.querySelectorAll('#fx-list .fx-row.sel')].map((r) => r.dataset.name),
    row(name) { return [...document.querySelectorAll('#fx-list .fx-row[data-name]')].find((r) => r.dataset.name === name); },
    selectRow(name, mods = {}) { const r = T.row(name); if (!r) throw new Error('no row ' + name); r.dispatchEvent(new MouseEvent('click', { bubbles: true, ...mods })); return true; },
    dbl(name) { const r = T.row(name); if (!r) throw new Error('no row ' + name); r.dispatchEvent(new MouseEvent('dblclick', { bubbles: true })); return true; },
    key(key, mods = {}) { q('#fx-list').dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...mods })); return true; },
    setText(v) { const i = q('#fx-modal-text'); i.value = v; i.dispatchEvent(new Event('input', { bubbles: true })); return true; },
    async idle(ms = 60000) {
      const t0 = performance.now();
      for (;;) {
        const ex = window.__files && window.__files.explorer;
        if (ex && !ex.busy && q('#fx-modal').hidden) { await sleep(30); if (!ex.busy && q('#fx-modal').hidden) return true; }
        if (performance.now() - t0 > ms) return false;
        await sleep(30);
      }
    },
    async waitModal(ms = 10000) { const t0 = performance.now(); while (q('#fx-modal').hidden) { if (performance.now() - t0 > ms) return false; await sleep(20); } return true; },
    async opfs(p = '/') {
      let d = await navigator.storage.getDirectory();
      for (const s of p.split('/').filter(Boolean)) d = await d.getDirectoryHandle(s);
      const out = [];
      const walk = async (dir, prefix) => { for await (const [name, h] of dir.entries()) { if (h.kind === 'file') { const f = await h.getFile(); out.push({ path: prefix + name, kind: 'file', size: f.size }); } else { out.push({ path: prefix + name + '/', kind: 'directory', size: 0 }); await walk(h, prefix + name + '/'); } } };
      await walk(d, '');
      return out.sort((a, b) => (a.path < b.path ? -1 : 1));
    },
    async sha(p) {
      let d = await navigator.storage.getDirectory();
      const parts = p.split('/').filter(Boolean);
      for (const s of parts.slice(0, -1)) d = await d.getDirectoryHandle(s);
      const f = await (await d.getFileHandle(parts[parts.length - 1])).getFile();
      const h = await crypto.subtle.digest('SHA-256', await f.arrayBuffer());
      return [...new Uint8Array(h)].map((b) => b.toString(16).padStart(2, '0')).join('');
    },
    async wipe() { const d = await navigator.storage.getDirectory(); for await (const [n] of d.entries()) await d.removeEntry(n, { recursive: true }); return true; },
  };
  window.__t = T;
  return true;
})()`;

export async function scenario({ page, check, data, log, browserName, target = 'preview' }) {
  const ev = page.ev;
  const T = (call) => ev(`window.__t.${call}`);
  const idle = async () => { const ok = await ev('window.__t.idle()'); if (!ok) throw new Error('file manager did not become idle'); };
  const rows = () => T('rows()');
  const opfs = (p = '/') => ev(`window.__t.opfs(${J(p)})`);
  const shaOf = (p) => ev(`window.__t.sha(${J(p)})`);
  const clickBtn = (id) => T(`click(${J('#fx-' + id)})`);
  const modalOk = async () => { await ev('window.__t.waitModal()'); await T(`click('#fx-modal-ok')`); };
  const nav = async (p) => { await ev(`window.__files.explorer.navigate(${J(p)})`); };
  const status = () => T(`text('#fx-status')`);
  const has = (list, name) => list.includes(name);
  const fileOf = (rel) => data.files[rel];

  await ev(HELPERS);
  await ev('window.__t.wipe()');
  await ev(`(() => { window.__csp = []; document.addEventListener('securitypolicyviolation', (e) => window.__csp.push(e.violatedDirective + ' ' + e.blockedURI)); return true; })()`);
  const csp = await ev(`fetch(location.href).then((r) => r.headers.get('content-security-policy') || '')`);
  check(target === 'preview' ? 'the page is served with the production Content-Security-Policy (style-src \'self\', no inline styles)' : 'CSP not in force on the dev server', target !== 'preview' || /style-src 'self'/.test(csp), csp.slice(0, 120));

  // ---------------------------------------------------------------- open the window
  check('the header has a Files button', await T(`exists('#files-btn')`));
  check('the page layout is untouched: the overlay is hidden before the click', await ev(`document.getElementById('files-overlay').hidden === true`));
  await ev(`(async () => { window.__t.click('#files-btn'); await window.__files.openWindow(); return true; })()`);
  await ev(`(async () => { for (let i = 0; i < 100 && !window.__files.explorer; i++) await new Promise((r) => setTimeout(r, 50)); return !!window.__files.explorer; })()`);
  await idle();
  check('the window opens on the OPFS root and lists it empty', J(await rows()) === '[]' && (await T(`text('#fx-crumbs')`)).includes('Browser files'));
  const store = await T(`text('#fx-storage')`);
  check('the storage line shows use and quota (navigator.storage.estimate) and the persistence state', /uses .* of about .* the browser allows/.test(store) && /Protected|Not protected/.test(store), store.slice(0, 160));
  check('the overlay covers the engine\'s windows (z-index above 1001)', await ev(`Number(getComputedStyle(document.getElementById('files-overlay')).zIndex) > 2000000000`));

  // ---------------------------------------------------------------- new folder, name rules
  await clickBtn('new-folder');
  await ev('window.__t.waitModal()');
  await T(`setText('a:b')`); await T(`click('#fx-modal-ok')`);
  check('a name with ":" is refused with the reason, the box stays', (await T(`text('#fx-modal-err')`)).includes('cannot contain') && !(await ev(`document.getElementById('fx-modal').hidden`)));
  await T(`setText('Projects')`); await T(`click('#fx-modal-ok')`); await idle();
  check('New folder creates a real OPFS directory', J((await opfs()).map((e) => e.path)) === '["Projects/"]' && has(await rows(), 'Projects/'));
  await clickBtn('new-folder'); await ev('window.__t.waitModal()'); await T(`setText('projects')`); await T(`click('#fx-modal-ok')`);
  check('a name that differs only by case is refused (Windows semantics)', (await T(`text('#fx-modal-err')`)).includes('already used'));
  await T(`click('#fx-modal-cancel')`); await idle();
  check('Cancel leaves everything as it was', J((await opfs()).map((e) => e.path)) === '["Projects/"]');

  // ---------------------------------------------------------------- upload files (the chooser)
  await T(`dbl('Projects')`); await idle();
  check('double-click on a folder enters it (breadcrumb)', (await T(`text('#fx-crumbs')`)).includes('Projects'));
  const loose = [fileOf('loose/readme.txt'), fileOf('loose/日本語 サンプル.wav')];
  await page.setFiles('#fx-input-files', loose.map((f) => f.path));
  await idle();
  let o = await opfs('/Projects');
  check('Upload files writes both files with their exact bytes (size and SHA-256 read back with the browser\'s own API)', o.length === 2 && (await shaOf('/Projects/readme.txt')) === loose[0].sha && (await shaOf('/Projects/日本語 サンプル.wav')) === loose[1].sha, J(o));
  check('the status line reports the upload', /Upload: 2 files written/.test(await status()), await status());

  // ---------------------------------------------------------------- upload a folder (the directory chooser)
  const dirFiles = ['proj/Song.flp', 'proj/Samples/kick.wav', 'proj/Samples/deep/鳴る [x].wav', 'proj/Samples/deep/b.bin'].map(fileOf);
  const dirMode = await page.setDirectory('#fx-input-dir', path.join(path.dirname(dirFiles[0].path)), dirFiles.map((f) => f.path));
  await idle();
  o = await opfs('/Projects/proj');
  const wantTree = ['Samples/', 'Samples/deep/', 'Samples/deep/b.bin', 'Samples/deep/鳴る [x].wav', 'Samples/kick.wav', 'Song.flp'];
  check(`Upload folder keeps the folder structure (${dirMode})`, J(o.map((e) => e.path)) === J(wantTree.sort()), J(o.map((e) => e.path)));
  let allSame = o.length > 0;
  for (const f of dirFiles) { const rel = f.path.split('/proj/')[1]; allSame = allSame && (await shaOf('/Projects/proj/' + rel)) === f.sha; }
  check('every uploaded file of the folder has the source\'s SHA-256', allSame);

  // ---------------------------------------------------------------- name clash: keep both / replace / skip
  await nav('/Projects');
  await page.setFiles('#fx-input-files', [loose[0].path]);
  await ev('window.__t.waitModal()');
  check('a name that is already used asks once (Replace / Keep both / Skip / Cancel)', await T(`exists('#fx-conflict-replace')`) && await T(`exists('#fx-conflict-keep')`) && await T(`exists('#fx-conflict-skip')`) && await T(`exists('#fx-conflict-cancel')`));
  await T(`click('#fx-conflict-keep')`); await idle();
  check('Keep both numbers the new file ("readme (2).txt") and leaves the old one', has(await rows(), 'readme (2).txt') && has(await rows(), 'readme.txt'));
  await page.setFiles('#fx-input-files', [loose[0].path]); await ev('window.__t.waitModal()'); await T(`click('#fx-conflict-skip')`); await idle();
  check('Skip writes nothing', (await opfs('/Projects')).filter((e) => e.path.startsWith('readme')).length === 2 && /1 skipped/.test(await status()), await status());
  await page.setFiles('#fx-input-files', [loose[0].path]); await ev('window.__t.waitModal()'); await T(`click('#fx-conflict-replace')`); await idle();
  check('Replace overwrites with the new bytes', (await shaOf('/Projects/readme.txt')) === loose[0].sha);
  await page.setFiles('#fx-input-files', [loose[0].path]); await ev('window.__t.waitModal()'); await T(`click('#fx-conflict-cancel')`); await idle();
  check('Cancel stops the operation', /cancelled/.test(await status()), await status());

  // ---------------------------------------------------------------- rename, copy / cut / paste, delete
  await nav('/Projects');
  await T(`selectRow('readme (2).txt')`); await T(`key('F2')`);
  await ev('window.__t.waitModal()'); await T(`setText('Readme.TXT')`); await T(`click('#fx-modal-ok')`);
  check('renaming to a name that differs from another only by case is refused', (await T(`text('#fx-modal-err')`)).includes('already used'));
  await T(`setText('notes.txt')`); await T(`click('#fx-modal-ok')`); await idle();
  check('F2 renames (move within the OPFS keeps the bytes)', has(await rows(), 'notes.txt') && !has(await rows(), 'readme (2).txt') && (await shaOf('/Projects/notes.txt')) === loose[0].sha);
  await T(`selectRow('notes.txt')`); await clickBtn('copy'); await nav('/Projects/proj'); await clickBtn('paste'); await idle();
  check('Copy + Paste copies into another folder', (await shaOf('/Projects/proj/notes.txt')) === loose[0].sha && (await shaOf('/Projects/notes.txt')) === loose[0].sha);
  await T(`selectRow('notes.txt')`); await clickBtn('copy'); await clickBtn('paste'); await idle();
  check('pasting a copy into the folder it came from numbers it without asking ("notes (2).txt")', has(await rows(), 'notes (2).txt') && !(await T(`exists('#fx-conflict-keep')`)) || has(await rows(), 'notes (2).txt'));
  await nav('/Projects'); await T(`selectRow('notes.txt')`); await clickBtn('copy'); await nav('/Projects/proj'); await clickBtn('paste'); await ev('window.__t.waitModal()');
  check('pasting a copy into another folder where the name exists asks (Replace / Keep both / Skip / Cancel)', await T(`exists('#fx-conflict-replace')`));
  await T(`click('#fx-conflict-keep')`); await idle();
  check('Keep both numbers it again ("notes (3).txt")', has(await rows(), 'notes (3).txt'), J(await rows()));
  await nav('/Projects'); await T(`selectRow('proj')`); await clickBtn('cut'); await T(`dbl('proj')`); await idle(); await clickBtn('paste'); await idle();
  check('a folder cannot be moved into itself', /cannot be put inside itself/.test(await status()), await status());
  await nav('/'); await T(`selectRow('Projects')`);
  await clickBtn('new-folder'); await ev('window.__t.waitModal()'); await T(`setText('Archive')`); await T(`click('#fx-modal-ok')`); await idle();
  await nav('/Projects'); await T(`selectRow('proj')`); await clickBtn('cut'); await nav('/Archive'); await clickBtn('paste'); await idle();
  o = await opfs('/Archive');
  check('Cut + Paste moves a whole folder (the source is gone, the tree and bytes are intact)', !(await opfs('/Projects')).some((e) => e.path.startsWith('proj')) && o.length === 10 && (await shaOf('/Archive/proj/Samples/kick.wav')) === fileOf('proj/Samples/kick.wav').sha, J(o.map((e) => e.path)));
  await nav('/Archive'); await T(`selectRow('proj')`); await T(`key('Delete')`);
  await ev('window.__t.waitModal()');
  check('Delete asks first and says how much goes', /files?,/.test(await ev(`document.querySelector('#fx-modal p').textContent`)) && /cannot be undone/.test(await ev(`document.querySelector('#fx-modal p').textContent`)));
  await T(`click('#fx-modal-cancel')`); await idle();
  check('Cancel on the Delete question deletes nothing', (await opfs("/Archive")).length === 10);
  await T(`key('Delete')`); await ev('window.__t.waitModal()'); await T(`click('#fx-modal-ok')`); await idle();
  check('Delete removes the folder with everything inside', (await opfs('/Archive')).length === 0);
  await T(`key('Backspace')`); await idle();
  check('Backspace goes up', (await T(`text('#fx-crumbs')`)) === 'Browser files');

  // ---------------------------------------------------------------- zips
  await nav('/');
  const zipOf = (n) => data.zips.find((z) => z.endsWith('/' + n));
  const extractBy = async (name, folder) => {
    await page.setFiles('#fx-input-zip', [zipOf(name)]);
    await ev('window.__t.waitModal()');
    const info = await ev(`document.querySelector('#fx-modal p').textContent`);
    if (folder) await T(`setText(${J(folder)})`);
    await T(`click('#fx-modal-ok')`); await idle();
    return info;
  };
  let info = await extractBy('utf8.zip', 'u8');
  o = await opfs('/u8');
  check('zip with the UTF-8 flag: names, stored/deflated data and an empty folder are extracted', J(o.map((e) => e.path)) === J(['P/', 'P/Empty/', 'P/Samples/', 'P/Samples/音.wav', 'P/曲.flp']) && (await shaOf('/u8/P/曲.flp')) === data.expected.d[0] && (await shaOf('/u8/P/Samples/音.wav')) === data.expected.d[1], J(o.map((e) => e.path)));
  check('the extract question reports the count, the size and where the names were read from', /utf8-flag/.test(info) && /2 files/.test(info), info);
  info = await extractBy('cp932.zip', 'cp');
  o = await opfs('/cp');
  check('zip without UTF-8 flag and without a Unicode field: Windows-31J names (クリスマス) are decoded', has(o.map((e) => e.path), 's - クリスマス.wav') && /cp932/.test(info), J(o.map((e) => e.path)) + ' | ' + info);
  info = await extractBy('unicode-extra.zip', 'ue');
  check('the Info-ZIP Unicode Path field is used', has((await opfs('/ue')).map((e) => e.path), 's - クリスマス.wav') && /unicode-extra/.test(info), info);
  info = await extractBy('evil.zip', 'ev');
  o = (await opfs('/ev')).map((e) => e.path);
  const outside = (await opfs('/')).map((e) => e.path).filter((p) => !p.includes('/') && !['Projects/', 'Archive/', 'u8/', 'cp/', 'ue/', 'ev/'].includes(p));
  check('zip-slip: "../x", "a/../../x" and "C:\\x" are refused; nothing is written outside the chosen folder', outside.length === 0 && !o.some((p) => /escape|up\.txt|drive/.test(p)), J(o) + ' outside=' + J(outside));
  check('absolute and backslash names are made relative, Windows-invalid characters are replaced', has(o, 'abs.txt') && has(o, 'bad/sep/win.txt') && has(o, 'we_ird_name_.txt') && has(o, 'ok/fine.txt'), J(o));
  check('the status line lists what was refused', /3 failed/.test(await status()), await status());
  info = await extractBy('badcrc.zip', 'crc');
  o = (await opfs('/crc')).map((e) => e.path);
  check('a damaged entry (CRC-32 mismatch) is removed and reported, the others are kept', J(o) === J(['after.bin', 'good.bin']) && /1 failed \(bad\.bin: CRC-32 mismatch/.test(await status()), J(o) + ' | ' + await status());
  info = await extractBy('unsupported.zip', 'unsup');
  o = (await opfs('/unsup')).map((e) => e.path);
  check('an encrypted entry and an unsupported method are reported, the rest is extracted', J(o) === J(['ok.bin']) && /2 failed/.test(await status()), J(o) + ' | ' + await status());

  // a zip dropped / uploaded as a plain file stays a file; Extract unpacks it
  await page.setFiles('#fx-input-files', [zipOf('utf8.zip')]); await idle();
  check('"Upload files" keeps a zip as a file', has(await rows(), 'utf8.zip'));
  await T(`selectRow('utf8.zip')`);
  check('the Extract button is enabled only for a selected .zip', await ev(`!document.getElementById('fx-extract').disabled`));
  await clickBtn('extract'); await ev('window.__t.waitModal()'); await T(`click('#fx-modal-ok')`); await idle();
  check('Extract on a zip in the OPFS unpacks it next to it ("utf8 (2)" because "utf8.zip"\'s stem is free but "u8" differs)', has(await rows(), 'utf8/'), J(await rows()));

  // cancel in the middle of an extraction of 24 x 3 MiB files
  await ev(`(async () => { window.__prog = []; const t0 = performance.now(); const iv = setInterval(() => { const l = document.querySelector('.fx-prog-label'); if (l) { window.__prog.push(l.textContent); if (/ [3-9]\\/24 /.test(l.textContent)) document.getElementById('fx-cancel') && document.getElementById('fx-cancel').click(); } }, 5); window.__ivc = iv; return true; })()`);
  info = await extractBy('many.zip', 'many');
  await ev('(() => { clearInterval(window.__ivc); return true; })()');
  const cnt = (await opfs('/many')).filter((e) => e.kind === 'file').length;
  check('Cancel in the middle of an extraction stops it: some files are there, not all, and the last partial file is not left half-written', cnt >= 3 && cnt < 24 && /cancelled/.test(await status()), `files=${cnt} status=${await status()}`);
  let partial = 0; for (const e of (await opfs('/many')).filter((x) => x.kind === 'file')) if (e.size !== 3 << 20) partial++;
  check('every file that is there is complete (3 MiB)', partial === 0, `partial=${partial}`);

  // ---------------------------------------------------------------- the reference project zip (real)
  const REF = path.join(os.homedir(), 'ダウンロード', 'serina1-serum2.zip');
  if (fs.existsSync(REF)) {
    const t0 = Date.now();
    await page.setFiles('#fx-input-zip', [REF]);
    await ev('window.__t.waitModal()');
    info = await ev(`document.querySelector('#fx-modal p').textContent`);
    await T(`setText('serina')`);
    const t1 = Date.now();
    await T(`click('#fx-modal-ok')`); await idle();
    const ms = Date.now() - t1;
    o = await opfs('/serina');
    const names = o.filter((e) => e.kind === 'file').map((e) => e.path);
    check('the reference project zip (49 files, 62,747,002 B unpacked) is extracted completely', names.length === 49 && o.reduce((n, e) => n + e.size, 0) === 62747002, `${names.length} files, ${o.reduce((n, e) => n + e.size, 0)} B, extract ${ms} ms (wall, load ${os.loadavg()[0].toFixed(1)}), question: ${info}`);
    // truth from Python's zipfile (independent reader) for the bytes; the names are checked against the FLP
    const py = execFileSync('python3', ['-c', `
import zipfile,hashlib,struct,json,sys
z=zipfile.ZipFile(sys.argv[1]); d=open(sys.argv[1],'rb').read()
e=d.rfind(b'PK\\x05\\x06'); n,cdsz,cdoff=struct.unpack('<HII',d[e+10:e+20]); p=cdoff; out={}
for i in z.infolist():
    fnl,exl,cml=struct.unpack('<HHH',d[p+28:p+34]); extra=d[p+46+fnl:p+46+fnl+exl]
    name=extra[9:].decode('utf8') if exl else i.filename
    out[name]=hashlib.sha256(z.read(i)).hexdigest(); p+=46+fnl+exl+cml
print(json.dumps(out,ensure_ascii=False))`, REF], { maxBuffer: 1 << 24 }).toString();
    const truth = JSON.parse(py);
    let same = 0, bad = [];
    for (const nm of Object.keys(truth)) { const got = names.includes(nm) ? await shaOf('/serina/' + nm) : null; if (got === truth[nm]) same++; else bad.push(nm); }
    check('every file has the SHA-256 that Python\'s zipfile reads from the zip (names: Unicode Path field)', same === 49 && bad.length === 0, `same=${same} bad=${J(bad)}`);
    const flpName = names.find((n) => n.endsWith('.flp'));
    const flp = await ev(`(async () => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('serina'); const f = await (await d.getFileHandle(${J(flpName)})).getFile(); const b = new Uint8Array(await f.arrayBuffer()); const out = {}; const names = ${J(names.filter((n) => !n.endsWith('.flp')))}; for (const n of names) { const u = new Uint8Array(n.length * 2); for (let i = 0; i < n.length; i++) { u[2*i] = n.charCodeAt(i) & 255; u[2*i+1] = n.charCodeAt(i) >> 8; } let found = false; outer: for (let i = 0; i + u.length <= b.length; i++) { if (b[i] !== u[0]) continue; for (let j = 1; j < u.length; j++) if (b[i+j] !== u[j]) continue outer; found = true; break; } out[n] = found; } return out; })()`);
    const missing = Object.keys(flp).filter((n) => !flp[n]);
    check('all 48 sample names are the names the FLP holds (UTF-16LE search in the extracted .flp: the Japanese names are exact, with U+3099 kept)', missing.length === 0 && Object.keys(flp).length === 48, `missing=${J(missing)}`);
  } else log('reference zip absent: skipped');

  // ---------------------------------------------------------------- download
  const dl = page.downloadDir;
  fs.rmSync(dl, { recursive: true, force: true }); fs.mkdirSync(dl, { recursive: true });
  const waitFile = async (name, ms = 20000) => { const t0 = Date.now(); for (;;) { const p = path.join(dl, name); if (fs.existsSync(p) && !fs.existsSync(p + '.crdownload') && !fs.readdirSync(dl).some((f) => f.endsWith('.part'))) { const a = fs.statSync(p).size; await sleep(150); if (fs.statSync(p).size === a) return p; } if (Date.now() - t0 > ms) return null; await sleep(100); } };
  await nav('/Projects');
  await T(`selectRow('日本語 サンプル.wav')`); await clickBtn('download');
  const p1 = await waitFile('日本語 サンプル.wav');
  check('Download saves a file with the exact bytes and the name', !!p1 && sha(fs.readFileSync(p1)) === loose[1].sha, p1 || 'no file');
  await nav('/u8'); await nav('/'); await T(`selectRow('u8')`);
  check('the Download button says "Download as zip" for a folder', (await T(`text('#fx-download')`)) === 'Download as zip');
  await clickBtn('download');
  const p2 = await waitFile('u8.zip');
  let zl = '';
  if (p2) zl = execFileSync('python3', ['-c', `
import zipfile,sys,json,hashlib
z=zipfile.ZipFile(sys.argv[1]); print(json.dumps({i.filename:[i.file_size,i.flag_bits&0x800,hashlib.sha256(z.read(i)).hexdigest() if not i.filename.endswith('/') else ''] for i in z.infolist()},ensure_ascii=False)); print(z.testzip())`, p2]).toString();
  const zinfo = zl ? JSON.parse(zl.split('\n')[0]) : {};
  check('Download as zip: the folder comes back as a valid zip (Python testzip), UTF-8 names with the flag, the same bytes', !!p2 && zl.split('\n')[1] === 'None' && zinfo['u8/P/曲.flp']?.[2] === data.expected.d[0] && zinfo['u8/P/Samples/音.wav']?.[2] === data.expected.d[1] && zinfo['u8/P/曲.flp'][1] === 0x800 && 'u8/P/Empty/' in zinfo, zl.slice(0, 300));

  // ---------------------------------------------------------------- drag and drop from the computer
  await nav('/');
  await clickBtn('new-folder'); await ev('window.__t.waitModal()'); await T(`setText('Dropped')`); await T(`click('#fx-modal-ok')`); await idle();
  await nav('/Dropped');
  // (a) a SIMULATED drop: a DragEvent with a DataTransfer that holds two File objects and a zip (the OS drop itself is not made by the browser here)
  const zbytes = fs.readFileSync(zipOf('utf8.zip')).toString('base64');
  await ev(`(async () => { const dt = new DataTransfer(); dt.items.add(new File(['one'], 'one.txt')); dt.items.add(new File(['two'], 'two 二.txt')); const zb = Uint8Array.from(atob(${J(zbytes)}), (c) => c.charCodeAt(0)); dt.items.add(new File([zb], 'dropped.zip', { type: 'application/zip' })); const w = document.querySelector('.fx-window'); w.dispatchEvent(new DragEvent('dragenter', { dataTransfer: dt, bubbles: true, cancelable: true })); const shown = !document.getElementById('fx-drop').hidden; window.__dropShown = shown; w.dispatchEvent(new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true })); return true; })()`);
  await ev('window.__t.waitModal()');
  check('a drop that includes a zip asks whether to unpack it', /dropped\.zip.* is a zip/.test(await ev(`document.querySelector('#fx-modal p').textContent`)) && (await ev('window.__dropShown')) === true);
  await T(`click('#fx-modal-ok')`); await ev('window.__t.waitModal()'); await T(`click('#fx-modal-ok')`); await idle();
  o = (await opfs('/Dropped')).map((e) => e.path);
  check('SIMULATED drop: the files are written, the zip is unpacked into a folder named after it', has(o, 'one.txt') && has(o, 'two 二.txt') && has(o, 'dropped/P/曲.flp') && !has(o, 'dropped.zip'), J(o));
  if (page.dropPaths) {
    await nav('/');
    await clickBtn('new-folder'); await ev('window.__t.waitModal()'); await T(`setText('Dropped2')`); await T(`click('#fx-modal-ok')`); await idle();
    await nav('/Dropped2');
    const logLen = await ev('window.__files.explorer.log.length');
    await page.dropPaths([path.dirname(path.dirname(dirFiles[0].path)) + '/proj', loose[0].path]);
    // the drop is read asynchronously (file-selector walks the dropped folder): wait for the operation's own status line, then for idle
    await ev(`(async () => { const t0 = performance.now(); while (window.__files.explorer.log.length <= ${logLen} && performance.now() - t0 < 20000) await new Promise((r) => setTimeout(r, 20)); return true; })()`);
    await idle();
    o = (await opfs('/Dropped2')).map((e) => e.path);
    check('REAL OS-style drop (CDP Input.dispatchDragEvent with file paths, one of them a FOLDER): the folder arrives with its structure, with the exact bytes', has(o, 'proj/Samples/deep/鳴る [x].wav') && has(o, 'proj/Song.flp') && has(o, 'readme.txt') && (await shaOf('/Dropped2/proj/Samples/kick.wav')) === fileOf('proj/Samples/kick.wav').sha, J(o));
  } else log(`real OS drop (a folder dragged from the desktop) cannot be made by WebDriver: not verified in ${browserName}`);

  // ---------------------------------------------------------------- persist
  await nav('/');
  const hasPersist = await T(`exists('#fx-persist')`);
  if (hasPersist) {
    await T(`click('#fx-persist')`);
    await ev(`(async () => { for (let i = 0; i < 60 && !/keep this data|did not grant/.test(document.getElementById('fx-status').textContent); i++) await new Promise((r) => setTimeout(r, 50)); return true; })()`);
    const ans = await status();
    check('"Ask the browser to keep it" calls navigator.storage.persist() and reports in words (the answer, or that the browser is asking: Firefox shows a permission prompt)', /keep this data|did not grant|Asking the browser/.test(ans), `${browserName}: ${ans}`);
    log(`persist(): ${browserName} -> "${ans.slice(0, 90)}"`);
  } else log('already persisted');

  // ---------------------------------------------------------------- big directory
  const N = 3000;
  const big = await ev(`(async () => { const t0 = performance.now(); const root = await navigator.storage.getDirectory(); const d = await root.getDirectoryHandle('Big', { create: true }); for (let i = 0; i < ${N}; i++) { const fh = await d.getFileHandle('f' + String(i).padStart(5, '0') + '.txt', { create: true }); } const t1 = performance.now(); const t2 = performance.now(); const ex = window.__files.explorer; const want = ex.navSeq + 1; await ex.navigate('/Big'); const t3 = performance.now(); const rows = document.querySelectorAll('#fx-list .fx-row[data-name]').length; while (ex.statsDone < want) await new Promise((r) => setTimeout(r, 20)); const t4 = performance.now(); return { create: t1 - t0, firstPaint: t3 - t2, allStats: t4 - t2, rows }; })()`);
  check(`a folder of ${N} files lists completely`, big.rows === N, J(big));
  log(`big directory (${N} entries): create ${big.create.toFixed(0)} ms, list+first paint ${big.firstPaint.toFixed(0)} ms, all sizes shown ${big.allStats.toFixed(0)} ms (wall, this machine, load ${os.loadavg()[0].toFixed(1)})`);

  // ---------------------------------------------------------------- the folders of the engine's saved state are not deleted silently (act1 R5)
  await ev(`(async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('fl', { create: true })).getDirectoryHandle('state', { create: true }); const w = await (await d.getFileHandle('registrations.bin', { create: true })).createWritable(); await w.write('x'); await w.close(); return true; })()`);
  await nav('/fl'); await T(`selectRow('state')`); await T(`key('Delete')`); await ev('window.__t.waitModal()');
  check('deleting /fl/state asks with a WARNING about the engine\'s registration / licence data', /WARNING: this holds the engine's saved registration/.test(await ev(`document.querySelector('#fx-modal p').textContent`)));
  await T(`click('#fx-modal-cancel')`); await idle();
  check('Cancel keeps it', J((await opfs('/fl/state')).map((e) => e.path)) === '["registrations.bin"]');
  await nav('/'); await T(`selectRow('fl')`); await T(`key('Delete')`); await ev('window.__t.waitModal()');
  check('so does deleting the folder that holds it (/fl)', /WARNING/.test(await ev(`document.querySelector('#fx-modal p').textContent`)));
  await T(`click('#fx-modal-cancel')`); await idle();
  await ev(`(async () => { await (await navigator.storage.getDirectory()).removeEntry('fl', { recursive: true }); return true; })()`);

  // ---------------------------------------------------------------- close
  await nav('/');
  await T(`click('#fx-close')`);
  check('the close button hides the window and the keyboard returns to the desktop', await ev(`document.getElementById('files-overlay').hidden === true && document.activeElement === document.getElementById('desktop')`));
  await ev(`window.__files.openWindow().then(() => true)`);
  await ev(`(() => { document.getElementById('fx-list').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); return true; })()`);
  check('Escape closes it', await ev(`document.getElementById('files-overlay').hidden === true`));
  check('no Content-Security-Policy violation was reported during the whole run', (await ev('window.__csp.length')) === 0, J(await ev('window.__csp')));
  await ev('window.__t.wipe()');
  void crypto;
}

// ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
// Stage 2 (decision 0216): the window as the engine's file dialog, driven WITHOUT the engine: the page's EngineFiles.select() is called with the request that the real engine made (the shapes of
// docs/investigations/fpick1/tables/engine-requests.json) and the answer it hands to the bridge is read. THE REQUESTS ARE THE TEST'S INPUT (a simulation of the engine); the real engine is in files-engine-dialog.mjs.
export async function selectScenario({ page, check, data, log, browserName }) {
  const ev = page.ev;
  const zbytes = fs.readFileSync(data.zips.find((z) => z.endsWith('/utf8.zip'))).toString('base64');
  const OPEN = { save: false, options: 536875008, filterIndex: 6, title: null, okLabel: null, fileName: null, defaultExt: '.flp', folder: 'C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Projects',
    filters: [['FL Studio song file (*.flp)', '*.flp'], ['Zipped song file (*.zip)', '*.zip'], ['MIDI file (*.mid;*.midi)', '*.mid;*.midi'], ['FL Studio Mobile song file (*.flm)', '*.flm'], ['BeatCreator groove file (*.zgr)', '*.zgr'], ['Supported song files (*.flp;*.zip;*.mid;*.midi;*.flm;*.zgr)', '*.flp;*.zip;*.mid;*.midi;*.flm;*.zgr']] };
  const SAVE = { save: true, options: 536870914, filterIndex: 3, title: null, okLabel: null, fileName: 'Sampler.wav', defaultExt: '.wav', folder: 'C:\\Users\\user\\Downloads\\proj',
    filters: [['FL Studio song file (*.flp)', '*.flp'], ['Zipped song file (*.zip)', '*.zip'], ['Microsoft wave file (*.wav)', '*.wav'], ['MPEG 3 audio file (*.mp3)', '*.mp3']] };
  const J = JSON.stringify;
  await ev(`(async () => {
    const root = await navigator.storage.getDirectory();
    for await (const [n] of root.entries()) await root.removeEntry(n, { recursive: true });
    const put = async (p, bytes) => { let d = root; const parts = p.split('/').filter(Boolean); for (const s of parts.slice(0, -1)) d = await d.getDirectoryHandle(s, { create: true }); const w = await (await d.getFileHandle(parts[parts.length - 1], { create: true })).createWritable(); await w.write(bytes); await w.close(); };
    const enc = (s) => new TextEncoder().encode(s);
    await put('proj/Song.flp', enc('FLhd-song')); await put('proj/other.flp', enc('FLhd-other')); await put('proj/kick.wav', enc('KICK')); await put('proj/snare.wav', enc('SNARE!')); await put('proj/sub/deep.wav', enc('DEEP')); await put('proj/notes.txt', enc('n'));
    await put('midi/a.mid', enc('MThd-a')); await put('midi/b.mid', enc('MThd-b')); await put('midi/c.txt', enc('c'));
    await put('zips/utf8.zip', Uint8Array.from(atob(${J(zbytes)}), (c) => c.charCodeAt(0)));
    window.__sel = { answers: [], n: 0, root: null, dispose: null };
    window.__sel.run = (request) => { const r = document.createElement('div'); r.id = 'sel-root'; document.body.append(r); window.__sel.root = r; const id = ++window.__sel.n; window.__sel.dispose = window.__files.engine.select(id, request, r, (a) => window.__sel.answers.push(a)); return id; };
    window.__sel.end = () => { if (window.__sel.dispose) window.__sel.dispose(); window.__sel.dispose = null; if (window.__sel.root) window.__sel.root.remove(); window.__sel.root = null; };
    window.__sel.q = (s) => window.__sel.root.querySelector(s);
    window.__sel.rows = () => [...window.__sel.root.querySelectorAll('#fx-list .fx-row[data-name]')].map((r) => r.dataset.name + (r.dataset.kind === 'directory' ? '/' : ''));
    window.__sel.row = (name, type, mods) => { const r = [...window.__sel.root.querySelectorAll('#fx-list .fx-row[data-name]')].find((x) => x.dataset.name === name); if (!r) throw new Error('no row ' + name); r.dispatchEvent(new MouseEvent(type, { bubbles: true, ...(mods || {}) })); return true; };
    window.__sel.wait = async (cond, ms = 15000) => { const t0 = performance.now(); while (!cond()) { if (performance.now() - t0 > ms) return false; await new Promise((r) => setTimeout(r, 25)); } return true; };
    window.__sel.ready = () => window.__sel.wait(() => window.__sel.root && window.__sel.q('#fx-list') && window.__files.engine.explorer && window.__files.engine.explorer.root === window.__sel.root && !window.__files.engine.explorer.busy && window.__sel.q('#fx-ok'));
    window.__sel.nav = (p) => window.__files.engine.explorer.navigate(p).then(() => true);
    window.__sel.answer = (n) => window.__sel.wait(() => window.__sel.answers.length >= n).then(() => { const a = window.__sel.answers[n - 1]; return a ? { ok: a.ok, typeIndex: a.typeIndex ?? null, reason: a.reason ?? null, items: (a.items || []).map((i) => [i.role, i.path, i.file ? i.file.size : null, i.file ? i.file.constructor.name : null]) } : null; });
    return true;
  })()`);
  const sel = (q) => ev(`(() => { const e = window.__sel.q(${J(q)}); return e ? { text: e.textContent, value: e.value ?? null, disabled: e.disabled ?? null, hidden: e.hidden ?? null } : null; })()`);
  const click = (q) => ev(`(() => { const e = window.__sel.q(${J(q)}); if (!e) throw new Error('no ' + ${J(q)}); e.click(); return true; })()`);
  const rows = () => ev('window.__sel.rows()');
  const open = async (req) => { const id = await ev(`window.__sel.run(${J(req)})`); await ev('window.__sel.ready()'); return id; };
  const typed = (v) => ev(`(() => { const i = window.__sel.q('#fx-filename'); i.value = ${J(v)}; i.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
  const IND = 'C:\\Users\\user\\Downloads\\';

  // ---- Open: a project, the file types of the engine, the files next to the project
  let n = 0;
  await open(OPEN);
  check('selection mode: the window is built in the given element with the engine\'s title, six types, "Supported song files" selected', /FL Studio asks: Open/.test((await sel('.fx-title strong')).text) && (await ev(`[...window.__sel.root.querySelectorAll('#fx-filter option')].map((o) => o.selected)`)).join() === 'false,false,false,false,false,true');
  check('the engine\'s start folder is only a note; the window starts at the OPFS root (the folder is not mapped)', /Image-Line\\FL Studio\\Projects/.test((await sel('#fx-note')).text) && (await sel('.fx-crumbs')).text === 'Browser files');
  await ev(`window.__sel.row('proj', 'dblclick')`);
  await ev(`window.__sel.wait(() => window.__sel.rows().includes('Song.flp'))`);
  check('only folders and files of the selected type show (flp: Song.flp, other.flp; not the wav/txt)', J(await rows()) === J(['sub/', 'other.flp', 'Song.flp']), J(await rows()));
  await ev(`(() => { const s = window.__sel.q('#fx-filter'); s.value = '2'; s.dispatchEvent(new Event('change')); return true; })()`);
  await ev(`window.__sel.wait(() => !window.__sel.rows().includes('Song.flp'))`);
  check('switching to the MIDI type hides the .flp files', J(await rows()) === J(['sub/']), J(await rows()));
  await ev(`(() => { const s = window.__sel.q('#fx-filter'); s.value = '5'; s.dispatchEvent(new Event('change')); return true; })()`);
  await ev(`window.__sel.wait(() => window.__sel.rows().includes('Song.flp'))`);
  await ev(`window.__sel.row('Song.flp', 'click')`);
  check('the name box follows the selection', (await sel('#fx-filename')).value === 'Song.flp');
  await click('#fx-ok'); n++;
  let a = await ev(`window.__sel.answer(${n})`);
  check('Open a project: the answer is the .flp (R) and its neighbours in the same folder (E; not the other .flp, not the sub-folder), as guest paths under the volume, with the bytes as File objects', a && a.ok && J(a.items.map((i) => [i[0], i[1]])) === J([['R', IND + 'proj\\Song.flp'], ['E', IND + 'proj\\kick.wav'], ['E', IND + 'proj\\notes.txt'], ['E', IND + 'proj\\snare.wav']]) && a.items.every((i) => i[3] === 'File') && a.items[0][2] === 9, J(a));
  check('the answer carries the type the person had selected (5 + 1 = 6) so that GetFileTypeIndex can say it', a && a.typeIndex === 6, J(a?.typeIndex));
  await ev('window.__sel.end()');

  // ---- Open: a MIDI file with the MIDI type selected: alone, type 3
  await open(OPEN);
  await ev(`(() => { const s = window.__sel.q('#fx-filter'); s.value = '2'; s.dispatchEvent(new Event('change')); return true; })()`);
  await ev(`window.__sel.nav('/midi')`);
  check('MIDI type: only the .mid files of the folder show', J(await rows()) === J(['a.mid', 'b.mid']), J(await rows()));
  await ev(`window.__sel.row('b.mid', 'click')`);
  await click('#fx-ok'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('a MIDI file is given alone (R), with type index 3', a && J(a.items.map((i) => [i[0], i[1]])) === J([['R', IND + 'midi\\b.mid']]) && a.typeIndex === 3, J(a));
  await ev('window.__sel.end()');

  // ---- Open: a name that does not exist, typed
  await open(OPEN);
  await ev(`window.__sel.nav('/midi')`);
  await typed('nope.mid'); await click('#fx-ok');
  await ev(`window.__sel.wait(() => /was not found/.test(document.querySelector('#sel-root #fx-status').textContent))`);
  check('a typed name that is not there says so and the dialog stays', /"nope.mid" was not found/.test((await sel('#fx-status')).text) && (await ev('window.__sel.answers.length')) === n);
  await typed('a.mid'); await click('#fx-ok'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('a typed name that is there is taken like a selection (case-insensitive lookup)', a && a.items.length === 1 && a.items[0][1] === IND + 'midi\\a.mid', J(a));
  await ev('window.__sel.end()');

  // ---- Open: a zip is unpacked in the page and the project inside is what the engine gets
  await open(OPEN);
  await ev(`window.__sel.nav('/zips')`);
  check('the zip shows with the "Supported song files" type', J(await rows()) === J(['utf8.zip']), J(await rows()));
  await ev(`window.__sel.row('utf8.zip', 'dblclick')`); n++;
  a = await ev(`window.__sel.answer(${n})`);
  const rec = JSON.parse(await ev('JSON.stringify(window.__files.engine.answers)')).at(-1);
  check('double-click on the zip: it is unpacked next to itself (OPFS) and the engine gets the project inside it, not the zip', a && a.ok && a.items.length === 1 && a.items[0][0] === 'R' && /^C:\\Users\\user\\Downloads\\zips\\utf8\\P\\.*\.flp$/.test(a.items[0][1]) && rec.unpackedZip === '/zips/utf8', J(a) + ' ' + J(rec));
  check('the unpacked files stay in the OPFS', (await ev(`(async () => { let d = await navigator.storage.getDirectory(); for (const s of ['zips', 'utf8', 'P']) d = await d.getDirectoryHandle(s); const out = []; for await (const [k] of d.entries()) out.push(k); return out.sort().join('|'); })()`)).includes('.flp'));
  await ev('window.__sel.end()');

  // ---- Open with several files (FOS_ALLOWMULTISELECT)
  await open({ ...OPEN, options: OPEN.options | 0x200 });
  await ev(`window.__sel.nav('/midi')`);
  await ev(`window.__sel.row('a.mid', 'click')`); await ev(`window.__sel.row('b.mid', 'click', { ctrlKey: true })`);
  await click('#fx-ok'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  if (!a) log('multi debug: ' + J(await ev(`({ answers: window.__sel.answers.length, status: window.__sel.q('#fx-status').textContent, name: window.__sel.q('#fx-filename').value, sel: [...window.__sel.root.querySelectorAll('.fx-row.sel')].map((r) => r.dataset.name), busy: window.__files.engine.explorer.busy, finished: window.__files.engine.explorer.finished, rows: window.__sel.rows(), crumbs: window.__sel.q('.fx-crumbs').textContent })`)));
  check('FOS_ALLOWMULTISELECT: two selected files are two R items', a && J(a.items.map((i) => i[1])) === J([IND + 'midi\\a.mid', IND + 'midi\\b.mid']) && a.items.every((i) => i[0] === 'R'), J(a));
  await ev('window.__sel.end()');
  await open(OPEN);
  await ev(`window.__sel.nav('/midi')`);
  await ev(`window.__sel.row('a.mid', 'click')`); await ev(`window.__sel.row('b.mid', 'click', { ctrlKey: true })`);
  await click('#fx-ok');
  await ev(`window.__sel.wait(() => /Choose one file/.test(document.querySelector('#sel-root #fx-status').textContent))`);
  check('without FOS_ALLOWMULTISELECT two files are refused in words', /Choose one file/.test((await sel('#fx-status')).text) && (await ev('window.__sel.answers.length')) === n);
  await ev('window.__sel.end()');

  // ---- Cancel: the close button, Escape
  await open(OPEN);
  await click('#fx-close'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('the close button cancels the engine\'s dialog (ok:false)', a && a.ok === false && /cancelled/.test(a.reason), J(a));
  await ev('window.__sel.end()');
  await open(OPEN);
  await ev(`(() => { window.__sel.q('#fx-list').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); return true; })()`); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('Escape cancels it too', a && a.ok === false, J(a));
  await ev('window.__sel.end()');

  // ---- Save: the engine's suggested name, the type, the start folder from the engine's folder, the default extension, the overwrite question
  await open(SAVE);
  check('Save: the window starts in the OPFS folder that the engine\'s folder stands for (Downloads\\proj -> /proj), with the suggested name, the Save label and the wav type', (await sel('.fx-crumbs')).text === 'Browser files › proj' && (await sel('#fx-filename')).value === 'Sampler.wav' && (await sel('#fx-ok')).text === 'Save' && (await ev(`window.__sel.q('#fx-filter').selectedOptions[0].textContent`)).includes('wave'));
  check('only .wav files show in the Save window (kick.wav, snare.wav) and the folder', J(await rows()) === J(['sub/', 'kick.wav', 'snare.wav']), J(await rows()));
  await typed('mix'); await click('#fx-ok'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('Save: "mix" gets the default extension and becomes one S item at the guest path of /proj/mix.wav, with type index 3', a && J(a.items.map((i) => [i[0], i[1], i[2]])) === J([['S', IND + 'proj\\mix.wav', null]]) && a.typeIndex === 3, J(a));
  await ev('window.__sel.end()');
  const saved = await ev(`(async () => { const r = await window.__files.engine.onSavedFile({ guestPath: ${J(IND + 'proj\\MIX.WAV')}, size: 5, data: new TextEncoder().encode('RIFFx').buffer }); const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('proj'); const f = await (await d.getFileHandle('mix.wav')).getFile(); return [r, await f.text()]; })()`);
  check('SIMULATED engine write: when the engine has finished writing the target (the saved-file message), the bytes are copied to the OPFS path chosen in the window (guest path compared case-insensitively)', J(saved) === J(['/proj/mix.wav', 'RIFFx']), J(saved));
  check('a saved-file message for a path the person did not choose in the window is left alone', (await ev(`window.__files.engine.onSavedFile({ guestPath: 'C:\\\\Users\\\\user\\\\Downloads\\\\other.wav', size: 1, data: new ArrayBuffer(1) })`)) === null);
  await open(SAVE);
  await typed('kick'); await click('#fx-ok');
  await ev(`window.__sel.wait(() => !window.__sel.q('#fx-modal').hidden)`);
  check('Save over an existing file asks "Replace the file?" (FOS_OVERWRITEPROMPT)', /"kick.wav" already exists/.test((await sel('#fx-modal p')).text));
  await click('#fx-modal-cancel');
  check('No keeps the dialog open and gives no answer', (await ev('window.__sel.answers.length')) === n);
  await click('#fx-ok'); await ev(`window.__sel.wait(() => !window.__sel.q('#fx-modal').hidden)`); await click('#fx-modal-ok'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('Yes answers with the existing file\'s name', a && a.items[0][1] === IND + 'proj\\kick.wav', J(a));
  await ev('window.__sel.end()');
  await open({ ...SAVE, options: 2 });
  await typed('sub'); await click('#fx-ok');
  await ev(`window.__sel.wait(() => /sub/.test(document.querySelector('#sel-root .fx-crumbs').textContent))`);
  check('a typed name that is a folder opens the folder, as the Windows dialog does', /proj › sub/.test((await sel('.fx-crumbs')).text), (await sel('.fx-crumbs')).text);
  await typed('bad:name'); await click('#fx-ok');
  await ev(`window.__sel.wait(() => /cannot contain/.test(document.querySelector('#sel-root #fx-status').textContent))`);
  check('a name Windows would not keep is refused with the reason', /cannot contain/.test((await sel('#fx-status')).text) && (await ev('window.__sel.answers.length')) === n);
  await ev('window.__sel.end()');

  // ---- Folder dialog (FOS_PICKFOLDERS): D + the files in it and below
  await open({ ...OPEN, options: 0x20 | 0x800, folder: null });
  await ev(`window.__sel.nav('/')`);
  check('folder mode: the file type list and name box are gone, only folders are listed, the button says "Choose this folder"', !(await sel('#fx-filename')) && !(await sel('#fx-filter')) && (await sel('#fx-ok')).text === 'Choose this folder' && J(await rows()) === J(['midi/', 'proj/', 'zips/']), J(await rows()));
  await ev(`window.__sel.row('proj', 'click')`);
  await click('#fx-ok'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('Choose the selected folder: the answer is the folder (D) and every file in and below it (E), 7 items', a && a.items[0][0] === 'D' && a.items[0][1] === IND + 'proj' && a.items.slice(1).every((i) => i[0] === 'E') && J(a.items.slice(1).map((i) => i[1]).sort()) === J([IND + 'proj\\Song.flp', IND + 'proj\\kick.wav', IND + 'proj\\mix.wav', IND + 'proj\\notes.txt', IND + 'proj\\other.flp', IND + 'proj\\snare.wav', IND + 'proj\\sub\\deep.wav'].sort()), J(a));
  await ev('window.__sel.end()');
  await open({ ...OPEN, options: 0x20 | 0x800, folder: null });
  await ev(`window.__sel.nav('/midi')`);
  await click('#fx-ok'); n++;
  a = await ev(`window.__sel.answer(${n})`);
  check('with nothing selected the folder shown is chosen', a && a.items[0][0] === 'D' && a.items[0][1] === IND + 'midi' && a.items.length === 4, J(a));
  await ev('window.__sel.end()');
  void log; void browserName;
}

// ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
// The usage block of the measurement beacon (decision 0216): after everything above, the page's own beacon (window.__app.perf.snapshot(): built, not sent) carries counts of what was done and no name.
export async function usageScenario({ page, check, browserName, full }) {
  const ev = page.ev;
  const snap = JSON.parse(await ev('JSON.stringify(window.__app.perf.snapshot())'));
  const f = snap.files;
  check('the beacon (version 3) carries the optional files block once Files was used', snap.v === 3 && f && f.v === 1);
  if (!f) return;
  check('engine dialogs are counted by kind and outcome', f.engine.open >= 1 && f.engine.save >= 1 && f.engine.folder >= 1 && f.engine.answered >= 1 && f.engine.cancelled >= 1 && f.engine.zipUnpacked >= 1 && f.engine.savedBack >= 1, J(f.engine));
  if (full) {
    check('the window and the three upload buttons are counted', f.opens >= 1 && f.pick.files >= 1 && f.pick.folder >= 1 && f.pick.zip >= 1, J({ opens: f.opens, pick: f.pick }));
    check('drops are counted by kind (files, a zip; a folder only where a real folder drop could be made)', f.drop.files >= 1 && f.drop.zips >= 1 && (browserName !== 'chrome' || f.drop.folders >= 1), J(f.drop));
    check('operations are counted', f.ops.mkdir >= 1 && f.ops.rename >= 1 && f.ops.delete >= 1 && f.ops.copy >= 1 && f.ops.move >= 1 && f.download.file >= 1 && f.download.folderZip >= 1 && f.extract.zips >= 6 && f.extract.cancelled >= 1, J({ ops: f.ops, download: f.download, extract: f.extract }));
    check('files written / skipped / failed and the MiB written are counted', f.written > 60 && f.skipped >= 1 && f.failed >= 5 && f.writtenMiB >= 60, J({ written: f.written, skipped: f.skipped, failed: f.failed, mib: f.writtenMiB }));
    check('failure codes seen in the run: zip-crc, zip-method, zip-encrypted, unsafe-path (and nothing outside the list)', ['zip-crc', 'zip-method', 'zip-encrypted', 'unsafe-path'].every((k) => (f.fail[k] ?? 0) >= 1), J(f.fail));
    check('persist() was counted as asked, and the answer (or that none came in 3 s) too', f.persist.asked >= 1 && (f.persist.granted + f.persist.denied + f.persist.pending) >= 1, J(f.persist));
    check('the storage estimate is in MiB (numbers) and persisted is a boolean', Number.isInteger(f.storage.usedMiB) && Number.isInteger(f.storage.quotaMiB) && typeof f.storage.persisted === 'boolean', J(f.storage));
  }
  const json = JSON.stringify(f);
  const names = ['Song.flp', 'serina', 'Sampler', 'readme', 'mini.mid', 'クリスマス', '日本語', 'kick', 'utf8.zip', 'Projects', 'proj'];
  check('PRIVACY: none of the file or folder names used in the run is in the block', names.every((n) => !json.includes(n)), json.slice(0, 300));
  check('PRIVACY: every string in the block is a failure code of the fixed list', (() => { const bad = []; const walk = (v, p) => { if (typeof v === 'string') bad.push(p); else if (v && typeof v === 'object') for (const [k, x] of Object.entries(v)) { if (p === 'fail' && !FAIL.includes(k)) bad.push('fail.' + k); walk(x, p ? p + '.' + k : k); } }; walk(f, ''); return bad.length === 0; })());
}
const FAIL = ['name', 'conflict', 'quota', 'locked', 'notfound', 'kind', 'zip-format', 'zip-crc', 'zip-method', 'zip-encrypted', 'zip-size', 'unsafe-path', 'cancelled', 'unsupported', 'security', 'io', 'other'];
