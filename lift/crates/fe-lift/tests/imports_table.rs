//! The import table (src/imports_table.rs) in the generator output.
//!
//! The synthetic PE imports three slots:
//! - kernel32.dll!TlsAlloc (in the table): the thunk at 0x1040 calls the runtime.
//! - kernel32.dll!TlsFree (same DLL, not in the table): the thunk at 0x1050 stays unsupported.
//! - k.dll!Fn (not in the table): the thunk at 0x1060 stays unsupported.

use fe_lift::generate;

const IB: u64 = 0x1_4000_0000;
const LFANEW: usize = 0x80;
const OPT: usize = LFANEW + 24;
const SEC: usize = OPT + 240;
const TEXT_RVA: u32 = 0x1000;
const TEXT_RAW: usize = 0x400;
const DATA_RVA: u32 = 0x2000;
const DATA_RAW: usize = 0x800;
const FILE_LEN: usize = 0x1000;

fn w16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn w32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn w64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn data_at(rva: u32) -> usize {
    DATA_RAW + (rva - DATA_RVA) as usize
}
fn put(b: &mut [u8], rva: u32, bytes: &[u8]) {
    let o = data_at(rva);
    b[o..o + bytes.len()].copy_from_slice(bytes);
}
fn rel(next: u32, target: u32) -> [u8; 4] {
    ((target as i64 - next as i64) as i32).to_le_bytes()
}

fn build() -> Vec<u8> {
    let mut b = vec![0u8; FILE_LEN];
    w16(&mut b, 0, 0x5a4d);
    w32(&mut b, 0x3c, LFANEW as u32);
    w32(&mut b, LFANEW, 0x4550);
    let f = LFANEW + 4;
    w16(&mut b, f, 0x8664);
    w16(&mut b, f + 2, 2);
    w16(&mut b, f + 16, 240);
    w16(&mut b, f + 18, 0x2022);
    let o = OPT;
    w16(&mut b, o, 0x20b);
    w32(&mut b, o + 16, TEXT_RVA);
    w64(&mut b, o + 24, IB);
    w32(&mut b, o + 32, 0x1000);
    w32(&mut b, o + 36, 0x200);
    w32(&mut b, o + 56, 0x3000);
    w32(&mut b, o + 60, 0x400);
    w16(&mut b, o + 68, 3);
    w32(&mut b, o + 108, 16);
    w32(&mut b, o + 120, DATA_RVA);
    w32(&mut b, o + 124, 40);

    b[SEC..SEC + 5].copy_from_slice(b".text");
    w32(&mut b, SEC + 8, 0x400);
    w32(&mut b, SEC + 12, TEXT_RVA);
    w32(&mut b, SEC + 16, 0x400);
    w32(&mut b, SEC + 20, TEXT_RAW as u32);
    w32(&mut b, SEC + 36, 0x6000_0020);
    let s1 = SEC + 40;
    b[s1..s1 + 5].copy_from_slice(b".data");
    w32(&mut b, s1 + 8, 0x800);
    w32(&mut b, s1 + 12, DATA_RVA);
    w32(&mut b, s1 + 16, 0x800);
    w32(&mut b, s1 + 20, DATA_RAW as u32);
    w32(&mut b, s1 + 36, 0xc000_0040);

    // Descriptor 1: kernel32.dll (OFT 0x2100, Name 0x2300, FirstThunk 0x2200).
    // Descriptor 2: k.dll (OFT 0x2120, Name 0x2320, FirstThunk 0x2220).
    // The zero descriptor after them terminates the table.
    let d = data_at(DATA_RVA);
    w32(&mut b, d, 0x2100);
    w32(&mut b, d + 12, 0x2300);
    w32(&mut b, d + 16, 0x2200);
    w32(&mut b, d + 20, 0x2120);
    w32(&mut b, d + 32, 0x2320);
    w32(&mut b, d + 36, 0x2220);
    put(&mut b, 0x2300, b"kernel32.dll\0");
    put(&mut b, 0x2320, b"k.dll\0");
    put(&mut b, 0x2400, b"\0\0TlsAlloc\0");
    put(&mut b, 0x2430, b"\0\0TlsFree\0");
    put(&mut b, 0x2440, b"\0\0Fn\0");
    for (rva, v) in [(0x2100, 0x2400u64), (0x2108, 0x2430), (0x2200, 0x2400), (0x2208, 0x2430), (0x2120, 0x2440), (0x2220, 0x2440)] {
        w64(&mut b, data_at(rva), v);
    }

    // Entry 0x1000 calls one thunk per import. Each thunk is `jmp [IAT slot]`.
    let t = TEXT_RAW;
    let code: &[(u32, Vec<u8>)] = &[
        (0x1000, [&[0xe8][..], &rel(0x1005, 0x1040)].concat()),
        (0x1005, [&[0xe8][..], &rel(0x100a, 0x1050)].concat()),
        (0x100a, [&[0xe8][..], &rel(0x100f, 0x1060)].concat()),
        (0x100f, vec![0xc3]),
        (0x1040, [&[0xff, 0x25][..], &rel(0x1046, 0x2200)].concat()),
        (0x1050, [&[0xff, 0x25][..], &rel(0x1056, 0x2208)].concat()),
        (0x1060, [&[0xff, 0x25][..], &rel(0x1066, 0x2220)].concat()),
    ];
    for (rva, bytes) in code {
        let o = t + (rva - TEXT_RVA) as usize;
        b[o..o + bytes.len()].copy_from_slice(bytes);
    }
    b
}

#[test]
fn table_thunk_calls_runtime_and_others_stay_unsupported() {
    let img = build();
    let mut out = Vec::new();
    let st = generate(&img, &mut out).unwrap();
    let c = String::from_utf8(out).unwrap();

    let tls_alloc = IB + 0x1040;
    let tls_free = IB + 0x1050;
    let fn_thunk = IB + 0x1060;

    assert_eq!(st.handled["jmp_imp:TlsAlloc"], 1);
    assert!(!st.handled.contains_key("jmp_imp:TlsFree"));
    assert!(c.contains("fle_imp_TlsAlloc(); return;"));
    assert!(!c.contains(&format!("L_{tls_alloc:x}: {{ /* {tls_alloc:#x} Jmp */ fle_unsupported")));
    assert!(!c.contains("fle_imp_TlsFree"));

    assert!(c.contains(&format!("L_{tls_free:x}: {{ /* {tls_free:#x} Jmp */ fle_unsupported(UINT64_C({tls_free:#x})); }}")));
    assert!(c.contains(&format!("L_{fn_thunk:x}: {{ /* {fn_thunk:#x} Jmp */ fle_unsupported(UINT64_C({fn_thunk:#x})); }}")));
    assert_eq!(st.unsupported["Jmp: indirect through IAT slot (not lifted)"], 2);
    assert!(!st.unsupported.contains_key("Jmp: indirect target unresolved"));
}
