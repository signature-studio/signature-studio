//! Decision 0100 on the real FLEngine_x64.dll ($FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll; with FLENGINE_PE set and the file
//! missing the tests fail, with the default missing they are skipped): the generated C with and without the virtual-call thunk seeds and the
//! landing seed. The C is about 200 MB: a writer splits it into functions and keeps a hash of each body (and the text of a few).
//!
//! The reference points are `fe-lift lift FILE OUT.c --pdata` runs of the base tree with and without `--no-vcall-thunks` and `--no-landings`
//! (lift/out-wallL/logs/gen-fle-*.txt) and the function-level comparison lift/out-wallL/py/cmpc.py (own parser, independent of this one).

use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

const THUNKS: usize = 82;

fn load() -> Option<Vec<u8>> {
    let explicit = std::env::var_os("FLENGINE_PE");
    let path = match &explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../binaries/win64/FLEngine_x64.dll"),
    };
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(e) if explicit.is_some() => panic!("FLENGINE_PE={}: {e}", path.display()),
        Err(e) => {
            eprintln!("SKIPPED: {}: {e}", path.display());
            None
        }
    }
}

/// Splits the generated C into function definitions (`void f_X(void)` ... `}`): the hash of each body, and the text of `keep`.
struct Funcs {
    partial: Vec<u8>,
    cur: Option<(String, Vec<u8>)>,
    keep: Vec<String>,
    hashes: BTreeMap<String, u64>,
    texts: BTreeMap<String, String>,
}

impl Funcs {
    fn new(keep: &[&str]) -> Funcs {
        Funcs { partial: Vec::new(), cur: None, keep: keep.iter().map(|s| s.to_string()).collect(), hashes: BTreeMap::new(), texts: BTreeMap::new() }
    }

    fn line(&mut self, l: &[u8]) {
        if let Some((name, body)) = &mut self.cur {
            body.extend_from_slice(l);
            body.push(b'\n');
            if l == b"}" {
                let mut h = DefaultHasher::new();
                body.hash(&mut h);
                self.hashes.insert(name.clone(), h.finish());
                if self.keep.contains(name) {
                    self.texts.insert(name.clone(), String::from_utf8_lossy(body).to_string());
                }
                self.cur = None;
            }
        } else if l.starts_with(b"void f_") && l.ends_with(b"(void)") {
            let name = String::from_utf8_lossy(&l[5..l.len() - 6]).to_string();
            self.cur = Some((name, Vec::new()));
        }
    }
}

impl std::io::Write for Funcs {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut rest = buf;
        while let Some(i) = rest.iter().position(|&b| b == b'\n') {
            self.partial.extend_from_slice(&rest[..i]);
            let l = std::mem::take(&mut self.partial);
            self.line(&l);
            rest = &rest[i + 1..];
        }
        self.partial.extend_from_slice(rest);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

const KEEP: &[&str] = &["f_477e73", "f_1376820", "f_1376840", "f_4126f0"];

struct Run {
    st: fe_lift::Stats,
    f: Funcs,
}

fn lift(data: &[u8], seeds: &[u64]) -> Run {
    let mut f = Funcs::new(KEEP);
    let st = fe_lift::generate_with_seeds(data, seeds, &mut f).unwrap();
    Run { st, f }
}

fn diff(a: &Run, b: &Run) -> (Vec<String>, Vec<String>, Vec<String>) {
    let changed = a.f.hashes.iter().filter(|(k, h)| b.f.hashes.get(*k).is_some_and(|g| g != *h)).map(|(k, _)| k.clone()).collect();
    let added = b.f.hashes.keys().filter(|k| !a.f.hashes.contains_key(*k)).cloned().collect();
    let removed = a.f.hashes.keys().filter(|k| !b.f.hashes.contains_key(*k)).cloned().collect();
    (changed, added, removed)
}

#[test]
fn the_thunk_and_landing_seeds_add_functions_and_change_only_the_entry_function() {
    let Some(data) = load() else { return };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let text = fe_decode::Text::from_pe(&pe).unwrap();
    // the seeds of `fe-lift lift FILE OUT.c --pdata`, in the order of the tool
    let mut base_seeds = fe_decode::pdata_seeds(&pe).unwrap();
    base_seeds.extend(fe_decode::adjustor::adjustor_seeds(&pe).unwrap());
    let base_an = fe_decode::analyze_with_seeds(&pe, &base_seeds).unwrap();
    let vcall = fe_decode::adjustor::vcall_seeds(&pe, &base_an).unwrap();
    let landing: Vec<u64> = fe_decode::setjmp::landings(&text, &base_an.insns).into_keys().collect();
    assert_eq!((vcall.len(), landing.as_slice()), (THUNKS, &[0x1376840u64][..]));

    let a = lift(&data, &base_seeds);
    let mut s = base_seeds.clone();
    s.extend(&vcall);
    let b = lift(&data, &s);
    s.extend(&landing);
    let c = lift(&data, &s);

    // a -> b: the thunks. No function of a changes: not one byte of any body
    let (changed, added, removed) = diff(&a, &b);
    assert_eq!((changed.len(), removed.len()), (0, 0), "{changed:?} {removed:?}");
    assert_eq!(added.len(), THUNKS);
    assert_eq!(b.st.functions - a.st.functions, THUNKS);
    assert_eq!(b.st.closure_insns - a.st.closure_insns, 3 * THUNKS);
    assert_eq!(b.st.bad_nodes, 0);
    assert_eq!(b.st.unsupported_total, a.st.unsupported_total, "the thunks add no unsupported instruction");
    assert_eq!((b.st.tail_calls, b.st.gotos), (a.st.tail_calls, a.st.gotos));
    assert_eq!(b.st.setjmp_landings, 0);
    // the thunk of unit 296 is a function of three instructions: the adjusted this, the vtable pointer, the dispatch of the slot
    let t = &b.f.texts["f_477e73"];
    assert!(t.contains("L_477e73: { /* 0x477e73 Add */"), "{t}");
    assert!(t.contains("L_477e77: { /* 0x477e77 Mov */ R[0] = fle_ld((R[1]), 8);}"), "{t}");
    assert!(t.contains("L_477e7a: { /* 0x477e7a Jmp */ { fle_dispatch(fle_ld((R[0] + UINT64_C(0x10)), 8)); return; }}"), "{t}");
    assert_eq!(t.lines().count(), 5, "{t}");

    // b -> c: the landing. The entry function is cut at the setjmp call and the rest is a function of its own
    let (changed, added, removed) = diff(&b, &c);
    assert_eq!(changed, vec!["f_1376820".to_string()]);
    assert_eq!(added, vec!["f_1376840".to_string()]);
    assert!(removed.is_empty());
    assert_eq!(c.st.setjmp_landings, 1);
    assert_eq!(c.st.functions - b.st.functions, 1);
    assert_eq!(c.st.bad_nodes, 0);
    assert_eq!(c.st.unsupported_total, b.st.unsupported_total);
    let e = &c.f.texts["f_1376820"];
    assert!(e.contains("f_412760(); R[4] += 8; }{ fle_landing lj; fle_landing_push(&lj, UINT64_C(0x1376840), R[4]); if (FLE_LANDING_SETJMP(&lj) == 0) { f_1376840(); } else { f_1376840(); } fle_landing_pop(&lj); return; }"), "{e}");
    assert!(!e.contains("L_1376840:"), "{e}");
    assert!(c.f.texts["f_1376840"].contains("L_1376840: { /* 0x1376840 Cmp */"));
    // the guest's longjmp ends in the dispatch of the saved address, not in an unsupported instruction
    let l = &c.f.texts["f_4126f0"];
    assert!(l.contains("L_412752: { /* 0x412752 Jmp */ { fle_dispatch(fle_ld((R[1]), 8)); return; }}"), "{l}");
    assert!(!l.contains("fle_unsupported"), "{l}");
}

#[test]
fn after_the_memory_form_jmp_only_the_import_thunks_remain_unsupported() {
    let Some(data) = load() else { return };
    let pe = fe_pe::Pe::parse(&data).unwrap();
    let mut seeds = fe_decode::pdata_seeds(&pe).unwrap();
    seeds.extend(fe_decode::adjustor::adjustor_seeds(&pe).unwrap());
    let a = lift(&data, &seeds);
    // 525 were unsupported before the memory form (524 import thunks and the jmp [rcx]); the import thunks are all that remain.
    // Later decisions answer some of these thunks, so the count falls: 524 - 6 (decision 0098: acmMetrics, SHGetMalloc,
    // EnumDisplaySettingsW, GetClipCursor, EnumFontFamiliesExW, GetFileVersionInfoSizeW) - 1 (0099: shell32!ILFree stops by name)
    // - 1 (0102: HeapCreate) - 1 (0103: shell32!SHGetFolderPathW, the thunk at 0x593450 now answers CSIDL_FONTS) - 1 (0106: gdi32!CreateBitmap,
    // FLEngine imports it too) - 1 (0107: shfolder.dll!SHGetFolderPathW, the thunk at 0x4262e0 shares the function and the CSIDL_APPDATA answer)
    // - 19 (0108: the 19 rows of the memory DC model, each with exactly one thunk in FLEngine: CreateCompatibleDC 0x423f60, DeleteDC 0x424070,
    // SelectObject 0x424520, SelectPalette 0x424530, CreateCompatibleBitmap 0x423f50, CreateDIBSection 0x423f80, GetDIBColorTable 0x424200, SetDIBColorTable 0x424580,
    // SetTextColor 0x424630, SetBkColor 0x424550, CreateBrushIndirect 0x423f40, CreateHalftonePalette 0x423fd0, GetSysColor 0x424ec0, CreateIconIndirect 0x4248d0,
    // BitBlt 0x423ee0, PatBlt 0x424410, FillRect 0x424b20, GetDIBits 0x424210, RealizePalette 0x424480) = 494
    // - 100 (0111: the 96 rows of decision 0111, all imported by FLEngine; 4 of them have two thunks each: ExtTextOutW, GetRandomRgn,
    // GetTextExtentPoint32W and DrawTextW; 27 rows answer and 69 stop by name, either way the thunk now calls the runtime) = 394
    // - 4 (0112: GetComputerNameExW 0x423430, GetComputerNameW 0x423420, GetFileVersionInfoW 0x423df0, VerQueryValueW 0x423e10; kernel32!LocaleNameToLCID is a
    // delay-load import, not counted here: measured 390 and not 389, the expectation of the decision) = 390
    // - 3 (0116: shell32!SHGetSpecialFolderLocation 0x593370 and the two thunks of shell32!SHGetPathFromIDListW, 0x593350 and 0x593360; measured by comparing the
    // generated C of the tree before and after: exactly these three functions change) = 387
    // - 8 (0117: wintrust.dll!WinVerifyTrust 0x4256d0 and the seven crypt32 functions CryptQueryObject 0x1375030, CryptMsgGetParam 0x1375040, CertCloseStore 0x1375050,
    // CryptMsgClose 0x1375060, CertFindCertificateInStore 0x1375070, CertGetNameStringW 0x1375080, CertFreeCertificateContext 0x1375090, one thunk each; the ninth row,
    // kernel32!GetUserPreferredUILanguages, is a delay-load import with no thunk here, as LocaleNameToLCID above; measured 379) = 379.
    // - 8 (0119: the thunks that gained an answer, measured by comparing the generated C of the tree before and after: exactly these eight functions change in FLEngine:
    // user32!DeleteMenu 0x424940, GetSystemMenu 0x424ed0, SetWindowTextW 0x425530, GetWindowLongPtrW 0x425970, SetClassLongPtrW 0x4259a0 and the three thunks of
    // user32!SendMessageW, 0x425380, 0x425390, 0x4253b0; the two delay-load rows wtsapi32!WTSRegisterSessionNotification and uxtheme!BufferedPaintInit have no thunk here)
    // = 371 (old: 379).
    // - 16 (0122: the thunks that gained an answer, measured by comparing the generated C of FLEngine at HEAD and with this decision: exactly these 16 functions change: kernel32!CreateThread
    // 0x406f80 and 0x423260, ExitThread 0x406fb0, GetExitCodeThread 0x4234e0, ResumeThread 0x423b20, SuspendThread 0x423c50, WaitForMultipleObjects 0x423d40, user32!DispatchMessageW 0x4249a0 and 0x4249c0,
    // GetMessageW 0x424e10, MsgWaitForMultipleObjects 0x4251b0, PeekMessageW 0x425200, PostMessageW 0x425210, PostQuitMessage 0x425220, TranslateMessage 0x425640, WaitMessage 0x4256b0;
    // dwmapi!DwmFlush has no thunk here: it is a delay-load import) = 355 (old: 371).
    // - 74 (0123: the thunks that gained an answer, measured by comparing the generated C of FLEngine of HEAD 8c8131b and of this decision function by function (`tools/cdigest.py`: 59,380 functions both, 0 added, 0
    // removed, 74 changed, all 74 import thunks, 0 other): kernel32!FreeResource 0x4233e0, user32!BeginDeferWindowPos 0x424770, DeferWindowPos 0x424780, CallWindowProcW 0x4247a0, DestroyWindow 0x424990, ... and the
    // comctl32 flat scroll bar family 0x592480..0x5924d0; the delay-load rows (PtInRegion is a gdi32 thunk of the region family) = 281 (old: 355).
    // - 2 (0125: user32!GetWindowDC 0x424f30 and user32!GetDCEx 0x424c60 gained an answer; measured by the generated C of FLEngine: exactly these 2 functions change) = 279 (old: 281).
    // - 30 (0126: 33 rows were appended (tests/wall_fr11.rs); the thunks that gained an answer, measured with this test itself: 279 -> 249, so exactly 30 of the 33 rows have a thunk in FLEngine that was `fle_unsupported`;
    // the rest are delay-load imports with no thunk here (as the earlier decisions note for LocaleNameToLCID), or share a thunk) = 249 (old: 279).
    // - 4 (0129: 4 rows were appended (tests/wall_fr12.rs): user32!GetLastActivePopup, kernel32!GetThreadPriority, user32!GetGuiResources, ole32!CoCreateInstance; the thunks that gained an answer, measured with this
    // test itself: 249 -> 245, so all 4 rows have a thunk in FLEngine that was `fle_unsupported`) = 245 (old: 249).
    // - 7 (0132: 7 rows were appended (tests/rows_0132/mod.rs): gdi32!CreatePatternBrush, user32!ShowScrollBar, kernel32!GetTempFileNameW, wininet!InternetOpenW, InternetSetOptionW, InternetOpenUrlW and
    // InternetCloseHandle; the thunks that gained an answer, measured by comparing the generated C of FLEngine of HEAD 3ce8e7f and of this decision function by function (tools-seh2/gencmp.py: 59,388 functions both,
    // 0 added, 0 removed, 7 changed, all 7 import thunks: f_423710 f_424000 f_4255a0 f_592a00 f_592a30 f_592a40 f_592a60), and by this test itself: 245 -> 238) = 238 (old: 245).
    // - 10 (0136: 10 rows were appended (tests/wall_fr14.rs): comctl32!_TrackMouseEvent, user32!GetMessageTime, GetMessagePos, GetMessageExtraInfo, GetDoubleClickTime, GetKeyboardState, MapVirtualKeyW, MessageBeep,
    // advapi32!OpenProcessToken and GetTokenInformation; the thunks that gained an answer: measured by this test itself, 238 -> 228, and by comparing the generated C of FLEngine function by function
    // (tools/genc_diff.py of the g3in work: all import thunks)) = 228 (old: 238).
    // - 3 (0139: 3 rows were appended (tests/rows_0139/mod.rs): dsound!DirectSoundCreate, DirectSoundCreate8, DirectSoundEnumerateW; FLEngine imports DirectSoundCreate only, so one thunk gained an answer: 228 -> 227) = 227 (old: 228).
    // - 1 (0144: 2 rows were appended (tests/rows_0144/mod.rs): shell32!SHAddToRecentDocs and shcore!GetDpiForMonitor; the first has the thunk 0x593320 in FLEngine (it was `fle_unsupported`; measured by this test itself, 227 -> 226,
    // and by the generated C of FLEngine function by function: 62,326 functions both, 1 changed, f_593320), the second is a delay-load import with no thunk here) = 226 (old: 227).
    // - 1 (0147: 1 row was appended (tests/rows_0147/mod.rs): user32!PostThreadMessageW; FLEngine has its thunk 0x425230 (it was `fle_unsupported`), so one thunk gained an answer: 226 -> 225) = 225 (old: 226).
    // - 1 (0267: 1 row was appended (tests/rows_0267/mod.rs): user32!SetWindowPlacement; FLEngine has its thunks 0x425500 (it was `fle_unsupported`) and 0x425510 uses the same IAT slot 0x1970ed8, so two thunks gained an answer: 225 -> 223, measured with this test and tests/alpha11_flengine_thunk.rs on the real DLL) = 223 (old: 225).
    assert_eq!(a.st.unsupported_total, 223);
    assert_eq!(a.st.unsupported["Jmp: indirect through IAT slot (not lifted)"], 223);
    assert_eq!(a.st.handled["jmp_ind:mem_dispatch"], 1);
}
