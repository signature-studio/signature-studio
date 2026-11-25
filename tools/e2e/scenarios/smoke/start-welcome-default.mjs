// (a) start-welcome-default: the start, the usage-data box closed with "No thanks", the Welcome closed with Escape (the Default project opens); the engine is idle and the project is open.
import { closeWelcome, dismissProblems, noThanks, WELCOME, waitProjectOpen, waitStable } from '../../lib/flow.mjs';

export default {
  name: 'start-welcome-default',
  expectFail: null,
  async run({ guest, log }) {
    const notes = [];
    const first = await waitStable(guest, 180000);
    notes.push(`first state ${first.state} after ${first.waitedMs} ms, title "${first.title}"`);
    if (first.state !== 'engine-idle') return { ok: false, engineState: first.state, notes };
    if (!(await guest.findWindow((w) => w.cls === WELCOME))) return { ok: false, engineState: first.state, notes: [...notes, 'no Welcome window at the first idle'] };
    await noThanks(guest);
    await closeWelcome(guest);
    const end = await waitProjectOpen(guest, first.title, 120000);
    notes.push(`project open ${end.opened} (waited ${end.waitedMs} ms): title "${first.title}" -> "${end.title}", state ${end.state}`);
    if (await dismissProblems(guest)) notes.push('the "Problems loading the project" box was closed');
    log(notes.at(-1));
    return { ok: end.opened && end.state === 'engine-idle', engineState: end.state, notes };
  },
};
