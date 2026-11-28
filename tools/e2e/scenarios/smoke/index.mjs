// the smoke set, in the order of the output
import start from './start-welcome-default.mjs';
import knob from './knob-right-click-permissions.mjs';
import fileOpen from './file-open-demo.mjs';
import cursorSize from './cursor-size.mjs';
import pluginManager from './plugin-manager-opens.mjs';
import pluginDatabase from './plugin-database-lists-builtins.mjs';
import pluginDatabaseRows from './plugin-database-rows-drawn.mjs';
import pageChrome from './page-chrome.mjs';
import contextMenuNone from './context-menu-none.mjs';
import clipboardLoad from './clipboard-load-permission.mjs';
import clipboardNoToast from './clipboard-no-toast.mjs';
import firstPaint from './first-paint-complete.mjs';

// the marks the smoke set carries (run.mjs refuses any other mark): none (#1499 is done: plugin-manager-opens has no mark)
export const ALLOWED_MARKS = [];

export default [start, knob, fileOpen, cursorSize, pluginManager, pluginDatabase, pluginDatabaseRows, pageChrome, contextMenuNone, clipboardLoad, clipboardNoToast, firstPaint];
