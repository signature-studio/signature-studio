//! Tests of decision 0139 (docs/decisions/0139-audio-s0-s2.md, PROVISIONAL): the DirectSound model (runtime/fle_dsound.c), the output ring (runtime/fle_audio_ring.c) and their hooks in the scheduler (fle_thread.c) and in the
//! guest's indirect calls (fle_win.c -> fle_com_try_slot).
//!
//! The guest's part is played by C functions of a dispatch table (harness/audio/ds_scenarios.c, built natively here with the REAL runtime files; the COM calls go through the vtables with `fle_dispatch`, exactly as lifted code
//! calls `call [reg+disp]`). Every expected value below is computed by hand from the rule of the Microsoft Learn page that is named next to it, or is a CHOICE WITHOUT A SOURCE of the decision (the S- names). Nothing is compared with a
//! real Windows. The virtual clock is in ticks of 100 ns; 44100 Hz is 226.757... ticks per frame, so the frame counts below are floor(ticks * 44100 / 10^7).
//!
//! 1. Generator: the 3 rows of decision 0139 are the last rows, right after the 598 of decisions 0129, 0132 and 0136, and no pseudo-VA moved.
//! 2. The enumeration, the objects, the error codes, Lock / Unlock, Play / position / Stop, the notifications on the virtual clock, the mixing and the formats, the ring.
//! 3. The scenarios that stop by name (a method that is not modeled, an unknown object, a use after the last Release).
//! 4. The polling mixer of FLEngine against the model in virtual time (the stream in the ring is the sine, sample for sample).
//! 5. Source checks of the parts that cannot run here (the hooks; the wasm transport; no host clock in the model).

mod etxt_common;
mod rows_0132;
mod rows_0139;
mod rows_0140;
mod rows_0144;
mod rows_0147;
mod rows_0267;

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    // FLE_FR15_RT_DIR (a copy of runtime/ with one mutation: the mutation runs of decision 0139 use it); unset: the tree's runtime/
    if let Ok(d) = std::env::var("FLE_FR15_RT_DIR") {
        return PathBuf::from(d);
    }
    manifest().join("../../runtime")
}
fn work_dir() -> PathBuf {
    let d = manifest().join("../../out-wallFR15/test");
    std::fs::create_dir_all(d.join("tmp")).unwrap();
    d
}

const FILES: [&str; 20] = [
    "fle_thread.c", "fle_win.c", "fle_class.c", "fle_event.c", "fle_sys.c", "fle_imports.c", "fle_proc.c", "fle_callback.c", "fle_env.c", "fle_vfs.c", "fle_vmap.c", "fle_vol.c", "fle_host.c", "fle_time.c", "fle_x87ctl.c",
    "fle_mxcsr.c", "fle_x87.c", "fle_locale.c", "fle_dsound.c", "fle_audio_ring.c",
];

fn clang_obj(src: &Path, obj: &Path, strict: bool) {
    let mut cmd = Command::new("clang");
    cmd.args(["-O0", "-std=gnu11", "-fPIE", "-c"]);
    if strict {
        cmd.args(["-Wall", "-Wextra", "-Werror"]);
    } else {
        cmd.arg("-w");
    }
    let cc = cmd.arg("-I").arg(rt_dir()).arg(src).arg("-o").arg(obj).env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed for {}: {}", src.display(), String::from_utf8_lossy(&cc.stderr));
}

/// The scenario program, built once per process (the runtime files with -Werror).
fn scen_exe() -> &'static PathBuf {
    static E: OnceLock<PathBuf> = OnceLock::new();
    E.get_or_init(|| {
        let dir = work_dir().join(etxt_common::per_process("scen"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut objs = Vec::new();
        for f in FILES {
            let o = dir.join(f.replace(".c", ".o"));
            clang_obj(&rt_dir().join(f), &o, true);
            objs.push(o);
        }
        let so = dir.join("ds_scenarios.o");
        clang_obj(&manifest().join("../../harness/audio/ds_scenarios.c"), &so, false);
        objs.push(so);
        // decision 0241: the transport of the scenarios (the stand-in for the AudioWorklet) instead of runtime/fle_audio_transport.c, which is not in FILES
        let st = dir.join("sim_transport.o");
        clang_obj(&manifest().join("../../harness/audio/sim_transport.c"), &st, true);
        objs.push(st);
        let exe = dir.join("scen");
        let mut cmd = Command::new("clang");
        cmd.arg("-pie").arg("-o").arg(&exe);
        for o in &objs {
            cmd.arg(o);
        }
        let l = cmd.arg("-lm").env("TMPDIR", work_dir().join("tmp")).output().expect("clang not found");
        assert!(l.status.success(), "link failed: {}", String::from_utf8_lossy(&l.stderr));
        exe
    })
}

struct Run {
    out: Vec<String>,
    err: String,
    code: Option<i32>,
    signal: Option<i32>,
}

/// The scenarios run with FLE_AUDIO_LEAD_MS=15, the lead that the hand computations of these tests (Learn: "typically about 15 milliseconds") use; the default of the runtime is 60 ms (S-audio-lead): `the_default_lead_is_60_ms...`.
fn run(args: &[&str]) -> Run {
    run_lead(args, Some("15"))
}

fn run_lead(args: &[&str], lead: Option<&str>) -> Run {
    let mut c = Command::new(scen_exe());
    c.args(args).env_remove("FLE_AUDIO_LEAD_MS");
    if let Some(l) = lead {
        c.env("FLE_AUDIO_LEAD_MS", l);
    }
    let r = c.output().expect("run failed");
    Run { out: String::from_utf8_lossy(&r.stdout).lines().map(|s| s.to_string()).collect(), err: String::from_utf8_lossy(&r.stderr).to_string(), code: r.status.code(), signal: r.status.signal() }
}

fn ok(args: &[&str]) -> Vec<String> {
    let r = run(args);
    assert_eq!(r.code, Some(0), "{args:?}: signal {:?} stderr {}", r.signal, r.err);
    r.out
}

fn stops(name: &str, word: &str) {
    let r = run(&[name]);
    assert_eq!(r.signal, Some(6), "{name}: not SIGABRT: code {:?} stderr {}", r.code, r.err);
    assert!(r.err.contains("fle: "), "{name}: no fle: message: {}", r.err);
    assert!(r.err.contains(word), "{name}: message lacks {word:?}: {}", r.err);
}

fn field(l: &str, key: &str) -> u64 {
    let k = format!("{key}=");
    let at = l.find(&k).unwrap_or_else(|| panic!("no {key} in {l}")) + k.len();
    l[at..].split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

/// The number that follows `key` in `l` (for lines that print "... consumed 1543 frames ...").
fn after(l: &str, key: &str) -> u64 {
    let at = l.find(key).unwrap_or_else(|| panic!("no {key:?} in {l}")) + key.len();
    l[at..].split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
}

// ---------------------------------------------------------------------------------------------
// 1. Generator
// ---------------------------------------------------------------------------------------------

#[test]
fn the_three_dsound_rows_are_the_last_rows_after_the_588_and_no_pseudo_va_moved() {
    // src/imports_table.rs: the rows are appended after decision 0132's 7 and decision 0136's 10 (decision 0015: append only). The pseudo-VA of row k (0-based) is 0x7FF900000000 + 0x10 * (k + 1) (tests/wall_fr13.rs checks the rule).
    let src = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let tail: Vec<&str> = src.lines().filter(|l| l.trim_start().starts_with("(\"") && l.contains("\"fle_imp_")).collect();
    let n = tail.len() - rows_0140::ROWS_0140.len() - rows_0144::ROWS_0144.len() - rows_0147::ROWS_0147.len() - rows_0267::ROWS_0267.len(); // decision 0267 appends 1 row (tests/rows_0267) after those of decision 0147; decision 0147 appends 1 row (tests/wall_fr20.rs) after those of decision 0144; decision 0140 appends 4 rows (tests/wall_fr16.rs) after these, decision 0144 2 rows (tests/wall_fr19.rs) after those
    for (i, (dll, name)) in rows_0139::ROWS_0139.iter().enumerate() {
        let want = format!("(\"{dll}\", \"{name}\", \"fle_imp_{name}\", \"jmp_imp:{name}\"),");
        assert_eq!(tail[n - 3 + i].trim(), want, "row {i} of decision 0139");
    }
    // the 7 rows of decision 0132 and the 10 of decision 0136 are right before them
    for (i, (dll, name)) in rows_0132::ROWS_0132.iter().enumerate() {
        let want = format!("(\"{dll}\", \"{name}\", \"fle_imp_{name}\", \"jmp_imp:{name}\"),");
        assert_eq!(tail[n - 3 - 10 - 7 + i].trim(), want, "row {i} of decision 0132");
    }
    assert_eq!(n, 581 + 7 + 10 + 3, "the table has the 581 rows of decision 0129 + 7 + 10 + 3");
    // the COM methods are NOT rows: the slots are pseudo-VAs of their own range (S-com-slot-range)
    assert!(!src.contains("IDirectSoundBuffer::"), "no method of a DirectSound object is a row");
}

// ---------------------------------------------------------------------------------------------
// 2. The model
// ---------------------------------------------------------------------------------------------

#[test]
fn the_list_of_scenarios() {
    let r = run(&["list"]);
    assert_eq!(r.code, Some(0));
    let names: Vec<&str> = r.out.iter().map(|s| s.as_str()).collect();
    assert_eq!(names, ["enum", "create", "errors", "play", "stop_resume", "notify", "notify_alone", "rt_mixer", "late_write", "late_write_sine", "formats", "ring_header", "stop_setfx (stops)", "stop_duplicate (stops)", "stop_3d (stops)",
        "stop_after_release (stops)", "stop_resample (stops)", "stop_second_primary (stops)", "stop_writeprimary (stops)", "stop_bad_this (stops)"]);
}

#[test]
fn the_enumeration_calls_the_callback_for_the_primary_device_twice_first_with_a_null_guid() {
    // Learn DSEnumCallback: "The primary device is enumerated twice, first with NULL in lpGuid and then with its GUID in that parameter"; the callback "Returns TRUE to continue enumerating drivers, or FALSE to stop";
    // lpContext is "the pointer passed to DirectSoundEnumerate". Learn DirectSoundEnumerate: DS_OK (0), or DSERR_INVALIDPARAM (E_INVALIDARG 0x80070057, the Return Values by Number table). The description of the primary device
    // "Primary Sound Driver" is the one Windows gives it; the module name and the second description are choices: S-ds-device-strings; the GUID (ends in the ASCII "FLSTUD"): S-ds-guid. The callback runs at depth 1 (fle_call_guest).
    let o = ok(&["enum"]);
    assert_eq!(o[0], "callback 1: guid=NULL description=\"Primary Sound Driver\" module=\"\" ctx=7 depth=1");
    assert_eq!(o[1], "callback 2: guid=0f5d1e00-0001-4000-8000-464c53545544 description=\"Browser Audio Output\" module=\"\" ctx=7 depth=1");
    assert_eq!(o[2], "hr=0");
    assert_eq!(o[3], "callback 1: guid=NULL description=\"Primary Sound Driver\" module=\"\" ctx=8 depth=1");
    assert_eq!(o[4], "hr=0 (callback stops after one)");
    assert_eq!(o[5], "callbackW 1: guid=NULL description=\"Primary Sound Driver\" module-first-unit=0");
    assert_eq!(o[6], "callbackW 2: guid=set description=\"Browser Audio Output\" module-first-unit=0");
    assert_eq!(o[7], "hr=0 (wide)");
    assert_eq!(o[8], "hr=80070057 (NULL callback)");
}

#[test]
fn the_device_objects_and_their_errors() {
    let o = ok(&["create"]);
    let want = [
        "Create hr=0",
        "vtable first slot is a pseudo-VA in the COM range: 1",
        "Create(device guid) hr=0",
        "Create(bad guid) hr=88780078 ppDS=0",            // Learn DirectSoundCreate8: DSERR_NODRIVER "or the given GUID is not a valid DirectSound device ID" = 0x88780078 (Return Values by Number); *ppDS NULL: S-ds-fail-null
        "Create(pUnkOuter) hr=80040110 ppDS=0",           // "Must be NULL, because aggregation is not supported": DSERR_NOAGGREGATION = CLASS_E_NOAGGREGATION 0x80040110
        "Create(NULL pp) hr=80070057",                    // DSERR_INVALIDPARAM
        "QI IDirectSound hr=0 same=1",
        "QI IDirectSound8 on a v1 object hr=80004002 out=0", // COM: E_NOINTERFACE 0x80004002 and a NULL out pointer (S-ds-noiface: the DirectSound table's 0x000001AE is not an HRESULT)
        "QI bogus hr=80004002 out=0",
        "AddRef 3 Release 2 Release 1",                   // QI added a reference: 1 + 1 (QI) + 1 (AddRef) = 3; Release returns the new count
        "Create8 hr=0",
        "QI IDirectSound8 on a v8 object hr=0 same=1",
        "GetCaps hr=0",
        "caps flags=1f min=100 max=200000 primary=1 rest-zero=1", // S-ds-caps; dwPrimaryBuffers "will always be 1" (Learn DSCAPS); flags PRIMARYMONO|STEREO|8BIT|16BIT|CONTINUOUSRATE = 1+2+4+8+0x10 (dsound.h)
        "GetCaps(bad size) hr=80070057",
        "scenario create done, threads 0, virtual clock 3600000 ms",
    ];
    assert_eq!(o, want);
}

#[test]
fn the_error_codes_of_every_method_follow_the_pages_or_the_choices_named_in_the_comments() {
    let o = ok(&["errors"]);
    let want: &[&str] = &[
        "SetCooperativeLevel(hwnd 0, PRIORITY) 80070057",         // S-ds-hwnd: a window of the virtual machine is required
        "SetCooperativeLevel(level 9) 80070057",                  // only DSSCL_NORMAL..WRITEPRIMARY (1..4) are levels
        "SetCooperativeLevel(a handle that is no window) 80070057", // S-ds-hwnd: the window table of the virtual machine decides
        "primary hr=0",
        "SetFormat before a cooperative level 88780046",          // Learn SetFormat: DSSCL_NORMAL fails; DSERR_PRIOLEVELNEEDED 0x88780046 (S-ds-format-nolevel: no level yet is the same)
        "SetCooperativeLevel(NORMAL) 0",
        "SetFormat under NORMAL 88780046",
        "SetCooperativeLevel(PRIORITY) 0",
        "SetFormat 24-bit 88780064",                              // S-ds-formats: PCM 8/16-bit and float 32-bit only; DSERR_BADFORMAT 0x88780064
        "SetFormat bad block align 88780064",                     // Learn WAVEFORMATEX: nBlockAlign "must be equal" to nChannels * wBitsPerSample / 8
        "SetFormat NULL 80070057",
        "SetFormat 0",
        "GetFormat(size query) hr=0 size=18",                     // Learn GetFormat: NULL pwfx returns the size (S: a PCM format is returned as a WAVEFORMATEX with cbSize = 18 bytes)
        "GetFormat(too small) 80070057",
        "GetFormat 0 written=18 tag=1 ch=2 rate=44100 bits=16",
        "second primary: (stops in a separate scenario)",
        "primary with bytes 80070057",                            // Learn DSBUFFERDESC: dwBufferBytes "must be 0", lpwfxFormat "must be NULL" for the primary buffer
        "primary with a format 80070057",
        "secondary without a format 80070057",
        "secondary 24-bit 88780064",
        "secondary of 1002 bytes (not a whole number of 4-byte blocks) 80070057", // S-ds-size-multiple
        "secondary of 2 bytes 80070057",                          // below DSBSIZE_MIN (4)
        "bad dwSize 80070057",
        "CTRL3D: (stops, separate scenario)",
        "LOCHARDWARE 88780032",                                   // Learn CreateSoundBuffer: no hardware acceleration "fails with either DSERR_CONTROLUNAVAIL or DSERR_INVALIDCALL" (S-ds-hw: INVALIDCALL 0x88780032)
        "LOCHARDWARE|LOCSOFTWARE 80040110",                       // pUnkOuter was 1 here: DSERR_NOAGGREGATION comes first (Learn: "Must be NULL")
        "unknown flag 80070057",
        "secondary hr=0",
        "primary Play without LOOPING 80070057",                  // Learn Play: "Primary buffers must be played with the DSBPLAY_LOOPING flag set"
        "primary Play LOOPING 0",
        "Play(reserved 1) 80070057",                              // dwReserved1 "Must be 0"
        "Play(priority 5) 80070057",                              // dwPriority "must be 0" without DSBCAPS_LOCDEFER
        "Play(LOCSOFTWARE on a buffer without LOCDEFER) 80070057",// Learn Play Remarks
        "SetCurrentPosition(on the primary) 88780032",            // Learn: "This method cannot be called on the primary buffer" (S-ds-primary-call: INVALIDCALL)
        "SetCurrentPosition(3528) 80070057",
        "SetCurrentPosition(3) 80070057",                         // not a block boundary (WAVEFORMATEX: data starts at the beginning of a block)
        "Unlock without Lock 88780032",
        "Lock(offset 3528) 80070057",                             // S-ds-lock-range
        "Lock(bytes 3532) 80070057",
        "Lock(0 bytes) 80070057",
        "Lock(bad flag) 80070057",
        "Lock(on the primary) 88780032",                          // S-ds-primary-lock
        "Lock(3000, 1000) 0 n1=528 n2=472 p2-minus-p1=-3000",     // Learn Lock: the buffer is circular; 3528 - 3000 = 528 bytes to the end, the other 472 at the start; ppvAudioPtr2 points to the beginning of the buffer
        "second Lock 88780032",                                   // S-ds-lock-twice
        "SetCooperativeLevel while locked 88780032",              // Learn: "Do not call this method if any buffers are locked" (S-ds-coop-locked)
        "Unlock(more bytes than locked) 80070057",                // S-ds-unlock-check
        "Unlock(wrong second pointer) 80070057",
        "Unlock(fewer bytes) 0",                                  // Learn Unlock: the counts "might be less than the size of the lock"
        "Lock without ppvAudioPtr2 (3000, 1000) 0 n1=528",        // Learn Lock: with NULL for ppvAudioPtr2 and pdwAudioBytes2 "the lock extends no further than the end of the buffer and does not wrap"
        "Unlock 0",
        "GetStatus (stopped) 0 st=0",
        "GetCurrentPosition (stopped) 0",
        "  play=0 write=0",
        "GetCurrentPosition(NULL, NULL) 0",                       // Learn: both pointers may be NULL ("if the value is not wanted")
        "QI notify 0",
        "notify pointer differs from the buffer: 1",
        "SetNotificationPositions(2) 0",
        "SetNotificationPositions(offset = size) 80070057",
        "SetNotificationPositions(bad event) 80070057",
        "SetNotificationPositions(OFFSETSTOP not last) 80070057", // Learn: DSBPN_OFFSETSTOP "should be the last item in the position-notify array"
        "SetNotificationPositions(NULL, 2) 80070057",
        "Play 0",
        "SetNotificationPositions while playing 88780032",        // Learn: "The buffer must be stopped when this method is called" (S-ds-notify-playing: INVALIDCALL)
        "GetStatus (playing) 0 st=5",                             // DSBSTATUS_PLAYING 1 | DSBSTATUS_LOOPING 4 (dsound.h values)
        "Stop 0",
        "GetStatus (stopped) 0 st=0",
        "Restore 0",                                              // S-ds-restore
        "QI buffer8 on a v8 device 0",                            // Learn CreateSoundBuffer: "Use QueryInterface to obtain IDirectSoundBuffer8"
        "SetVolume(-100) 0",
        "SetVolume(1) 80070057",                                  // Learn SetVolume: DSBVOLUME_MAX (0) to DSBVOLUME_MIN (-10000)
        "SetVolume(-10001) 80070057",
        "SetPan(10001) 80070057",                                 // Learn SetPan: DSBPAN_LEFT (-10000) to DSBPAN_RIGHT (10000)
        "SetFrequency(22050) on a CTRLFREQUENCY-less buffer 8878001e", // DSERR_CONTROLUNAVAIL 0x8878001E: the control must be asked for at creation
        "GetVolume 0 v=-100",
        "refs: Release x2 (QI added two) 2 1",
        "scenario errors done, threads 0, virtual clock 3600000 ms",
    ];
    assert_eq!(o.len(), want.len(), "{o:?}");
    for (i, (a, b)) in o.iter().zip(want.iter()).enumerate() {
        assert_eq!(a, b, "line {i}");
    }
}

#[test]
fn play_moves_the_play_cursor_with_the_virtual_clock_and_the_write_cursor_leads_it_by_15_ms() {
    // 44100 Hz, 16-bit stereo = 4 bytes a frame, a buffer of 882 frames = 3528 bytes; Learn GetCurrentPosition: "The write cursor always leads the play cursor, typically by about 15 milliseconds' worth of audio data" (S-ds-lead-ms:
    // 15 ms = floor(44100 * 15 / 1000) = 661 frames = 2644 bytes). Play starts at the clock tick t0; the position is read after Sleep(k ms), at the clock value `ticks` (printed before the read; a read moves the clock by 10 ticks, S-ds-pos-read-tick).
    // play = (floor(ticks * 44100 / 10^7) mod 882) * 4; write = ((that + 661) mod 882) * 4.
    let o = ok(&["play"]);
    assert_eq!(o[4], "Lock entire hr=0 n1=3528 p2=0 n2=0");
    assert!(o.contains(&"ring after Play: state=2 rate=44100 write=661 read=0 fill=661 lead=661".to_string()), "{o:?}");
    let mut seen = 0;
    for l in o.iter().filter(|l| l.starts_with("t+")) {
        let ticks = after(l, "ticks ");
        let c = ticks * 44100 / 10_000_000;
        let (pl, wr) = ((c % 882) * 4, ((c + 661) % 882) * 4);
        assert_eq!(field(l, "play"), pl, "{l}");
        assert_eq!(field(l, "write"), wr, "{l}");
        assert_eq!(field(l, "write_frames"), c + 661, "the output has been handed over up to consumed + lead: {l}");
        seen += 1;
    }
    assert_eq!(seen, 9);
    // the late poll (a single thread: Sleep(40) moved the clock without a scheduling decision): the frames are handed over from the unchanged buffer, nothing is lost
    let last = o.iter().find(|l| l.starts_with("t+48")).unwrap();
    assert!(last.contains("underruns=0") && last.contains("late=0"), "{last}");
    assert!(o.contains(&"ring content checked up to frame 2778".to_string()), "the ring holds the ramp of the buffer, frame for frame, over 3 cycles of the buffer: {o:?}");
    // the ring: frame k of the stream is (k mod 882) / 32768 on the left and the negative on the right (Lock / write / Unlock of a ramp); the 16-bit sample is divided by 32768 (S-ds-pcm16-scale)
    assert!(o.contains(&"ring frame 5: 0.000152587891 -0.000152587891 ; frame 881: 0.0268859863 -0.0268859863 ; frame 882: 0 0".to_string()), "{o:?}");
    assert!(o.contains(&"stats: locks 1 unlocks 1 plays 1 devices 1 secondary 1".to_string()));
}

#[test]
/* S-audio-lead (the PM's decision on decision 0139): the default lead of the write cursor over the play cursor is 60 ms = floor(44100 * 60 / 1000) = 2646 frames; FLE_AUDIO_LEAD_MS changes it (30 ms = 1323 frames; a value outside 1..1000 is the default). */
fn the_default_lead_is_60_ms_and_the_environment_changes_it() {
    for (env, frames) in [(None, 2646u64), (Some("30"), 1323), (Some("15"), 661), (Some("0"), 2646), (Some("5000"), 2646), (Some("abc"), 2646)] {
        let r = run_lead(&["play"], env);
        assert_eq!(r.code, Some(0), "{env:?}: signal {:?} stderr {}", r.signal, r.err);
        let l = r.out.iter().find(|l| l.starts_with("ring after Play:")).unwrap_or_else(|| panic!("{env:?}: no ring line in {:?}", r.out));
        assert_eq!(field(l, "lead"), frames, "{env:?}: {l}");
        assert_eq!(field(l, "write"), frames, "{env:?}: the output is handed over up to consumed + lead at the Play: {l}");
    }
}

#[test]
fn the_write_cursor_that_the_guest_sees_leads_the_play_cursor_by_15_ms_whatever_the_hand_over_lead() {
    // S-ds-write-cursor (0295): Learn GetCurrentPosition: "The write cursor always leads the play cursor, typically by about 15 milliseconds' worth of audio data". The write cursor that GetCurrentPosition returns is the play cursor + 661 frames (floor(44100 * 15 / 1000)),
    // while the output is handed over up to consumed + the hand-over lead (S-audio-lead: 60 ms = 2646 frames by default, FLE_AUDIO_LEAD_MS): the two are separate quantities. The scenario and the hand computation of play_moves_the_play_cursor_... with the default lead and with 120 ms (5292 frames).
    for (env, lead) in [(None, 2646u64), (Some("120"), 5292)] {
        let r = run_lead(&["play"], env);
        assert_eq!(r.code, Some(0), "{env:?}: signal {:?} stderr {}", r.signal, r.err);
        let mut seen = 0;
        for l in r.out.iter().filter(|l| l.starts_with("t+")) {
            let c = after(l, "ticks ") * 44100 / 10_000_000;
            assert_eq!(field(l, "play"), (c % 882) * 4, "{env:?}: {l}");
            assert_eq!(field(l, "write"), ((c + 661) % 882) * 4, "{env:?}: the write cursor is the play cursor + 15 ms: {l}");
            assert_eq!(field(l, "write_frames"), c + lead, "{env:?}: the output is handed over up to consumed + the hand-over lead: {l}");
            seen += 1;
        }
        assert_eq!(seen, 9, "{env:?}");
    }
}

#[test]
fn stop_keeps_the_frames_already_handed_over_and_a_later_play_neither_repeats_nor_skips() {
    // S-ds-stop-drain: after Stop at 5 ms (consumed 220 frames + the 661 handed over = 881) the play cursor is 881 frames = 3524 bytes, the write cursor is the same (a stopped buffer has committed nothing: S-ds-stopped-write),
    // and the ring holds 881 frames. Learn Stop: "sets the play cursor to the sample that follows the last sample played".
    let o = ok(&["stop_resume"]);
    assert!(o.contains(&"Stop 0".to_string()));
    assert!(o.contains(&"after Stop: play=3524 write=3524 ring write_frames=881 state=1".to_string()), "{o:?}");
    assert!(o.contains(&"30 ms later (stopped): play=3524 write=3524 ring write_frames=881".to_string()));
    // the second Play: the device clock ran on while nothing played (S-ds-idle-gap), the ring jumps to the consumed count; the buffer goes on from frame 881, wraps to 0, 1: the stream at the resume index is 881, 0, 1
    let resume = o.iter().find(|l| l.starts_with("Play 0 (device consumed")).unwrap();
    assert_eq!(after(resume, "consumed "), 1543, "{resume}: 5 ms + 30 ms of Sleep and two position reads: floor(350020 * 44100 / 10^7)");
    assert!(o.iter().any(|l| l.starts_with("ring frames from the resume index") && l.ends_with(": 881 0 1 ")), "{o:?}");
    // 2 ms after the second Play: consumed = resume + 88 (floor of 2 ms of 44100 Hz = 88.2), the play cursor 881 + 88 mod 882 = 87 frames = 348 bytes
    let l = o.iter().find(|l| l.starts_with("2 ms after the second Play")).unwrap();
    assert_eq!(field(l, "play"), 348, "{l}");
    assert_eq!(field(l, "write"), 2992, "(881 + 88 + 661) mod 882 = 748 frames: {l}");
    assert!(l.contains("late=0"));
}

#[test]
fn notifications_signal_the_events_when_the_play_cursor_reaches_the_offsets_and_wake_the_waiting_thread() {
    // a buffer of 882 frames (3528 bytes), notification offsets 0 and 1764 (frame 441 = 10 ms); SetCurrentPosition(1764); Play at tick T. The play cursor reaches offset 0 (the wrap) after 441 frames = 10 ms (ceil(441 * 10^7 /
    // 44100) = 100000 ticks, exactly), offset 1764 after 882 frames = 20 ms, and so on alternately (Learn SetNotificationPositions: "whenever the read or play cursor reaches one of the specified offsets, the associated event is
    // signaled"; a position equal to the start position is not signaled at Play: S-ds-notify-start). WaitForMultipleObjects(2, bWaitAll FALSE) returns the index of the signaled event (Learn).
    let o = ok(&["notify"]);
    let play = o.iter().find(|l| l.starts_with("Play 0 at ")).unwrap();
    let t0 = after(play, "at ");
    let wakes: Vec<&String> = o.iter().filter(|l| l.starts_with("mixer woke")).collect();
    assert_eq!(wakes.len(), 6);
    for (k, l) in wakes.iter().enumerate() {
        let idx = (k % 2) as u64;
        assert!(l.starts_with(&format!("mixer woke: index {idx} at {} ticks", t0 + 100_000 * (k as u64 + 1))), "{l}");
    }
    assert!(o.iter().any(|l| l.starts_with(&format!("main woke at {} ticks", t0 + 750_000))), "Sleep(75): {o:?}");
    assert!(o.iter().any(|l| l.ends_with("mixer wakes so far 6")));
}

#[test]
fn the_virtual_clock_moves_to_the_next_notification_when_every_thread_waits() {
    // the scheduler asks the device for the time of the next notification (fle_audio_next_tick): with no thread that has a time-out, the clock still moves to 10, 20, 30 ms and the wait of the main thread ends
    let o = ok(&["notify_alone"]);
    let w: Vec<&String> = o.iter().filter(|l| l.starts_with("main alone with a sleeper")).collect();
    assert_eq!(w.len(), 3);
    for (k, l) in w.iter().enumerate() {
        assert!(l.starts_with(&format!("main alone with a sleeper: woke index {} at {} ticks", k % 2, 36_000_000_000u64 + 100_000 * (k as u64 + 1))), "{l}");
    }
}

#[test]
fn formats_volume_pan_and_mixing_produce_the_samples_of_the_pages_rules() {
    let o = ok(&["formats"]);
    let s = o.join("\n");
    // 16-bit stereo (16384, -8192) -> (0.5, -0.25): the sample is divided by 32768 (S-ds-pcm16-scale); 8 frames, then silence
    assert!(s.contains("A alone:\n  status after the end 0; ring frames: (0.5,-0.25) (0.5,-0.25) (0.5,-0.25) (0.5,-0.25) (0.5,-0.25) (0.5,-0.25) (0.5,-0.25) (0.5,-0.25) (0,0) (0,0) \n"), "{s}");
    // IEEE float 32-bit stereo is copied
    assert!(s.contains("B alone (float):\n  status after the end 0; ring frames: (0.5,0.25) (0.5,0.25) (0.5,0.25) (0.5,0.25) (0.5,0.25) (0.5,0.25) (0.5,0.25) (0.5,0.25) (0,0) (0,0) \n"), "{s}");
    // mono 16-bit k * 1000 / 32768 on both channels: 0, 0.030517578125 (= 1000/32768), 0.06103515625, 0.091552734375, 0.1220703125, 0.152587890625, 0.18310546875, 0.213623046875 (printed to 7 significant digits)
    assert!(s.contains("(0,0) (0.03051758,0.03051758) (0.06103516,0.06103516) (0.09155273,0.09155273) (0.1220703,0.1220703) (0.1525879,0.1525879) (0.1831055,0.1831055) (0.213623,0.213623) (0,0) \n"), "{s}");
    // 8-bit unsigned: 0 -> -1, 128 -> 0, 255 -> 127/128 = 0.9921875; the second channel 192 -> 64/128 = 0.5
    assert!(s.contains("D alone (8-bit unsigned: 0, 128, 255 / 192):\n  status after the end 0; ring frames: (-1,0.5) (0,0.5) (0.9921875,0.5) \n"), "{s}");
    // pan = 10000 silences the left channel; volume -6000 is 10^(-6000/2000) = 0.001 (Learn SetVolume: hundredths of a dB). B (float 0.5, 0.25) joins at the first frame not yet handed over; A (0.5, -0.25) is looping:
    // joined frames: left 0.5 + 0 = 0.5, right 0.25 + (-0.25 * 0.001) = 0.24975; after B's 8 frames: A alone (0, -0.00025)
    assert!(s.contains("ring frames from B's join: (0.5,0.24975) (0.5,0.24975) (0.5,0.24975) (0.5,0.24975) (0.5,0.24975) (0.5,0.24975) (0.5,0.24975) (0.5,0.24975) (0,-0.00025) (0,-0.00025) \n"), "{s}");
    assert!(s.contains("frame before the join: (0,-0.00025) \n"), "{s}");
    assert!(s.contains("B status 0, A status 5"), "{s}");
}

#[test]
fn the_ring_header_has_the_layout_that_javascript_reads() {
    // fixed layout, version 1 (the offsets are the interface of the page: docs/decisions/0139-audio-s0-s2.md section 9)
    let o = ok(&["ring_header"]);
    assert_eq!(o[0], "magic 52414c46 version 1 channels 2 capacity 32768 data_offset 8296 block_log 256 sizeof 8296 state 0 rate 0 external 0");
    assert_eq!(o[1], "offsetof write_frames 32 read_frames 40 underruns 48 blocks_total 56 blocks 104; sizeof block 32");
    assert_eq!(o[2], "after a device: state 1 rate 44100");
}

// ---------------------------------------------------------------------------------------------
// 3. Stops by name
// ---------------------------------------------------------------------------------------------

#[test]
fn a_method_that_is_not_modeled_stops_by_name() {
    stops("stop_setfx", "IDirectSoundBuffer8::SetFX: IDirectSoundBuffer8::SetFX is not modeled");
    stops("stop_duplicate", "IDirectSound::DuplicateSoundBuffer: IDirectSound::DuplicateSoundBuffer is not modeled");
}

#[test]
fn what_the_model_does_not_do_stops_by_name_and_does_not_guess() {
    stops("stop_3d", "DSBCAPS_CTRL3D (3D buffers) is not modeled");
    stops("stop_resample", "resampling is not modeled");
    stops("stop_second_primary", "a second primary buffer on one device");
    stops("stop_writeprimary", "DSSCL_WRITEPRIMARY");
    stops("stop_after_release", "the object was released (use after the last Release)");
    stops("stop_bad_this", "`this` is not a DirectSound object that this runtime made");
}

// ---------------------------------------------------------------------------------------------
// 4. The polling mixer of FLEngine against the model
// ---------------------------------------------------------------------------------------------

#[test]
fn the_engines_polling_mixer_runs_against_the_model_in_virtual_time_without_an_underrun() {
    // FLEngine's mixer in its polling form (audio-m5.md section 4.2; the real engine's calls are in docs/audio-s0.md): two blocks of 1024 frames (23.2 ms), SetCurrentPosition(block), Play(LOOPING), then Sleep(1) +
    // GetCurrentPosition until the half changes, write the half the play cursor is not in, check that the write cursor is not in it. The first write comes when the play cursor wraps into the first half: 1024 frames = 1024 /
    // 44100 s = 23.22 ms later, i.e. at the first 1 ms poll after that. The stream in the ring is zeros for the first two blocks (the second half is played first, then the first half is still zero), then the sine, sample for
    // sample (block j is heard at stream frames [(2 + j) * 1024, (3 + j) * 1024)).
    let o = ok(&["rt_mixer", "20", "0", "0", "1024"]);
    assert_eq!(o[0], "SetCooperativeLevel 0");
    let w = o.iter().find(|l| l.starts_with("rt_mixer: written")).unwrap();
    assert!(w.starts_with("rt_mixer: written 20 blocks, 464 polls, the engine's underrun check fired 0 times"), "{w}");
    assert!(w.contains("first write at +36000240240 ticks"), "the clock value of the first write: 24 ms of Sleep(1) polls plus the reads: {w}");
    assert!(o.contains(&"rt_mixer: stream frames 0..2047 non-zero: 0; frames 2048..20479 against the sine: 0 mismatches (ring write_frames 21144)".to_string()), "{o:?}");
    let r = o.iter().find(|l| l.starts_with("rt_mixer: ring write_frames")).unwrap();
    assert!(r.ends_with("underruns 0"), "{r}");
    // the default block of FLEngine (4096 frames, 92.9 ms), 6 blocks: the first write 93 ms after Play, the same exact stream
    let o = ok(&["rt_mixer", "6"]);
    let w = o.iter().find(|l| l.starts_with("rt_mixer: written")).unwrap();
    assert!(w.contains("the engine's underrun check fired 0 times"), "{w}");
    assert!(o.iter().any(|l| l.contains("against the sine: 0 mismatches")), "{o:?}");
    // S-audio-lead: the same engine geometry (two blocks of 4096 frames) at the default lead (60 ms) and at 90 ms: the stream is the sine sample for sample (the output reads `lead` frames ahead of the play cursor; the half is rewritten before it is read)
    for lead in [None, Some("90")] {
        let r = run_lead(&["rt_mixer", "6"], lead);
        assert_eq!(r.code, Some(0), "{lead:?}: {}", r.err);
        assert!(r.out.iter().any(|l| l.contains("against the sine: 0 mismatches")), "{lead:?}: {:?}", r.out);
        assert!(r.out.iter().any(|l| l.contains("the engine's underrun check fired 0 times")), "{lead:?}: {:?}", r.out);
    }
}

// ---------------------------------------------------------------------------------------------
// 5. Source checks
// ---------------------------------------------------------------------------------------------

fn read(rel: &str) -> String {
    std::fs::read_to_string(rt_dir().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn the_hooks_are_where_the_decision_says_and_the_generated_code_is_untouched() {
    // the slots reach the model through fle_win_try_thunk (the one hook that already-generated code calls), not through a new hook in fle_dispatch.h: no generated object has to be rebuilt
    let win = read("fle_win.c");
    assert!(win.contains("int fle_com_try_slot(uint64_t va) __attribute__((weak));"));
    let at = win.find("int fle_win_try_thunk(uint64_t va)\n{").unwrap();
    assert!(win[at..at + 400].contains("if (fle_com_try_slot && fle_com_try_slot(va)) return 1;"));
    assert!(!read("fle_dispatch.h").contains("fle_com_try_slot"), "fle_dispatch.h is unchanged: it is included by every generated unit");
    // the scheduler: the device is polled before the waits are tested and gives the next notification time to next_deadline; the pacing branch is guarded by the weak hooks
    let th = read("fle_thread.c");
    assert!(th.contains("if (fle_audio_poll) fle_audio_poll();"));
    assert!(th.contains("if (fle_audio_next_tick) {"));
    assert!(th.contains("fle_audio_paced && fle_audio_wait_until && fle_audio_paced() && next_deadline(&dl)"));
    assert!(th.contains("if (fle_audio_switch_note) fle_audio_switch_note(cur, to);"));
    // fle_event.c: the device sets events from the host side without the register convention
    assert!(read("fle_event.h").contains("int fle_event_host_set(uint64_t h);"));
}

#[test]
fn the_model_reads_no_host_clock_and_starts_no_thread_and_opens_no_file() {
    for f in ["fle_dsound.c", "fle_audio_ring.c"] {
        let s = read(f);
        for bad in ["clock_gettime", "gettimeofday", "<time.h>", "pthread", "fopen(", "usleep", "nanosleep", "<unistd.h>"] {
            assert!(!s.contains(bad), "{f} contains {bad}");
        }
        assert!(s.matches("getenv(").count() <= 2, "{f}: getenv only for FLE_AUDIO_TRACE and FLE_AUDIO_LEAD_MS");
    }
}

#[test]
fn every_slot_of_every_vtable_is_filled_and_the_offsets_the_engine_calls_are_the_ones_audio_m5_read() {
    let s = read("fle_dsound.c");
    // IDirectSoundBuffer: GetCurrentPosition 0x20 = slot 4, GetFormat 0x28 = 5, GetStatus 0x48 = 9, Lock 0x58 = 11, Play 0x60 = 12, SetCurrentPosition 0x68 = 13, SetFormat 0x70 = 14, Stop 0x90 = 18, Unlock 0x98 = 19,
    // Restore 0xa0 = 20 (docs/investigations/audio-m5.md section 3.2, read from the engine's call sites); IDirectSound: CreateSoundBuffer 0x18 = 3, SetCooperativeLevel 0x30 = 6; IDirectSoundNotify: SetNotificationPositions 0x18 = 3
    let b = s.find("static const unsigned b[VT_B_N] = {").unwrap();
    let list = &s[b..b + 600];
    let names: Vec<&str> = list.split(|c: char| c == ',' || c == '{' || c == '}').map(|x| x.trim()).filter(|x| x.starts_with("M_B_")).collect();
    for (slot, name) in [(4, "M_B_GETCURPOS"), (5, "M_B_GETFORMAT"), (9, "M_B_GETSTATUS"), (11, "M_B_LOCK"), (12, "M_B_PLAY"), (13, "M_B_SETCURPOS"), (14, "M_B_SETFORMAT"), (18, "M_B_STOP"), (19, "M_B_UNLOCK"), (20, "M_B_RESTORE")] {
        assert_eq!(names[slot], name, "slot {slot} (offset {:#x})", slot * 8);
    }
    assert_eq!(names.len(), 24);
    let d = s.find("static const unsigned ds[VT_DS_N] = {").unwrap();
    let dn: Vec<&str> = s[d..d + 400].split(|c: char| c == ',' || c == '{' || c == '}').map(|x| x.trim()).filter(|x| x.starts_with("M_DS_")).collect();
    assert_eq!(dn[3], "M_DS_CREATESOUNDBUFFER");
    assert_eq!(dn[6], "M_DS_SETCOOP");
    assert_eq!(dn.len(), 12);
}

// ---------------------------------------------------------------------------------------------
// 7. Decision 0241 (aud1, PROVISIONAL): S-ds-late-write. With an external consumer (here the stand-in of harness/audio/sim_transport.c: the play position is a count the scenario sets, the ring is the C ring) a write that comes after the
//    device handed the frames to the output takes effect while it is still at least the guard (256 frames: S-ds-rewrite-guard) ahead of the output's read position; nearer frames are counted as lost. The scenarios run with the
//    default lead of the runtime, 60 ms = 2646 frames; a device of 8192 frames plays LOOPING from frame 0 and is synchronised at consumed 1000 (3646 frames handed over, all silent) unless the 4th number says another.
// ---------------------------------------------------------------------------------------------

fn late(consumed: u64, first: u64, n: u64, presync: u64) -> Vec<String> {
    let args = ["late_write".to_string(), consumed.to_string(), first.to_string(), n.to_string(), presync.to_string()];
    let a: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let r = run_lead(&a, Some("60"));
    assert_eq!(r.code, Some(0), "{a:?}: signal {:?} stderr {}", r.signal, r.err);
    r.out
}

fn find<'a>(out: &'a [String], key: &str) -> &'a str {
    out.iter().find(|l| l.contains(key)).unwrap_or_else(|| panic!("no line with {key:?} in {out:?}"))
}

/// (ramp frames, silent frames, other frames, non-silent frames outside the region) of the "region:" line
fn region(out: &[String]) -> (u64, u64, u64, u64) {
    let l = find(out, "region: ramp frames=");
    (field(l, "ramp frames"), field(l, "silent frames"), field(l, "other frames"), field(l, "non-silent frames"))
}

/// (events, applied, lost) of the "late writes:" line, and the same three numbers as the page counters (fle_audio_rewrite_stat, which the page reads; absent from the sine scenario: then the same numbers again)
fn late_counts(out: &[String]) -> ((u64, u64, u64), (u64, u64, u64)) {
    let l = find(out, "late writes: events=");
    let own = (field(l, "events"), field(l, "applied"), field(l, "lost"));
    assert_eq!(after(l, "inside the guard="), 0, "the runtime offered frames to the transport that are nearer than the guard to the read position: {l}");
    match l.find("page counters: ") {
        Some(at) => {
            let p = &l[at..];
            (own, (field(p, "events"), field(p, "applied"), field(p, "lost")))
        }
        None => (own, own),
    }
}

fn fnv_of_zero_floats(frames: u64) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for _ in 0..frames * 2 {
        h = (h ^ 0).wrapping_mul(1099511628211);
    }
    h
}

#[test]
fn a_write_that_comes_after_the_hand_over_but_before_the_output_reads_it_reaches_the_output() {
    // consumed 1000 at the write, frames [1500, 2500) written: the output reads frame 1500 500 frames later, the guard is 256: all 1000 frames are handed over again
    let out = late(1000, 1500, 1000, 1000);
    assert_eq!(region(&out), (1000, 0, 0, 0), "{out:?}");
    assert_eq!(late_counts(&out), ((1, 1000, 0), (1, 1000, 0)), "{out:?}");
    // a region that wraps the end of the buffer: the output is at 7500 (synchronised at 7000: frames 7000..9646 handed over), frames [8000, 8500) = positions 8000..8191 and 0..307 are written
    let w = late(7500, 8000, 500, 7000);
    assert_eq!(region(&w), (500, 0, 0, 0), "{w:?}");
    assert_eq!(late_counts(&w), ((1, 500, 0), (1, 500, 0)), "{w:?}");
}

#[test]
fn the_frames_nearer_than_the_guard_or_already_read_are_not_changed_and_the_ones_within_the_guard_are_counted_as_lost() {
    // consumed 1800 at the write, frames [1500, 2500): [1500, 1800) were read before the write (they are the old content and are not counted: they belong to the next lap), [1800, 2056) are within 256 frames of the output
    // (256 lost), [2056, 2500) are in time (444 applied)
    let out = late(1800, 1500, 1000, 1000);
    assert_eq!(region(&out), (444, 556, 0, 0), "{out:?}");
    assert_eq!(late_counts(&out), ((1, 444, 256), (1, 444, 256)), "{out:?}");
    // everything within the guard: nothing is handed over again, 200 frames are lost
    let l = late(1800, 1800, 200, 1000);
    assert_eq!(region(&l), (0, 200, 0, 0), "{l:?}");
    assert_eq!(late_counts(&l), ((1, 0, 200), (1, 0, 200)), "{l:?}");
    // the guard is exact: the first frame at 256 frames from the output is handed over (255 lost: d = 0..255), frame 255 is not
    let g = late(1800, 1800, 257, 1000);
    assert_eq!(late_counts(&g).0, (1, 1, 256), "{g:?}");
}

#[test]
fn a_write_that_is_not_late_changes_nothing_and_is_not_counted() {
    // frames [6000, 7000) are farther ahead than the 3646 frames handed over: no late write, the ring holds the 3646 frames of silence as before (the hash of 7292 zero floats, computed here)
    let out = late(1000, 6000, 1000, 1000);
    assert_eq!(late_counts(&out), ((0, 0, 0), (0, 0, 0)), "{out:?}");
    let l = find(&out, "ring checksum");
    let want = format!("{:016x}", fnv_of_zero_floats(3646));
    assert!(l.contains(&want) && l.contains("frames 0..3646"), "{l}; want {want}");
    // a write that ends before the output (frames [100, 300) at consumed 1000: next lap) is not a late write either
    let b = late(1000, 100, 200, 1000);
    assert_eq!(late_counts(&b).0, (0, 0, 0), "{b:?}");
}

#[test]
fn the_frames_put_over_the_handed_over_ones_join_the_stream_without_a_step() {
    // a sine in two halves; half 1 is written after its first 550 frames were handed over as silence: the ring equals the ideal stream frame for frame (the border of the handed-over-again frames has no step)
    let out = run_lead(&["late_write_sine"], Some("60"));
    assert_eq!(out.code, Some(0), "{}", out.err);
    let l = find(&out.out, "sine: checked frames=");
    assert_eq!(field(l, "checked frames"), 6646, "{l}");
    assert_eq!(field(l, "differing from the ideal stream"), 0, "{l}");
    let step = |key: &str| -> String { let at = l.find(key).unwrap() + key.len(); l[at..].split_whitespace().next().unwrap().to_string() };
    assert_eq!(step("largest step of the ring="), step("ideal="), "{l}");
    assert_eq!(late_counts(&out.out).0, (1, 550, 0), "{:?}", out.out);
}

#[test]
fn the_late_write_is_taken_only_with_an_external_consumer() {
    let s = read("fle_dsound.c");
    let at = s.find("if (fle_audio_external() && b->playing)").expect("the Unlock hook");
    let end = at + s[at..].find("ret(HR_OK);").unwrap();
    assert!(s[at..end].contains("rewrite_unread("), "rewrite_unread is called in the external-consumer branch of Unlock");
    assert_eq!(s.matches("rewrite_unread(").count(), 2, "one definition and one call, nowhere else");
    // the default scenarios (no external consumer) never reach it: the `play` scenario of the tests above counts no late write
    let p = ok(&["play"]);
    assert!(p.iter().all(|l| !l.contains("late writes")), "{p:?}");
}
