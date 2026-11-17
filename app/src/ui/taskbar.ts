// Decision 0272: the page's end of the engine's taskbar entry (lift/runtime/fle_input.h fle_win_taskbar_buttons / fle_input_host_syscommand, decision 0267). On Windows a minimized window is brought back
// by its button on the taskbar, which sends the window WM_SYSCOMMAND(SC_RESTORE). The virtual machine has no taskbar, so the page shows ONE button while the engine says that a window is minimized and has a
// taskbar button (the Worker reads fle_g3_taskbar_iconic and posts the window's number when it changes; a build without the export posts nothing, and then no button is ever shown). A press gives the engine a
// `syscmd` raw event, which travels the road every input travels (the ring, the IdleDriver, fle_g3_taskbar_syscommand at the engine's idle). The page names no window: the number is the engine's own.
import { IN, SC, type RawInput } from '../input/events.ts';

/** the part of a <button> this class uses (a test gives an object of its own) */
/** what the engine says when no window is minimized (fle_g3_taskbar_iconic returns 0) */
export const NO_WINDOW = 0;

export interface RestoreButton {
  hidden: boolean | string;
  addEventListener(type: 'click', listener: () => void): void;
}

export interface TaskbarHooks {
  /** gives the raw event to the engine as the desktop's input does */
  send(ev: RawInput): void;
  /** the keyboard goes back to the desktop (the focused button disappears when the window is restored) */
  focusDesktop(): void;
}

export class TaskbarRestore {
  private readonly button: RestoreButton;
  private readonly hooks: TaskbarHooks;
  private lid = NO_WINDOW;
  private pressed = 0;

  constructor(button: RestoreButton, hooks: TaskbarHooks) {
    this.button = button;
    this.hooks = hooks;
    button.addEventListener('click', () => this.press());
  }

  /** the engine's number of the minimized window the button restores (0: no button) */
  get window(): number { return this.lid; }
  get shown(): boolean { return !this.button.hidden; }
  /** how many times the button restored (sent the system command) */
  get presses(): number { return this.pressed; }

  /** the Worker's report: the first minimized window with a taskbar button is now `lid` (0: none) */
  update(lid: number): void {
    if (lid === this.lid) return;
    this.lid = lid;
    this.button.hidden = lid === NO_WINDOW;
  }

  /** the engine has ended: nobody is left to restore anything */
  clear(): void { this.update(NO_WINDOW); }

  private press(): void {
    if (this.lid === NO_WINDOW) return;
    this.pressed++;
    this.hooks.send({ k: 'syscmd', kind: IN.SYSCMD, lid: this.lid, cmd: SC.RESTORE });
    this.hooks.focusDesktop();
  }
}
