// runs opfs_test.js (emscripten, -sMODULARIZE) in a dedicated Worker; the page posts {mode}; the Worker answers with the printed lines
importScripts('opfs_test.js');
onmessage = (e) => {
  const lines = [];
  let done;
  const finished = new Promise((r) => { done = r; });
  createOpfsTest({ arguments: [e.data.mode], print: (t) => { lines.push(t); if (t === 'DONE') done(); }, printErr: (t) => lines.push('E ' + t) }).then(() => {}, (err) => { lines.push('E module failed: ' + err); done(); });
  Promise.race([finished, new Promise((r) => setTimeout(r, e.data.timeout || 120000))]).then(() => postMessage({ lines }));
};
