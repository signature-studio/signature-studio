//! Expectation tests against the real FLEngine_x64.dll.
//!
//! Path: $FLENGINE_PE, else <repo root>/binaries/win64/FLEngine_x64.dll.
//! If FLENGINE_PE is set and the file is missing the tests fail. If the default
//! path is missing the tests print a SKIPPED line and pass (the binary is not
//! shipped in the repository).
//! Expected values were measured from the extracted file (sizes/hash from the
//! installer stream, header/import values from `fe-pe pe-info` and cross-checked
//! with an independent Python parser).

use fe_pe::{ImportSymbol, Pe, PeError};
use std::path::PathBuf;

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

macro_rules! real {
    ($d:ident, $pe:ident) => {
        let Some($d) = load() else { return };
        let $pe = Pe::parse(&$d).unwrap();
    };
}

fn sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut tail = data[data.len() / 64 * 64..].to_vec();
    tail.push(0x80);
    while tail.len() % 64 != 56 {
        tail.push(0);
    }
    tail.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    let blocks = data[..data.len() / 64 * 64].chunks(64).chain(tail.chunks(64));
    for blk in blocks {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([blk[4 * i], blk[4 * i + 1], blk[4 * i + 2], blk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[test]
fn sha256_self_check() {
    assert_eq!(sha256(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(sha256(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}

#[test]
fn identity() {
    let Some(d) = load() else { return };
    assert_eq!(d.len(), 22_227_016);
    assert_eq!(sha256(&d), "b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d");
}

#[test]
fn headers() {
    real!(d, pe);
    assert_eq!(pe.file_header.machine, 0x8664);
    assert_eq!(pe.file_header.characteristics, 0x2022);
    assert_eq!(pe.file_header.time_date_stamp, 0x6abb_b7e3);
    assert_eq!(pe.optional.magic, 0x20b);
    assert_eq!(pe.optional.image_base, 0x400000);
    assert_eq!(pe.optional.size_of_image, 0x18c0000);
    assert_eq!(pe.optional.size_of_headers, 0x400);
    assert_eq!(pe.optional.address_of_entry_point, 0xf76820);
    assert_eq!(pe.optional.section_alignment, 0x1000);
    assert_eq!(pe.optional.file_alignment, 0x200);
    assert_eq!(pe.optional.subsystem, 2);
    assert_eq!(pe.optional.dll_characteristics, 0x160);
    assert_eq!(pe.optional.number_of_rva_and_sizes, 16);
    let _ = d;
}

#[test]
fn section_table() {
    real!(d, pe);
    let got: Vec<(&str, u32, u32, u32, u32)> = pe
        .sections
        .iter()
        .map(|s| (s.name.as_str(), s.virtual_address, s.virtual_size, s.pointer_to_raw_data, s.size_of_raw_data))
        .collect();
    let want = vec![
        (".text", 0x1000, 0xfa34b0, 0x400, 0xfa3600),
        (".data", 0xfa5000, 0x240008, 0xfa3a00, 0x240200),
        (".bss", 0x11e6000, 0x388e9c, 0, 0),
        (".idata", 0x156f000, 0x670e, 0x11e3c00, 0x6800),
        (".didata", 0x1576000, 0x1460, 0x11ea400, 0x1600),
        (".edata", 0x1578000, 0x94, 0x11eba00, 0x200),
        (".rdata", 0x1579000, 0x45, 0x11ebc00, 0x200),
        (".reloc", 0x157a000, 0x8c590, 0x11ebe00, 0x8c600),
        (".pdata", 0x1607000, 0xa9620, 0x1278400, 0xa9800),
        (".rsrc", 0x16b1000, 0x20e400, 0x1321c00, 0x20e400),
    ];
    assert_eq!(got, want);
    let _ = d;
}

#[test]
fn exports() {
    real!(d, pe);
    let ex = pe.exports().unwrap();
    let got: Vec<(u32, &str, u32, u64)> =
        ex.iter().map(|e| (e.ordinal, e.name.as_deref().unwrap(), e.rva, e.va)).collect();
    assert_eq!(
        got,
        vec![
            (1, "dbkFCallWrapperAddr", 0x11ec2b0, 0x15ec2b0),
            (2, "__dbk_fcall_wrapper", 0x1d5a0, 0x41d5a0),
            (3, "CreateFruityInstance", 0xf75990, 0x1375990),
        ]
    );
    assert!(ex.iter().all(|e| e.forwarder.is_none()));
    let _ = d;
}

#[test]
fn imports_totals() {
    real!(d, pe);
    let im = pe.imports().unwrap();
    assert_eq!(im.len(), 24);
    let total: usize = im.iter().map(|i| i.entries.len()).sum();
    assert_eq!(total, 738);
    assert!(im.iter().flat_map(|i| &i.entries).all(|e| matches!(e.symbol, ImportSymbol::Name { .. })));
    let counts: Vec<(&str, usize)> = im.iter().map(|i| (i.dll.as_str(), i.entries.len())).collect();
    assert_eq!(
        counts,
        vec![
            ("shlwapi.dll", 1), ("opengl32.dll", 2), ("version.dll", 3), ("user32.dll", 227),
            ("oleaut32.dll", 19), ("advapi32.dll", 14), ("kernel32.dll", 184), ("wintrust.dll", 1),
            ("SHFolder.dll", 1), ("crypt32.dll", 7), ("gdi32.dll", 127), ("mpr.dll", 4),
            ("winmm.dll", 24), ("oleacc.dll", 1), ("wininet.dll", 15), ("winspool.drv", 5),
            ("comdlg32.dll", 2), ("comctl32.dll", 30), ("shell32.dll", 22), ("IMAGEHLP.DLL", 3),
            ("ole32.dll", 29), ("msacm32.dll", 14), ("api-ms-win-crt-string-l1-1-0.dll", 1), ("dsound.dll", 2),
        ]
    );
    let _ = d;
}

#[test]
fn import_iat_slots() {
    real!(d, pe);
    let im = pe.imports().unwrap();
    let find = |dll: &str| im.iter().find(|i| i.dll == dll).unwrap();

    let sh = find("shlwapi.dll");
    assert_eq!(sh.first_thunk, 0x15709c8);
    assert_eq!(sh.entries[0].symbol, ImportSymbol::Name { hint: 0, name: "PathUnExpandEnvStringsW".into() });
    assert_eq!(sh.entries[0].iat_va, 0x400000 + 0x15709c8);

    let gl = find("opengl32.dll");
    assert_eq!(gl.entries[0].symbol, ImportSymbol::Name { hint: 0, name: "wglUseFontBitmapsW".into() });
    assert_eq!(gl.entries[1].symbol, ImportSymbol::Name { hint: 0, name: "wglGetProcAddress".into() });
    assert_eq!(gl.entries[1].iat_rva, 0x15709e0);

    let k = find("kernel32.dll");
    assert_eq!(k.first_thunk, 0x1571248);
    assert_eq!(k.entries[0].symbol, ImportSymbol::Name { hint: 0, name: "SetFileAttributesW".into() });
    assert_eq!(k.entries[0].iat_va, 0x400000 + 0x1571248);
    let last = k.entries.last().unwrap();
    assert_eq!(last.symbol, ImportSymbol::Name { hint: 0, name: "SetThreadLocale".into() });
    assert_eq!(last.iat_rva, 0x1571800);

    let ds = find("dsound.dll");
    assert_eq!(ds.entries[1].symbol, ImportSymbol::Name { hint: 0, name: "DirectSoundCreate".into() });
    assert_eq!(ds.entries[1].iat_rva, 0x1572188);

    // every IAT slot lies in a file-backed region, and slots are unique
    let mut seen = std::collections::BTreeSet::new();
    for e in im.iter().flat_map(|i| &i.entries) {
        assert!(pe.rva_to_offset(e.iat_rva).is_ok());
        assert!(seen.insert(e.iat_rva), "duplicate IAT slot {:#x}", e.iat_rva);
    }
    let _ = d;
}

#[test]
fn rva_mapping() {
    real!(d, pe);
    assert_eq!(pe.rva_to_offset(0x1000).unwrap(), 0x400);
    assert_eq!(pe.rva_to_offset(0xf76820).unwrap(), 0xf76820 - 0x1000 + 0x400);
    assert_eq!(pe.va_to_offset(0x1375990).unwrap(), 0xf75990 - 0x1000 + 0x400);
    assert_eq!(pe.va_to_offset(0x41d5a0).unwrap(), 0x1d5a0 - 0x1000 + 0x400);
    // .bss: mapped, not file-backed
    assert_eq!(pe.rva_to_offset(0x11e6000), Err(PeError::RvaNotFileBacked(0x11e6000)));
    assert_eq!(pe.va_to_offset(0x400000 + 0x1000), Ok(0x400));
    // .data tail beyond raw size but inside virtual size is not file-backed
    assert_eq!(pe.rva_to_offset(0x18c0000), Err(PeError::RvaUnmapped(0x18c0000)));
    assert!(matches!(pe.va_to_offset(0x3fffff), Err(PeError::VaOutOfImage(_))));
    let _ = d;
}

#[test]
fn truncated_real_dll_does_not_panic() {
    let Some(d) = load() else { return };
    for len in [0usize, 63, 64, 0x100, 0x1ff, 0x400, 0x1000, 0x11e3c00, 0x11ea000, d.len() - 1] {
        if let Ok(pe) = Pe::parse(&d[..len]) {
            let _ = pe.exports();
            let _ = pe.imports();
        }
    }
    assert!(Pe::parse(&d[..0x100]).is_err());
}
