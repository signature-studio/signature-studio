// The regression set (Issue #1408): one scenario per report of the owner (pm-work/ledger/instructions.tsv), in the order of the output.
// A scenario's mark (expectFail) may only name an issue of ALLOWED_MARKS (the open issues the set was written for); run.mjs refuses any other mark.
import r01 from './r01-hidpi-edge-and-text.mjs';
import r02 from './r02-browser-sounds-stacking.mjs';
import r03 from './r03-open-demo-project.mjs';
import r04 from './r04-clipboard-copy.mjs';
import r05 from './r05-clipboard-paste.mjs';
import r06 from './r06-clipboard-permission-knob.mjs';
import r07 from './r07-topbar-numbers.mjs';
import r08 from './r08-cursor-window-edge.mjs';
import r09 from './r09-cursor-pencil-mode.mjs';
import r10 from './r10-theme-switch-all-13.mjs';
import r11 from './r11-language-switch-all-7.mjs';
import r12 from './r12-recent-persists.mjs';
import r13 from './r13-settings-general-scroll.mjs';
import r14 from './r14-chord-progression-start.mjs';
import r15 from './r15-riff-machine-open.mjs';
import r16 from './r16-topbar-menu-value.mjs';
import r17 from './r17-mouse-1000hz-no-dropout.mjs';
import r18 from './r18-startup-no-dropout-10s.mjs';

export { OPEN_ISSUES as ALLOWED_MARKS } from './common.mjs';

export default [r01, r02, r03, r04, r05, r06, r07, r08, r09, r10, r11, r12, r13, r14, r15, r16, r17, r18];
