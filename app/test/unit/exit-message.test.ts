// Decision 0209: the engine's last words as the beacon carries them (src/perf/exit-message.ts): which line is chosen, and that nothing personal (home directories, user names, file names, e-mail, URL, IP) leaves the page.
import { describe, expect, it } from 'vitest';
import { EXIT_MESSAGE_MAX, engineExitMessage, sanitizeEngineText } from '../../src/perf/exit-message.ts';
import { PERF_EXIT_MESSAGE_MAX, beaconProblems } from '../../shared/perf.ts';

describe('sanitizeEngineText: home directories and names', () => {
  it('a Linux / macOS home directory loses the user name and the file name, keeps the extension', () => {
    expect(sanitizeEngineText('open failed: /home/ubuntu/work/proj/song.bin')).toBe('open failed: <path>.bin');
    expect(sanitizeEngineText('cannot read /Users/taro/Music/Samples/kick.wav (2)')).toBe('cannot read <path>.wav (2)');
    expect(sanitizeEngineText('x /root/.config/a')).toBe('x <path>');
  });
  it('a Windows user profile loses the user name, a directory with a space in its name is one component, the file name goes', () => {
    expect(sanitizeEngineText('fle: CreateFileW: C:\\Users\\Jane Doe\\Documents\\Image-Line\\mix.dat failed')).toBe('fle: CreateFileW: <path>.dat failed');
    expect(sanitizeEngineText('D:\\Music\\Taro\\x')).toBe('<path>');
    expect(sanitizeEngineText('\\\\server\\share\\a\\b.txt: no')).toBe('<path>.txt: no');
  });
  it('the virtual machine\'s own paths are kept (they are constants of the engine, and say which module was asked for)', () => {
    const m = 'fle: LoadLibraryW: C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll is not modelled';
    expect(sanitizeEngineText(m)).toBe(m);
    expect(sanitizeEngineText('fle: X: C:\\Windows\\System32\\dsound.dll')).toBe('fle: X: C:\\Windows\\System32\\dsound.dll');
    expect(sanitizeEngineText('reading /proc/self/maps failed')).toBe('reading /proc/self/maps failed');
  });
  it('the names of a person\'s files go wherever they stand, a project (.flp), an archive, a sample', () => {
    expect(sanitizeEngineText('fle: ReadFile: my secret song.flp: not modelled')).toBe('fle: ReadFile: my secret <file>.flp: not modelled');
    expect(sanitizeEngineText('opened serina1-serum2.zip then kick.WAV')).toBe('opened <file>.zip then <file>.WAV');
  });
  it('e-mail addresses, URLs and IPv4 addresses are replaced', () => {
    expect(sanitizeEngineText('mail jane.doe+x@example.co.jp sent')).toBe('mail <email> sent');
    expect(sanitizeEngineText('GET https://www.image-line.com/path?token=abc#x failed')).toBe('GET <url> failed');
    expect(sanitizeEngineText('connect 192.168.1.20 refused')).toBe('connect <ip> refused');
  });
  it('API names, hex addresses and the words of a normal stop message are not touched', () => {
    for (const m of [
      'fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no structured exception handling is modelled, stopped here',
      'fle: pruned function 0x2001040 was executed (prune list, decision 0168; check the CPUID answer of fle_cpuid.c and calls of ippSetCpuFeatures)',
      'fle: indirect call to 0x0 from 0x1800a2b10 (and/or a null function pointer)',
      'limit: the guest asked for 3000000000 bytes (read/write)',
    ]) expect(sanitizeEngineText(m)).toBe(m);
  });
  it('control characters and line breaks become one space; the result is at most 400 characters and ends with an ellipsis when cut', () => {
    expect(sanitizeEngineText('a\nb\r\n\tc\u0000d')).toBe('a b c d');
    const long = sanitizeEngineText('fle: X: ' + 'w'.repeat(1000));
    expect(long).toHaveLength(EXIT_MESSAGE_MAX);
    expect(long.endsWith('…')).toBe(true);
    expect(EXIT_MESSAGE_MAX).toBe(PERF_EXIT_MESSAGE_MAX);
  });
  it('is idempotent (cleaning twice changes nothing more)', () => {
    for (const m of ['C:\\Users\\Jane Doe\\a b\\c.flp x', '/home/u/a/b.c and /Users/t/z', 'a@b.cc http://x/y 10.0.0.1']) {
      const once = sanitizeEngineText(m);
      expect(sanitizeEngineText(once)).toBe(once);
    }
  });
  it('whatever it makes passes the server-side check of the beacon (no home directory, e-mail, URL, control character)', () => {
    const raw = ['C:\\Users\\Jane Doe\\x.flp', '/home/ubuntu/work/a', 'a@b.cc', 'https://h/p', '/Users/taro/z', '\\\\h\\s\\f'].join(' | ');
    const b = { v: 3, sid: 'a'.repeat(12), engine: { present: 'B', exit: { reason: 'stopped', code: 134, presents: 1, message: sanitizeEngineText(raw) } } };
    const probs = beaconProblems(b).filter((p) => p.startsWith('engine.exit'));
    expect(probs).toEqual([]);
  });
});

describe('engineExitMessage: which line says why', () => {
  const info = { reason: 'stopped', code: 134, message: 'exit 134' };
  it('the runtime message wins, with or without the fruity prefix and the [err] marker', () => {
    expect(engineExitMessage(['fruity: == stop: SIGABRT (runtime abort, see runtime message)', 'fruity: runtime message: fle: CreateFileW: not modelled'], info)).toBe('fle: CreateFileW: not modelled');
    expect(engineExitMessage(['[err] fle: A: why', '== stop: SIGABRT (runtime abort, see runtime message)', 'runtime message: fle: A: why'], info)).toBe('fle: A: why');
    expect(engineExitMessage(['[err] fruity: runtime message: fle: B: why'], info)).toBe('fle: B: why');
  });
  it('a runtime message wins over a "fle:" line printed after it', () => {
    expect(engineExitMessage(['fruity: runtime message: fle: first: why', '[err] fle: later: noise'], info)).toBe('fle: first: why');
  });
  it('the LAST runtime message of the log is the one', () => {
    expect(engineExitMessage(['fruity: runtime message: old', 'x', 'fruity: runtime message: new'], info)).toBe('new');
  });
  it('without it: the last "fle:" or "limit:" line, then the stop line, then the trap\'s first line; else null', () => {
    expect(engineExitMessage(['fle: first', 'noise', '[err] fle: second: why', 'fruity: exit code 134'], info)).toBe('fle: second: why');
    expect(engineExitMessage(['limit: out of memory', 'x'], info)).toBe('limit: out of memory');
    expect(engineExitMessage(['x', 'fruity: == stop: SIGSEGV'], info)).toBe('== stop: SIGSEGV');
    expect(engineExitMessage(['x'], { reason: 'trap', code: 1, message: 'RuntimeError: unreachable\n at f' })).toBe('RuntimeError: unreachable');
    expect(engineExitMessage(['x'], info)).toBeNull();
    expect(engineExitMessage([], null)).toBeNull();
    expect(engineExitMessage(['fruity: runtime message: '], info)).toBeNull();
  });
  it('the import of the thunk and the guest return address of the stop report are added in brackets; the import only when the message does not name the API', () => {
    const rep = [
      'fruity: == stop: SIGABRT (runtime abort, see runtime message)',
      'fruity: runtime message: fle: unsupported instruction at 0x425030',
      'fruity: abort kind: fle_unsupported',
      'fruity: the stop is in the lifted thunk f_425030 (innermost lifted function) = `jmp [IAT slot]` of import user32.dll!IsClipboardFormatAvailable (read from the image bytes there)',
      'fruity: guest stack at rsp (return address, 4 shadow slots, then stack arguments): 0x4505f9 0xa6bad8 0x13c11b98',
    ];
    expect(engineExitMessage(rep, info)).toBe('fle: unsupported instruction at 0x425030 [import user32.dll!IsClipboardFormatAvailable] [guest return 0x4505f9]');
    // a message that names the API already: the import is not repeated; the return address is
    expect(engineExitMessage(['fruity: runtime message: fle: LCMapStringW: character of 0x80 or above', 'fruity: the stop is in the lifted thunk f_1 (x) = `jmp [IAT slot]` of import kernel32.dll!LCMapStringW (read from the image bytes there)', 'fruity: guest stack at rsp (return address, 4 shadow slots, then stack arguments): 0x4505f9 0x1'], info)).toBe('fle: LCMapStringW: character of 0x80 or above [guest return 0x4505f9]');
    // no stop report: just the message
    expect(engineExitMessage(['fruity: runtime message: fle: A: why'], info)).toBe('fle: A: why');
  });
  it('a long message is cut before its suffix, so that the import and the return address always survive and the whole is at most 400 characters', () => {
    const m = engineExitMessage(['fruity: runtime message: fle: X: ' + 'w'.repeat(900), 'fruity: the stop is in the lifted thunk f_1 (x) = `jmp [IAT slot]` of import a.dll!Foo (read from the image bytes there)', 'fruity: guest stack at rsp (return address, 4 shadow slots, then stack arguments): 0x4505f9 0x1'], info) as string;
    expect(m).toHaveLength(400);
    expect(m.endsWith('… [import a.dll!Foo] [guest return 0x4505f9]')).toBe(true);
  });
  it('the message has the personal parts removed before it is returned', () => {
    expect(engineExitMessage(['fruity: runtime message: fle: CreateFileW: C:\\Users\\Jane Doe\\Desktop\\mine.flp: not modelled'], info)).toBe('fle: CreateFileW: <path>.flp: not modelled');
  });
  it('a log of 200,000 lines is read from the end, not whole', () => {
    const lines = Array.from({ length: 200000 }, (_, i) => `line ${i}`);
    lines.push('fruity: runtime message: fle: Z: why');
    expect(engineExitMessage(lines, info)).toBe('fle: Z: why');
    const t0 = performance.now();
    expect(engineExitMessage(lines.slice(0, -1), info)).toBeNull();
    expect(performance.now() - t0).toBeLessThan(200);
  });
});
