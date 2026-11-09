'use strict';
// non-blocking Worker: returns to its event loop after every message (what a JSPI/Asyncify-suspended engine looks like from outside)
onmessage = (e) => postMessage(e.data);
