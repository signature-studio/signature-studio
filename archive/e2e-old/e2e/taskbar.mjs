// Decision 0272: the restore button of a minimized window, in a real Chrome, without the engine. Group 'taskbar-restore' of test/e2e/run.mjs.
//
// What is under test: index.html, style.css and src/ui/taskbar.ts as main.ts wires them. The Worker's message "the first minimized window with a taskbar button is N" is simulated by calling
// window.__app.taskbar.update(N) - the same call the EngineClient makes for a {t:'taskbar'} message (no engine is started: the page shows its error for the missing bundle, which does not matter here).
// Checked: hidden until reported; exactly one button however often it is reported; the button takes no room (the stage, the room and the guest's screen do not change); it can be pressed with the keyboard
// (Enter and Space) and the mouse, each press sends one raw event and the keyboard goes back to the desktop; it is gone again when the engine says no window is minimized.

export async function taskbarScenario({ browser, base, check }) {
  const page = await browser.newPage({ width: 1930, height: 1092 });
  try {
    await page.goto(`${base}/`);
    await page.waitFor("window.__app && window.__app.taskbar && document.getElementById('taskbar-restore')", 60000, 'the page is up');
    const probe = () => page.ev(`(() => {
      const b = document.getElementById('taskbar-restore'), r = b.getBoundingClientRect(), st = document.getElementById('stage').getBoundingClientRect();
      const a = window.__app;
      return {
        count: document.querySelectorAll('#taskbar-restore').length, tag: b.tagName, type: b.type, hiddenAttr: b.hidden, display: getComputedStyle(b).display, w: r.width, h: r.height,
        left: r.left - st.left, bottom: st.bottom - r.bottom, stage: [st.left, st.top, st.width, st.height], room: a.screen.room ? [a.screen.room.width, a.screen.room.height] : null, guest: [a.screen.guest.width, a.screen.guest.height],
        resizes: a.screen.resizes.length, inputEvents: a.inputEvents, presses: a.taskbar.presses, window: a.taskbar.window, active: document.activeElement && document.activeElement.id, status: document.getElementById('taskbar').getAttribute('role'),
      };
    })()`);
    const key = async (type, k, code, vk, text) => page.S('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, ...(text ? { text } : {}) });
    const press = async (k, code, vk, text) => { await key('keyDown', k, code, vk, text); await key('keyUp', k, code, vk); };

    const p0 = await probe();
    check('taskbar: before the engine reports a window the control is one <button type="button"> and it is hidden (display none, no size)', p0.count === 1 && p0.tag === 'BUTTON' && p0.type === 'button' && p0.hiddenAttr === true && p0.display === 'none' && p0.w === 0 && p0.h === 0, JSON.stringify(p0));
    check('taskbar: the region around it announces changes (role="status")', p0.status === 'status', String(p0.status));

    await page.ev('window.__app.taskbar.update(41)');
    await page.ev('window.__app.taskbar.update(41)');
    await page.ev('window.__app.taskbar.update(41)');
    const p1 = await probe();
    check('taskbar: reported three times, there is still exactly one button, shown at the bottom left of the stage', p1.count === 1 && p1.hiddenAttr === false && p1.display !== 'none' && p1.w > 40 && p1.h > 16 && p1.left >= 0 && p1.left < 40 && p1.bottom >= 0 && p1.bottom < 40, JSON.stringify(p1));
    check('taskbar: showing it changes no room: the stage, the room, the guest\'s screen and the number of display changes sent are the same', JSON.stringify([p0.stage, p0.room, p0.guest, p0.resizes]) === JSON.stringify([p1.stage, p1.room, p1.guest, p1.resizes]), JSON.stringify({ a: [p0.stage, p0.room, p0.guest, p0.resizes], b: [p1.stage, p1.room, p1.guest, p1.resizes] }));
    const where = await page.ev(`(() => { const b = document.getElementById('taskbar-restore'); const r = b.getBoundingClientRect(); const e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return e === b; })()`);
    check('taskbar: nothing covers the button (it is the element at its own centre)', where === true, String(where));

    // keyboard: Tab from the desktop reaches the button, Enter and Space press it, each press is one raw event, and the keyboard is the desktop's again
    await page.ev("document.getElementById('desktop').focus()");
    await press('Tab', 'Tab', 9);
    const pt = await probe();
    check('taskbar: Tab from the desktop moves the focus to the button', pt.active === 'taskbar-restore', JSON.stringify({ active: pt.active }));
    await press('Enter', 'Enter', 13, '\r');
    await page.waitFor('window.__app.taskbar.presses === 1', 5000, 'Enter pressed the button');
    const pe = await probe();
    check('taskbar: Enter presses it: one system command for the reported window was sent as an input event, and the focus is back on the desktop', pe.presses === 1 && pe.inputEvents === p1.inputEvents + 1 && pe.active === 'desktop' && pe.window === 41, JSON.stringify(pe));
    await page.ev("document.getElementById('taskbar-restore').focus()");
    await press(' ', 'Space', 32, ' ');
    await page.waitFor('window.__app.taskbar.presses === 2', 5000, 'Space pressed the button');
    const ps = await probe();
    check('taskbar: Space presses it too', ps.presses === 2 && ps.inputEvents === p1.inputEvents + 2 && ps.active === 'desktop', JSON.stringify(ps));

    // the mouse
    const c = await page.ev(`(() => { const r = document.getElementById('taskbar-restore').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()`);
    await page.S('Input.dispatchMouseEvent', { type: 'mouseMoved', x: c.x, y: c.y });
    await page.S('Input.dispatchMouseEvent', { type: 'mousePressed', x: c.x, y: c.y, button: 'left', clickCount: 1 });
    await page.S('Input.dispatchMouseEvent', { type: 'mouseReleased', x: c.x, y: c.y, button: 'left', clickCount: 1 });
    await page.waitFor('window.__app.taskbar.presses === 3', 5000, 'the click pressed the button');
    const pm = await probe();
    check('taskbar: a mouse click presses it (and the pointer press on the button was not sent to the engine as a click on the desktop)', pm.presses === 3 && pm.inputEvents === p1.inputEvents + 3, JSON.stringify(pm));

    // another window, then none
    await page.ev('window.__app.taskbar.update(52)');
    const p2 = await probe();
    check('taskbar: another minimized window is reported: still one button, now for that window', p2.count === 1 && p2.hiddenAttr === false && p2.window === 52, JSON.stringify(p2));
    await page.ev('window.__app.taskbar.update(0)');
    const p3 = await probe();
    check('taskbar: the engine says no window is minimized: the button is gone and takes no space', p3.hiddenAttr === true && p3.display === 'none' && p3.w === 0 && p3.h === 0, JSON.stringify(p3));
    const pn = p3.presses;
    await page.ev("document.getElementById('desktop').focus()");
    await press('Tab', 'Tab', 9);
    const p4 = await probe();
    check('taskbar: a hidden button is not in the Tab order, and nothing was sent', p4.active !== 'taskbar-restore' && p4.presses === pn, JSON.stringify({ active: p4.active, presses: p4.presses }));
    // the engine ended while a window was minimized: the page clears the button (exit handler)
    await page.ev('window.__app.taskbar.update(7)');
    await page.ev('window.__app.taskbar.clear()');
    const p5 = await probe();
    check('taskbar: when the engine has ended the button is cleared', p5.hiddenAttr === true && p5.window === 0, JSON.stringify(p5));
  } finally {
    await page.close();
  }
}
