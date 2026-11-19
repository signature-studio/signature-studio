//! Tests for VirtualProtect, VirtualQuery, CreateFileMappingW, MapViewOfFile and UnmapViewOfFile in runtime/fle_vmap.c
//! (docs/decisions/0024-apis-a3-a5-a13.md; sources: docs/investigations/apis-a3-a5-a13-run.md).
//!
//! Expected values are written here from the rules on the Microsoft pages (VirtualProtect, VirtualQuery,
//! MEMORY_BASIC_INFORMATION, Memory Protection Constants, CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, File
//! Mapping Security and Access Rights, System Error Codes), not read back from the C code. A value the pages do not
//! give is marked "assumption", "inference" or "choice" and is the only thing such a test pins down. The host is Linux:
//! nothing is compared with a real Windows, and no host file is read or written.

mod apis_common;
use apis_common::*;

const A13: [&str; 5] = ["VirtualProtect", "VirtualQuery", "CreateFileMappingW", "MapViewOfFile", "UnmapViewOfFile"];

#[test]
fn rows_are_in_the_table_and_declared_in_the_header() {
    let table = std::fs::read_to_string(manifest().join("src/imports_table.rs")).unwrap();
    let h = std::fs::read_to_string(src_dir().join("fle_vmap.h")).unwrap();
    for n in A13 {
        let row = format!("(\"kernel32.dll\", \"{n}\", \"fle_imp_{n}\", \"jmp_imp:{n}\")");
        assert_eq!(table.matches(&row).count(), 1, "{n}: exactly one row in imports_table.rs");
        assert!(h.contains(&format!("void fle_imp_{n}(void)")), "{n} is declared in fle_vmap.h");
    }
}

/// The C helpers that decode a MEMORY_BASIC_INFORMATION written by VirtualQuery. The offsets are those of the struct
/// on the MEMORY_BASIC_INFORMATION page for x64 (PVOID, PVOID, DWORD, WORD (+2 pad), SIZE_T, DWORD, DWORD, DWORD): the
/// test declares its own mirror struct, so the layout is not read from the runtime.
const MBI: &str = r#"
typedef struct { void *BaseAddress; void *AllocationBase; uint32_t AllocationProtect; uint16_t PartitionId; uint64_t RegionSize; uint32_t State; uint32_t Protect; uint32_t Type; } MBI;
_Static_assert(sizeof(MBI) == 48, "MEMORY_BASIC_INFORMATION is 48 bytes on x64");
_Static_assert(__builtin_offsetof(MBI, RegionSize) == 24, "RegionSize at 24");
_Static_assert(__builtin_offsetof(MBI, State) == 32, "State at 32");
_Static_assert(__builtin_offsetof(MBI, Type) == 40, "Type at 40");
static MBI mbi;
static uint64_t vq(uint64_t addr) { memset(&mbi, 0xCC, sizeof mbi); return CALL(fle_imp_VirtualQuery, addr, A(&mbi), sizeof mbi); }
#define MEM_COMMIT 0x1000u
#define MEM_RESERVE 0x2000u
#define MEM_PRIVATE 0x20000u
#define MEM_MAPPED 0x40000u
static uint64_t vm_alloc(uint64_t addr, uint64_t size, uint32_t type, uint32_t prot) { return CALL(fle_imp_VirtualAlloc, addr, size, type, prot); }
"#;

fn run_m(name: &str, body: &str) -> Vec<String> {
    let o = run_src(name, MBI, body);
    assert!(o.ok, "{name} did not exit normally (signal {:?}): {}\n{}", o.signal, o.stderr, o.stdout);
    o.stdout.lines().map(|s| s.to_string()).collect()
}

fn abort_m(prefix: &str, setup: &str, cases: &[(&str, &str)]) {
    for (label, stmts) in cases {
        let body = format!("{setup}\n    {stmts}\n    printf(\"returned %llu\\n\", U(R[0]));");
        let o = run_src(&format!("{prefix}-{label}"), MBI, &body);
        assert!(!o.ok, "{label}: expected an abort, got stdout {}", o.stdout);
        assert!(!o.stdout.contains("returned"), "{label}: returned instead of stopping");
        assert!(o.stderr.contains("fle:"), "{label}: no fle: message: {}", o.stderr);
        assert_eq!(o.signal, Some(6), "{label}: not SIGABRT");
    }
}

// ---- VirtualQuery ----

#[test]
fn query_describes_reserved_and_committed_pages_of_a_virtual_alloc_region() {
    let l = run_m(
        "vmap-query",
        r#"
    uint64_t base = vm_alloc(0, 0x10000, MEM_RESERVE, 0x04), r;
    /* reserved: State MEM_RESERVE, the whole region is one run; Protect is undefined for reserved pages */
    r = vq(base);
    printf("a ret %llu base %d ab %d ap %x size %llx state %x type %x\n", U(r), (uint64_t)mbi.BaseAddress == base, (uint64_t)mbi.AllocationBase == base, mbi.AllocationProtect, U(mbi.RegionSize), mbi.State, mbi.Type);
    /* commit 3 pages in the middle with PAGE_READONLY */
    vm_alloc(base + 0x2000, 0x3000, 0x1000, 0x02);
    r = vq(base);
    printf("b size %llx state %x\n", U(mbi.RegionSize), mbi.State);
    r = vq(base + 0x2000);
    printf("c base_off %llx ab_off %llx size %llx state %x prot %x ap %x type %x\n", U((uint64_t)mbi.BaseAddress - base), U((uint64_t)mbi.AllocationBase - base), U(mbi.RegionSize), mbi.State, mbi.Protect, mbi.AllocationProtect, mbi.Type);
    /* a query inside a page is rounded down to the page, and the run is counted from the page */
    r = vq(base + 0x3123);
    printf("d base_off %llx size %llx state %x prot %x\n", U((uint64_t)mbi.BaseAddress - base), U(mbi.RegionSize), mbi.State, mbi.Protect);
    r = vq(base + 0x5000);
    printf("e base_off %llx size %llx state %x\n", U((uint64_t)mbi.BaseAddress - base), U(mbi.RegionSize), mbi.State);
    r = vq(base + 0xF000);
    printf("f size %llx state %x\n", U(mbi.RegionSize), mbi.State);
    /* the structure is 48 bytes; a larger buffer length gives the same count */
    r = CALL(fle_imp_VirtualQuery, base, A(&mbi), 100); printf("g %llu\n", U(r));
    /* the page: the return value is the number of bytes returned in the buffer */
    printf("pid %u\n", mbi.PartitionId);
"#,
    );
    assert_eq!(
        l,
        [
            // page: "State: MEM_RESERVE"; "AllocationProtect: the protection when the region was initially allocated";
            // the region is MEM_PRIVATE (a VirtualAlloc region); 0x10000 bytes; 48 bytes returned
            "a ret 48 base 1 ab 1 ap 4 size 10000 state 2000 type 20000",
            // page: the run of pages with the same state, counted from the queried page: 2 reserved pages
            "b size 2000 state 2000",
            "c base_off 2000 ab_off 0 size 3000 state 1000 prot 2 ap 4 type 20000",
            // page: "lpAddress ... is rounded down to the next page boundary"
            "d base_off 3000 size 2000 state 1000 prot 2",
            "e base_off 5000 size b000 state 2000",
            "f size 1000 state 2000",
            "g 48",
            // choice: the page gives no meaning to PartitionId; zero
            "pid 0",
        ]
    );
}

#[test]
fn query_reports_the_protection_of_each_commit_and_splits_runs() {
    let l = run_m(
        "vmap-query-runs",
        r#"
    uint64_t base = vm_alloc(0, 0x6000, 0x3000, 0x04);   /* reserve + commit, READWRITE */
    uint64_t r;
    vm_alloc(base + 0x2000, 0x1000, 0x1000, 0x02);        /* page 2 READONLY */
    vm_alloc(base + 0x4000, 0x1000, 0x1000, 0x20);        /* page 4 EXECUTE_READ */
    r = vq(base);          printf("p0 size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    r = vq(base + 0x2000); printf("p2 size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    r = vq(base + 0x3000); printf("p3 size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    r = vq(base + 0x4000); printf("p4 size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    r = vq(base + 0x5000); printf("p5 size %llx prot %x ap %x\n", U(mbi.RegionSize), mbi.Protect, mbi.AllocationProtect);
    /* page: the allocation protection stays the value of the original reservation */
"#,
    );
    assert_eq!(
        l,
        ["p0 size 2000 prot 4", "p2 size 1000 prot 2", "p3 size 1000 prot 4", "p4 size 1000 prot 20", "p5 size 1000 prot 4 ap 4"]
    );
}

#[test]
fn query_of_a_view_reports_mem_mapped() {
    let l = run_m(
        "vmap-query-view",
        r#"
    uint64_t h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x5000, 0), v, r;
    v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);
    r = vq(v);
    printf("a ret %llu base %d ab %d size %llx state %x prot %x type %x\n", U(r), (uint64_t)mbi.BaseAddress == v, (uint64_t)mbi.AllocationBase == v, U(mbi.RegionSize), mbi.State, mbi.Protect, mbi.Type);
    r = vq(v + 0x2345);
    printf("b base_off %llx size %llx\n", U((uint64_t)mbi.BaseAddress - v), U(mbi.RegionSize));
    r = CALL(fle_imp_UnmapViewOfFile, v); printf("unmap %llu\n", U(r));
"#,
    );
    // page: pages of a mapped view are MEM_MAPPED; the view is committed (SEC_COMMIT is assumed); the size is the
    // view's size rounded up to the page size; AllocationBase is the view's base
    assert_eq!(l, ["a ret 48 base 1 ab 1 size 5000 state 1000 prot 4 type 40000", "b base_off 2000 size 3000", "unmap 1"]);
}

#[test]
fn query_fails_above_the_highest_address_and_a_short_buffer_stops_and_untracked_memory_is_free_since_decision_0123() {
    let l = run_m(
        "vmap-query-high",
        r#"
    uint64_t r;
    SETLE(0); r = vq(0x7FFFFFFF0000ull); printf("a %llu le %u\n", U(r), LE());
    SETLE(0); r = vq(0xFFFFFFFFFFFFF000ull); printf("b %llu le %u\n", U(r), LE());
"#,
    );
    // page: "If lpAddress specifies an address above the highest memory address accessible to the process, the function
    // fails with ERROR_INVALID_PARAMETER" (87); the highest address is the one GetSystemInfo reports
    assert_eq!(l, ["a 0 le 87", "b 0 le 87"]);
    // decision 0123 (PM's decision 4; old: ("free-space", "vq(0x10000);"), ("host-stack", "vq(A(stackish));") and ("after-region", "vq(base + 0x2000);") stopped the run, because the runtime tracked nothing but
    // VirtualAlloc regions and file views): an address that nothing tracks is MEM_FREE (Learn VirtualQuery: free pages have State MEM_FREE and AllocationBase, AllocationProtect, Protect and Type undefined, here 0;
    // S-vq-free). A host stack or heap block is not tracked (S-vq-heap), so it is MEM_FREE too. The size of the free region runs to the next tracked region (tests/wall_fr9.rs checks images and ranges).
    let l = run_m(
        "vmap-query-free",
        r#"
    uint64_t base = vm_alloc(0, 0x2000, 0x3000, 0x04), r; static char stackish[64];
    r = vq(0);                    printf("null: %llu state %x allocbase %llx\n", U(r), mbi.State, U((uint64_t)mbi.AllocationBase));
    r = vq(0x10000);              printf("low: %llu base %llx state %x allocbase %llx prot %x type %x size>0 %d\n", U(r), U((uint64_t)mbi.BaseAddress), mbi.State, U((uint64_t)mbi.AllocationBase), mbi.Protect, mbi.Type, mbi.RegionSize > 0);
    r = vq(A(stackish));          printf("host: %llu state %x allocbase %llx type %x\n", U(r), mbi.State, U((uint64_t)mbi.AllocationBase), mbi.Type);
    r = vq(base + 0x2000);        printf("after: %llu state %x base %llx\n", U(r), mbi.State, U((uint64_t)mbi.BaseAddress) - base);
    r = vq(base);                 printf("inside: %llu state %x\n", U(r), mbi.State);
    CALL(fle_imp_VirtualFree, base, 0, 0x8000);
    r = vq(base);                 printf("released: %llu state %x allocbase %llx\n", U(r), mbi.State, U((uint64_t)mbi.AllocationBase));
"#,
    );
    assert_eq!(l[0], "null: 48 state 10000 allocbase 0");
    let l = &l[1..];
    assert_eq!(l[0], "low: 48 base 10000 state 10000 allocbase 0 prot 0 type 0 size>0 1");
    assert_eq!(l[1], "host: 48 state 10000 allocbase 0 type 0");
    assert_eq!(l[2], "after: 48 state 10000 base 2000");
    assert_eq!(l[3], "inside: 48 state 1000");   // MEM_COMMIT 0x1000 (a VirtualAlloc of MEM_RESERVE | MEM_COMMIT 0x3000)
    assert_eq!(l[4], "released: 48 state 10000 allocbase 0");
    abort_m(
        "vmap-abort-query",
        "uint64_t base = vm_alloc(0, 0x2000, 0x3000, 0x04); static char stackish[64]; static MBI small;",
        &[
            ("buffer-short", "CALL(fle_imp_VirtualQuery, base, A(&small), 47);"),
            ("buffer-zero", "CALL(fle_imp_VirtualQuery, base, A(&small), 0);"),
            ("buffer-null", "CALL(fle_imp_VirtualQuery, base, 0, 48);"),
            // decision 0123 (old: ("null-address", "vq(0);") stopped): address 0 is untracked, so MEM_FREE (checked in the free-space run above: it is the same rule)
        ],
    );
}

#[test]
fn query_after_virtual_free_answers_free_on_the_released_address_since_decision_0123() {
    // decision 0123 (old: abort_m(...) with ("released", "vq(base);"): the released range was untracked and the query stopped): it is MEM_FREE now, like any untracked address (checked in
    // query_fails_above_the_highest_address_and_a_short_buffer_stops_and_untracked_memory_is_free_since_decision_0123, "released"); what still stops is a buffer that is too short or NULL
    let l = run_m(
        "vmap-query-freed",
        r#"
    uint64_t base = vm_alloc(0, 0x2000, 0x3000, 0x04), r;
    CALL(fle_imp_VirtualFree, base, 0, 0x8000);
    r = vq(base);
    printf("released: %llu state %x\n", U(r), mbi.State);
"#,
    );
    assert_eq!(l, ["released: 48 state 10000"]);
}

// ---- VirtualProtect ----

#[test]
fn protect_changes_committed_pages_and_returns_the_old_protection() {
    let l = run_m(
        "vmap-protect",
        r#"
    uint64_t base = vm_alloc(0, 0x4000, 0x3000, 0x04), r;
    uint32_t old = 0xAAAAAAAA;
    volatile uint8_t *p = (volatile uint8_t *)base;
    p[0] = 7; p[0x2000] = 9;
    /* READWRITE -> READONLY on pages 0..1: old protection is that of the first page */
    r = CALL(fle_imp_VirtualProtect, base, 0x2000, 0x02, A(&old)); printf("a %llu old %x\n", U(r), old);
    printf("read_ok %d\n", p[0] == 7);
    printf("write_faults %d\n", FAULTS(p[0] = 8));
    printf("value_kept %d\n", p[0] == 7);
    printf("other_pages_writable %d\n", !FAULTS(p[0x2000] = 10));
    /* the query shows the two runs */
    vq(base);          printf("q0 size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    vq(base + 0x2000); printf("q2 size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    /* back to READWRITE; the old protection is READONLY now */
    r = CALL(fle_imp_VirtualProtect, base, 0x2000, 0x04, A(&old)); printf("b %llu old %x\n", U(r), old);
    printf("write_ok %d value %d\n", !FAULTS(p[0] = 8), p[0]);
    /* NOACCESS: reads and writes fault */
    r = CALL(fle_imp_VirtualProtect, base + 0x1000, 0x1000, 0x01, A(&old)); printf("c %llu old %x\n", U(r), old);
    printf("noaccess_read_faults %d\n", FAULTS((void)*(volatile uint8_t *)(base + 0x1000)));
    vq(base + 0x1000); printf("qn prot %x size %llx\n", mbi.Protect, U(mbi.RegionSize));
    /* a byte range straddling a page boundary changes both pages */
    r = CALL(fle_imp_VirtualProtect, base + 0x2fff, 2, 0x02, A(&old)); printf("d %llu old %x\n", U(r), old);
    vq(base + 0x2000); printf("qd size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    /* only the first page's old protection is stored when the pages differ */
    r = CALL(fle_imp_VirtualProtect, base, 0x4000, 0x04, A(&old)); printf("e %llu old %x\n", U(r), old);
    vq(base); printf("qe size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
"#,
    );
    assert_eq!(
        l,
        [
            "a 1 old 4",
            "read_ok 1",
            "write_faults 1",
            "value_kept 1",
            "other_pages_writable 1",
            "q0 size 2000 prot 2",
            "q2 size 2000 prot 4",
            "b 1 old 2",
            "write_ok 1 value 8",
            "c 1 old 4",
            "noaccess_read_faults 1",
            "qn prot 1 size 1000",
            // page: "a 2-byte range straddling a page boundary causes the protection attributes of both pages to be changed"
            "d 1 old 4",
            "qd size 2000 prot 2",
            // old protection: that of the first page of the region
            "e 1 old 4",
            "qe size 4000 prot 4",
        ]
    );
}

#[test]
fn protect_executable_values_are_applied() {
    let l = run_m(
        "vmap-protect-exec",
        r#"
    uint64_t base = vm_alloc(0, 0x1000, 0x3000, 0x04), r;
    uint32_t old = 0;
    static const uint32_t prots[] = { 0x10, 0x20, 0x40, 0x02, 0x04, 0x01 };
    int i;
    for (i = 0; i < 6; i++) {
        r = CALL(fle_imp_VirtualProtect, base, 0x1000, prots[i], A(&old));
        vq(base);
        printf("%x ret %llu prot %x\n", prots[i], U(r), mbi.Protect);
    }
"#,
    );
    // page (Memory Protection Constants): PAGE_EXECUTE 0x10, EXECUTE_READ 0x20, EXECUTE_READWRITE 0x40, READONLY 0x2,
    // READWRITE 0x4, NOACCESS 0x1
    assert_eq!(
        l,
        ["10 ret 1 prot 10", "20 ret 1 prot 20", "40 ret 1 prot 40", "2 ret 1 prot 2", "4 ret 1 prot 4", "1 ret 1 prot 1"]
    );
}

#[test]
fn protect_fails_on_uncommitted_pages_and_leaves_the_protection_alone() {
    let l = run_m(
        "vmap-protect-fail",
        r#"
    uint64_t base = vm_alloc(0, 0x4000, 0x2000, 0x04), r;  /* reserved only */
    uint32_t old = 0x55555555;
    vm_alloc(base, 0x1000, 0x1000, 0x04);                   /* page 0 committed */
    /* the range includes a reserved page: fails, no page is changed (page) */
    SETLE(0); r = CALL(fle_imp_VirtualProtect, base, 0x2000, 0x02, A(&old)); printf("a %llu le %u old %x\n", U(r), LE(), old);
    vq(base); printf("q size %llx prot %x\n", U(mbi.RegionSize), mbi.Protect);
    /* only reserved pages */
    SETLE(0); r = CALL(fle_imp_VirtualProtect, base + 0x2000, 0x1000, 0x02, A(&old)); printf("b %llu le %u\n", U(r), LE());
    /* the range runs past the region's end */
    SETLE(0); r = CALL(fle_imp_VirtualProtect, base + 0x3000, 0x2000, 0x02, A(&old)); printf("c %llu le %u\n", U(r), LE());
    /* lpflOldProtect NULL fails (page) */
    SETLE(0); r = CALL(fle_imp_VirtualProtect, base, 0x1000, 0x02, 0); printf("d %llu le %u\n", U(r), LE());
    vq(base); printf("qd prot %x\n", mbi.Protect);
"#,
    );
    // page: failure returns zero with the error on GetLastError. The code is not named: 487 (ERROR_INVALID_ADDRESS, named
    // in System Error Codes) for a bad range and 87 for the NULL pointer are assumptions.
    assert_eq!(l, ["a 0 le 487 old 55555555", "q size 1000 prot 4", "b 0 le 487", "c 0 le 487", "d 0 le 87", "qd prot 4"]);
}

#[test]
fn protect_spanning_two_regions_fails() {
    let l = run_m(
        "vmap-protect-span",
        r#"
    /* two committed regions that touch each other, at addresses chosen here (both far from anything the host maps) */
    uint64_t a = vm_alloc(0x300000000000ull, 0x10000, 0x3000, 0x04);
    uint64_t b = vm_alloc(0x300000010000ull, 0x10000, 0x3000, 0x04), r;
    uint32_t old = 0;
    SETLE(0);
    printf("adjacent_setup %d %d\n", a == 0x300000000000ull, b == 0x300000010000ull);
    /* page: the pages "cannot span adjacent reserved regions that were allocated by separate calls" */
    r = CALL(fle_imp_VirtualProtect, a + 0xF000, 0x2000, 0x02, A(&old)); printf("span %llu le %u\n", U(r), LE());
    /* each region alone is fine */
    r = CALL(fle_imp_VirtualProtect, a + 0xF000, 0x1000, 0x02, A(&old)); printf("first %llu old %x\n", U(r), old);
    r = CALL(fle_imp_VirtualProtect, b, 0x1000, 0x02, A(&old)); printf("second %llu old %x\n", U(r), old);
"#,
    );
    assert_eq!(l, ["adjacent_setup 1 1", "span 0 le 487", "first 1 old 4", "second 1 old 4"]);
}

#[test]
fn protect_cases_outside_the_page_stop_the_run() {
    abort_m(
        "vmap-abort-protect",
        "uint64_t base = vm_alloc(0, 0x2000, 0x3000, 0x04); uint32_t old; static char stackish[64];\n    uint64_t h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x2000, 0), v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);",
        &[
            ("size-zero", "CALL(fle_imp_VirtualProtect, base, 0, 0x02, A(&old));"),
            ("guard-modifier", "CALL(fle_imp_VirtualProtect, base, 0x1000, 0x104, A(&old));"),
            ("nocache-modifier", "CALL(fle_imp_VirtualProtect, base, 0x1000, 0x204, A(&old));"),
            ("writecopy", "CALL(fle_imp_VirtualProtect, base, 0x1000, 0x08, A(&old));"),
            ("execute-writecopy", "CALL(fle_imp_VirtualProtect, base, 0x1000, 0x80, A(&old));"),
            ("zero-protection", "CALL(fle_imp_VirtualProtect, base, 0x1000, 0, A(&old));"),
            ("cfg-flag", "CALL(fle_imp_VirtualProtect, base, 0x1000, 0x40000020u, A(&old));"),
            ("two-flags", "CALL(fle_imp_VirtualProtect, base, 0x1000, 0x06, A(&old));"),
            ("untracked-memory", "CALL(fle_imp_VirtualProtect, A(stackish), 8, 0x02, A(&old));"),
            ("view", "CALL(fle_imp_VirtualProtect, v, 0x1000, 0x02, A(&old));"),
            ("size-overflow", "CALL(fle_imp_VirtualProtect, base, 0xFFFFFFFFFFFFFFF0ull, 0x02, A(&old));"),
            ("null-address", "CALL(fle_imp_VirtualProtect, 0, 0x1000, 0x02, A(&old));"),
        ],
    );
}

// ---- CreateFileMappingW, MapViewOfFile, UnmapViewOfFile ----

#[test]
fn anonymous_mapping_is_zero_filled_and_views_are_coherent() {
    let l = run_m(
        "vmap-anon",
        r#"
    uint64_t h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x3000, 0), v1, v2, r, k, sum = 0;
    printf("h %d mult4 %d\n", h != 0, (h % 4) == 0);
    v1 = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);
    printf("v1 %d aligned64k %d\n", v1 != 0, (v1 % 65536) == 0);
    for (k = 0; k < 0x3000; k++) sum += ((uint8_t *)v1)[k];
    printf("zero %llu\n", U(sum));
    ((uint8_t *)v1)[0] = 0x11; ((uint8_t *)v1)[0x2fff] = 0x22;
    v2 = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);
    printf("v2 distinct %d coherent %d %d\n", v2 != v1 && v2 != 0, ((uint8_t *)v2)[0] == 0x11, ((uint8_t *)v2)[0x2fff] == 0x22);
    ((uint8_t *)v2)[1] = 0x33;
    printf("back %d\n", ((uint8_t *)v1)[1] == 0x33);
    printf("counts h %llu views %llu objs %llu\n", U(fle_vmap_handle_count()), U(fle_vmap_view_count()), U(fle_vmap_object_count()));
    /* page: the object does not close until every view is unmapped and the handle is closed (any order) */
    r = CALL(fle_imp_CloseHandle, h); printf("close %llu\n", U(r));
    printf("view_still_ok %d\n", ((uint8_t *)v1)[0] == 0x11);
    printf("counts h %llu views %llu objs %llu\n", U(fle_vmap_handle_count()), U(fle_vmap_view_count()), U(fle_vmap_object_count()));
    r = CALL(fle_imp_UnmapViewOfFile, v1); printf("unmap1 %llu\n", U(r));
    r = CALL(fle_imp_UnmapViewOfFile, v2); printf("unmap2 %llu\n", U(r));
    printf("counts h %llu views %llu objs %llu\n", U(fle_vmap_handle_count()), U(fle_vmap_view_count()), U(fle_vmap_object_count()));
    printf("unmapped_faults %d\n", FAULTS((void)*(volatile uint8_t *)v1));
"#,
    );
    assert_eq!(
        l,
        [
            "h 1 mult4 1",
            // page (MapViewOfFile): the offset must be a multiple of the allocation granularity; the view's base is a
            // fresh range of address space (aligned to the granularity: a choice, as VirtualAlloc's)
            "v1 1 aligned64k 1",
            // page: "The initial contents of the pages in a file mapping object backed by the paging file are 0"
            "zero 0",
            // page: views of one object are coherent
            "v2 distinct 1 coherent 1 1",
            "back 1",
            "counts h 1 views 2 objs 1",
            "close 1",
            "view_still_ok 1",
            "counts h 0 views 2 objs 1",
            "unmap1 1",
            "unmap2 1",
            "counts h 0 views 0 objs 0",
            // page (UnmapViewOfFile): the range is invalidated
            "unmapped_faults 1",
        ]
    );
}

#[test]
fn unmap_in_either_order_and_the_object_lives_until_both_are_gone() {
    let l = run_m(
        "vmap-order",
        r#"
    uint64_t h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x1000, 0), v, r;
    v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);
    r = CALL(fle_imp_UnmapViewOfFile, v); printf("unmap %llu objs %llu\n", U(r), U(fle_vmap_object_count()));
    r = CALL(fle_imp_CloseHandle, h); printf("close %llu objs %llu\n", U(r), U(fle_vmap_object_count()));
    /* the closed handle is not usable any more */
    SETLE(0); v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0); printf("map_closed %llu le %u\n", U(v), LE());
"#,
    );
    assert_eq!(l, ["unmap 1 objs 1", "close 1 objs 0", "map_closed 0 le 6"]);
}

#[test]
fn view_access_types_follow_the_object_protection() {
    let l = run_m(
        "vmap-access",
        r#"
    uint64_t rw = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x1000, 0);
    uint64_t ro = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x02, 0, 0x1000, 0);
    uint64_t a, b, c, d, e, f;
    a = CALL(fle_imp_MapViewOfFile, rw, 0x2, 0, 0, 0);          /* FILE_MAP_WRITE */
    b = CALL(fle_imp_MapViewOfFile, rw, 0x4, 0, 0, 0);          /* FILE_MAP_READ */
    c = CALL(fle_imp_MapViewOfFile, rw, 0xF001F, 0, 0, 0);      /* FILE_MAP_ALL_ACCESS = write */
    d = CALL(fle_imp_MapViewOfFile, rw, 0x6, 0, 0, 0);          /* WRITE | READ = write */
    e = CALL(fle_imp_MapViewOfFile, ro, 0x4, 0, 0, 0);
    f = CALL(fle_imp_MapViewOfFile, rw, 0x1, 0, 0, 0);          /* FILE_MAP_COPY */
    printf("all_mapped %d\n", a && b && c && d && e && f);
    ((uint8_t *)a)[0] = 0x5A;
    printf("read_sees_write %d\n", ((uint8_t *)b)[0] == 0x5A);
    printf("read_view_write_faults %d\n", FAULTS(((volatile uint8_t *)b)[1] = 1));
    printf("all_access_writable %d\n", !FAULTS(((volatile uint8_t *)c)[2] = 1));
    printf("write_read_writable %d\n", !FAULTS(((volatile uint8_t *)d)[3] = 1));
    printf("ro_object_view_read_faults_on_write %d\n", FAULTS(((volatile uint8_t *)e)[0] = 1));
    /* copy-on-write: starts as the object's content, a write is private and not seen by other views */
    printf("copy_initial %d\n", ((uint8_t *)f)[0] == 0x5A);
    ((uint8_t *)f)[0] = 0x77;
    printf("copy_private %d original %d\n", ((uint8_t *)f)[0] == 0x77, ((uint8_t *)a)[0] == 0x5A);
    vq(f); printf("copy_prot %x type %x\n", mbi.Protect, mbi.Type);
    vq(b); printf("read_prot %x\n", mbi.Protect);
    vq(a); printf("write_prot %x\n", mbi.Protect);
"#,
    );
    assert_eq!(
        l,
        [
            "all_mapped 1",
            "read_sees_write 1",
            "read_view_write_faults 1",
            "all_access_writable 1",
            "write_read_writable 1",
            "ro_object_view_read_faults_on_write 1",
            "copy_initial 1",
            "copy_private 1 original 1",
            // page (Memory Protection Constants): a copy-on-write view is PAGE_WRITECOPY (0x8) until a write; reporting
            // it as such is an inference (the page does not say what VirtualQuery shows for FILE_MAP_COPY views)
            "copy_prot 8 type 40000",
            "read_prot 2",
            "write_prot 4",
        ]
    );
}

#[test]
fn view_offset_length_and_size_rules() {
    let l = run_m(
        "vmap-offset",
        r#"
    uint64_t h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x30000, 0), v, k;
    int ok;
    /* the object has 0x30000 bytes; mark each 64 KiB block of the object */
    v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);
    for (k = 0; k < 3; k++) ((uint8_t *)v)[k * 0x10000] = (uint8_t)(0xA0 + k);
    CALL(fle_imp_UnmapViewOfFile, v);
    /* an offset of one granularity: the view starts at block 1; 0 length means to the end of the mapping */
    v = CALL(fle_imp_MapViewOfFile, h, 0x4, 0, 0x10000, 0);
    printf("a %d first %x\n", v != 0, ((uint8_t *)v)[0]);
    vq(v); printf("size %llx\n", U(mbi.RegionSize));
    CALL(fle_imp_UnmapViewOfFile, v);
    /* a length rounds the view up to the page size */
    v = CALL(fle_imp_MapViewOfFile, h, 0x4, 0, 0x20000, 100);
    vq(v); printf("b first %x size %llx\n", ((uint8_t *)v)[0], U(mbi.RegionSize));
    CALL(fle_imp_UnmapViewOfFile, v);
    v = CALL(fle_imp_MapViewOfFile, h, 0x4, 0, 0, 0x1001);
    vq(v); printf("c size %llx\n", U(mbi.RegionSize));
    CALL(fle_imp_UnmapViewOfFile, v);
    /* exactly to the end */
    v = CALL(fle_imp_MapViewOfFile, h, 0x4, 0, 0x20000, 0x10000);
    printf("d %d\n", v != 0);
    CALL(fle_imp_UnmapViewOfFile, v);
    /* the high offset is part of the file offset: 0x1_0000_0000 is beyond this mapping */
"#,
    );
    assert_eq!(l, ["a 1 first a1", "size 20000", "b first a2 size 1000", "c size 2000", "d 1"]);
}

#[test]
fn unmap_fails_for_an_address_that_is_not_a_view_base() {
    let l = run_m(
        "vmap-unmap-bad",
        r#"
    uint64_t h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x2000, 0), v, r;
    v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);
    SETLE(0); r = CALL(fle_imp_UnmapViewOfFile, v + 0x1000); printf("inside %llu le %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_UnmapViewOfFile, 0); printf("null %llu le %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_UnmapViewOfFile, 0x12340000); printf("stray %llu le %u\n", U(r), LE());
    r = CALL(fle_imp_UnmapViewOfFile, v); printf("ok %llu\n", U(r));
    SETLE(0); r = CALL(fle_imp_UnmapViewOfFile, v); printf("twice %llu le %u\n", U(r), LE());
"#,
    );
    // page: the value "must be identical to the value returned by a previous call"; failure returns zero. The code is not
    // named: 487 is an assumption.
    assert_eq!(l, ["inside 0 le 487", "null 0 le 487", "stray 0 le 487", "ok 1", "twice 0 le 487"]);
}

#[test]
fn mapping_of_a_virtual_file_is_a_snapshot_of_its_content() {
    let name = cw("C:\\map.bin");
    let l = run_m(
        "vmap-file",
        &format!(
            r#"
    static uint8_t data[70000];
    uint32_t n = 0;
    uint64_t f, h, v, r, k;
    int ok = 1;
    for (k = 0; k < 70000; k++) data[k] = (uint8_t)(k * 7 + 3);
    f = CALL(fle_imp_CreateFileW, {name}, 0xC0000000u, 1, 0, 2, 0x80, 0);
    r = CALL(fle_imp_WriteFile, f, A(data), 70000, A(&n), 0);
    printf("written %llu %u\n", U(r), n);
    /* PAGE_READONLY needs GENERIC_READ (page); maximum size 0 = the file's size */
    h = CALL(fle_imp_CreateFileMappingW, f, 0, 0x02, 0, 0, 0);
    printf("h %d\n", h != 0);
    v = CALL(fle_imp_MapViewOfFile, h, 0x4, 0, 0, 0);
    vq(v); printf("view %d size %llx\n", v != 0, U(mbi.RegionSize));
    for (k = 0; k < 70000; k++) if (((uint8_t *)v)[k] != data[k]) ok = 0;
    printf("content %d\n", ok);
    printf("tail_zero %d\n", ((uint8_t *)v)[70000] == 0 && ((uint8_t *)v)[0x11fff] == 0);
    /* a size below the file length keeps that size */
    r = CALL(fle_imp_CreateFileMappingW, f, 0, 0x02, 0, 0x1000, 0);
    v = CALL(fle_imp_MapViewOfFile, r, 0x4, 0, 0, 0);
    vq(v); printf("small size %llx\n", U(mbi.RegionSize));
    /* the file handle can be closed; the mapping keeps its content */
    CALL(fle_imp_CloseHandle, f);
    printf("after_close %d\n", ((uint8_t *)v)[5] == data[5]);
"#,
            name = name,
        ),
    );
    assert_eq!(l, ["written 1 70000", "h 1", "view 1 size 12000", "content 1", "tail_zero 1", "small size 1000", "after_close 1"]);
}

#[test]
fn mapping_failures_and_invalid_handles() {
    let name = cw("C:\\empty.bin");
    let l = run_m(
        "vmap-fail",
        &format!(
            r#"
    uint64_t f, r;
    f = CALL(fle_imp_CreateFileW, {name}, 0xC0000000u, 1, 0, 2, 0x80, 0);
    /* page: an attempt to map a file with a length of 0 fails with ERROR_FILE_INVALID (1006) */
    SETLE(0); r = CALL(fle_imp_CreateFileMappingW, f, 0, 0x02, 0, 0, 0); printf("empty %llu le %u\n", U(r), LE());
    /* NULL, closed, INVALID_HANDLE_VALUE as a mapping handle and a find-style value */
    SETLE(0); r = CALL(fle_imp_MapViewOfFile, 0, 0x4, 0, 0, 0); printf("map_null %llu le %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_MapViewOfFile, INVH, 0x4, 0, 0, 0); printf("map_inv %llu le %u\n", U(r), LE());
    CALL(fle_imp_CloseHandle, f);
    SETLE(0); r = CALL(fle_imp_CreateFileMappingW, f, 0, 0x02, 0, 0, 0); printf("closed_file %llu le %u\n", U(r), LE());
    SETLE(0); r = CALL(fle_imp_CreateFileMappingW, 0, 0, 0x02, 0, 0, 0); printf("null_file %llu le %u\n", U(r), LE());
"#,
            name = name,
        ),
    );
    // CreateFileMapping: failure returns NULL; the codes other than 1006 are not named (6 is an assumption)
    assert_eq!(
        l,
        [
            "empty 0 le 1006",
            "map_null 0 le 6",
            "map_inv 0 le 6",
            "closed_file 0 le 6",
            "null_file 0 le 6",
        ]
    );
}

#[test]
fn sec_commit_is_accepted_and_the_large_size_high_part_counts() {
    let l = run_m(
        "vmap-sec",
        r#"
    uint64_t h, v, r;
    h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04 | 0x08000000u, 0, 0x1000, 0); printf("sec_commit %d\n", h != 0);
    v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0); vq(v); printf("size %llx\n", U(mbi.RegionSize));
    /* sizeHigh is the high-order DWORD of the size: 1 means 4 GiB, above this runtime's limit */
"#,
    );
    assert_eq!(l, ["sec_commit 1", "size 1000"]);
}

#[test]
fn duplicate_handle_of_a_mapping_handle_shares_the_object() {
    let l = run_m(
        "vmap-dup",
        r#"
    uint64_t pp = CALL(fle_imp_GetCurrentProcess, 0), h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x1000, 0), d = 0, v, w, r;
    r = CALL(fle_imp_DuplicateHandle, pp, h, pp, A(&d), 0, 0, 2); printf("dup %llu distinct %d\n", U(r), d != 0 && d != h);
    printf("handles %llu objs %llu\n", U(fle_vmap_handle_count()), U(fle_vmap_object_count()));
    v = CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0);
    w = CALL(fle_imp_MapViewOfFile, d, 0x2, 0, 0, 0);
    ((uint8_t *)v)[9] = 0x42;
    printf("same_object %d\n", ((uint8_t *)w)[9] == 0x42);
    /* closing one handle leaves the other usable */
    CALL(fle_imp_CloseHandle, h);
    printf("handles %llu objs %llu\n", U(fle_vmap_handle_count()), U(fle_vmap_object_count()));
    r = CALL(fle_imp_MapViewOfFile, d, 0x4, 0, 0, 0); printf("map_dup %d\n", r != 0);
    /* DUPLICATE_CLOSE_SOURCE closes the source mapping handle */
    r = CALL(fle_imp_DuplicateHandle, pp, d, pp, A(&h), 0, 0, 3); printf("move %llu\n", U(r));
    printf("handles %llu\n", U(fle_vmap_handle_count()));
    SETLE(0); r = CALL(fle_imp_CloseHandle, d); printf("old_closed %llu le %u\n", U(r), LE());
"#,
    );
    assert_eq!(
        l,
        [
            "dup 1 distinct 1",
            "handles 2 objs 1",
            "same_object 1",
            "handles 1 objs 1",
            "map_dup 1",
            "move 1",
            "handles 1",
            "old_closed 0 le 6",
        ]
    );
}

#[test]
fn mapping_cases_outside_the_pages_stop_the_run() {
    let file = cw("C:\\abortmap.bin");
    abort_m(
        "vmap-abort-map",
        &format!(
            "static uint8_t buf[300000]; uint32_t n; uint64_t f = CALL(fle_imp_CreateFileW, {file}, 0xC0000000u, 1, 0, 2, 0x80, 0), fw = CALL(fle_imp_CreateFileW, {w}, 0x40000000u, 1, 0, 2, 0x80, 0);\n    CALL(fle_imp_WriteFile, f, A(buf), 1000, A(&n), 0);\n    uint64_t h = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x20000, 0), ro = CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x02, 0, 0x20000, 0);",
            file = file,
            w = cw("C:\\writeonly.bin"),
        ),
        &[
            ("create-attrs", "CALL(fle_imp_CreateFileMappingW, INVH, 8, 0x04, 0, 0x1000, 0);"),
            ("create-named", &format!("CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0x1000, {});", cw("Local\\x"))),
            ("create-writecopy", "CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x08, 0, 0x1000, 0);"),
            ("create-execute-read", "CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x20, 0, 0x1000, 0);"),
            ("create-noaccess", "CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x01, 0, 0x1000, 0);"),
            ("create-sec-reserve", "CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04 | 0x04000000u, 0, 0x1000, 0);"),
            ("create-sec-image", "CALL(fle_imp_CreateFileMappingW, f, 0, 0x02 | 0x01000000u, 0, 0, 0);"),
            ("create-large-pages", "CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04 | 0x80000000u, 0, 0x1000, 0);"),
            ("create-anon-size-zero", "CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 0, 0, 0);"),
            ("create-over-limit", "CALL(fle_imp_CreateFileMappingW, INVH, 0, 0x04, 1, 0, 0);"),
            ("create-readwrite-over-file", "CALL(fle_imp_CreateFileMappingW, f, 0, 0x04, 0, 0, 0);"),
            ("create-size-above-file", "CALL(fle_imp_CreateFileMappingW, f, 0, 0x02, 0, 5000, 0);"),
            ("create-file-without-read", "CALL(fle_imp_CreateFileMappingW, fw, 0, 0x02, 0, 0, 0);"),
            ("create-foreign-handle", "CALL(fle_imp_CreateFileMappingW, 0x1234, 0, 0x04, 0, 0x1000, 0);"),
            ("map-foreign-handle", "CALL(fle_imp_MapViewOfFile, 0x1234, 0x2, 0, 0, 0);"),
            ("map-vfs-file-handle", "CALL(fle_imp_MapViewOfFile, f, 0x2, 0, 0, 0);"),
            ("map-write-on-readonly-object", "CALL(fle_imp_MapViewOfFile, ro, 0x2, 0, 0, 0);"),
            ("map-all-access-on-readonly-object", "CALL(fle_imp_MapViewOfFile, ro, 0xF001F, 0, 0, 0);"),
            ("map-execute", "CALL(fle_imp_MapViewOfFile, h, 0x24, 0, 0, 0);"),
            ("map-access-zero", "CALL(fle_imp_MapViewOfFile, h, 0, 0, 0, 0);"),
            ("map-copy-with-write", "CALL(fle_imp_MapViewOfFile, h, 0x3, 0, 0, 0);"),
            ("map-large-pages", "CALL(fle_imp_MapViewOfFile, h, 0x20000002u, 0, 0, 0);"),
            ("map-offset-unaligned", "CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0x1000, 0);"),
            ("map-offset-beyond", "CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0x20000, 0);"),
            ("map-offset-high", "CALL(fle_imp_MapViewOfFile, h, 0x2, 1, 0, 0);"),
            ("map-length-beyond", "CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0, 0x20001);"),
            ("map-length-with-offset-beyond", "CALL(fle_imp_MapViewOfFile, h, 0x2, 0, 0x10000, 0x10001);"),
            ("close-never-issued-in-region", "CALL(fle_imp_CloseHandle, FLE_VMAP_HANDLE_BASE + 4 * 900);"),
        ],
    );
}

#[test]
fn a_second_thread_stops_the_run() {
    let o = run_src(
        "vmap-thread",
        &format!("{MBI}\nstatic void *th(void *x) {{ (void)x; CALL(fle_imp_UnmapViewOfFile, 0); printf(\"returned-from-thread\\n\"); return 0; }}"),
        "    pthread_t t;\n    CALL(fle_imp_UnmapViewOfFile, 0);\n    pthread_create(&t, 0, th, 0);\n    pthread_join(t, 0);\n    printf(\"returned\\n\");",
    );
    assert!(!o.ok && !o.stdout.contains("returned"));
    assert_eq!(o.signal, Some(6));
    assert!(o.stderr.contains("fle: UnmapViewOfFile") && o.stderr.contains("second thread"), "{}", o.stderr);
}

#[test]
fn source_never_opens_host_files_or_reads_host_memory_maps() {
    let code = strip_comments(&std::fs::read_to_string(src_dir().join("fle_vmap.c")).unwrap());
    for bad in [
        "getpid", "getenv", "environ", "fopen", "open(", "shm_open", "/proc", "/etc", "/home", "/dev", "/tmp", "mkstemp", "tmpfile", "<sys/stat.h>",
        "exit(", "_exit", "O_RDWR", "O_TMPFILE", "msync", "remap_file_pages",
    ] {
        assert!(!code.contains(bad), "fle_vmap.c must not use {bad}");
    }
    // the only host object it makes is an anonymous memory file
    assert!(code.contains("memfd_create"));
}
