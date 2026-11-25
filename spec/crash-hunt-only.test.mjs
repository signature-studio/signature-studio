#!/usr/bin/env node
// spec/crash-hunt-only.test.mjs : tools/crash-hunt/hunt.mjs --only (Issue #1416): onlyKeep keeps the item, the items on its path (their submenus open the next level) and nothing else. Exit code 0 = all passed.
import { onlyKeep } from '../tools/crash-hunt/hunt.mjs';

let failed = 0;
const check = (name, got, want) => {
  if (got === want) console.log(`ok ${name}`);
  else { failed++; console.log(`not ok ${name}: got ${got}, want ${want}`); }
};
check('no --only keeps every id', onlyKeep(null, 'View/11/1'), true);
check('the item itself is kept', onlyKeep('View/11/1', 'View/11/1'), true);
check('an ancestor on its path is kept', onlyKeep('View/11/1', 'View/11'), true);
check('the root on its path is kept', onlyKeep('View/11/1', 'View'), true);
check('a sibling of the item is not kept', onlyKeep('View/11/1', 'View/11/2'), false);
check('a number with the same start is not kept', onlyKeep('View/11/1', 'View/11/10'), false);
check('a menu with the same start is not kept', onlyKeep('View/1', 'View/11'), false);
check('another root is not kept', onlyKeep('View/11/1', 'File/3'), false);
check('a key is not kept', onlyKeep('View/11/1', 'key F5'), false);
process.exit(failed ? 1 : 0);
