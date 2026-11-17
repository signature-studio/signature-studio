//! The atomic read-modify-write helpers of runtime/fle_rmw.h under real concurrency (mt4 stage A): four host threads hammer one 8-byte and one misaligned word with the same loops that fe-lift emits (a compare-and-swap loop for a LOCK-prefixed
//! ADD, fle_xadd, fle_xchg, fle_cas, the interlock path for a misaligned operand). The totals must be exact; a non-atomic read-then-write loses updates and fails.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

const SRC: &str = r#"
#include "fle_runtime.h"
#include <pthread.h>
#define N 400000
static uint8_t M[64] __attribute__((aligned(16)));
static void *worker(void *arg)
{
    long id = (long)arg;
    for (int i = 0; i < N; i++) {
        uint64_t o, t;
        /* lock add dword [M+0], 1 : the loop of fe-lift */
        do { o = fle_ld((uint64_t)(uintptr_t)(M + 0), 4); t = (o + 1) & 0xffffffffu; } while (fle_cas((uint64_t)(uintptr_t)(M + 0), 4, o, t) != o);
        /* lock xadd qword [M+8], 3 */
        (void)fle_xadd((uint64_t)(uintptr_t)(M + 8), 8, 3);
        /* lock add byte [M+16], 1 */
        do { o = fle_ld((uint64_t)(uintptr_t)(M + 16), 1); t = (o + 1) & 0xff; } while (fle_cas((uint64_t)(uintptr_t)(M + 16), 1, o, t) != o);
        /* misaligned lock add dword [M+25], 1 (the interlock path) */
        do { o = fle_ld((uint64_t)(uintptr_t)(M + 25), 4); t = (o + 1) & 0xffffffffu; } while (fle_cas((uint64_t)(uintptr_t)(M + 25), 4, o, t) != o);
        /* misaligned xadd qword [M+35], 1 */
        (void)fle_xadd((uint64_t)(uintptr_t)(M + 35), 8, 1);
        /* a spin lock made of xchg (the memory-manager pattern): the guarded counter must not lose updates */
        while (fle_xchg((uint64_t)(uintptr_t)(M + 48), 1, 1) != 0) {}
        { uint64_t c = fle_ld((uint64_t)(uintptr_t)(M + 52), 4); fle_st((uint64_t)(uintptr_t)(M + 52), 4, c + 1); }
        (void)fle_xchg((uint64_t)(uintptr_t)(M + 48), 1, 0);
    }
    (void)id;
    return 0;
}
int main(void)
{
    pthread_t th[4];
    for (long k = 0; k < 4; k++) pthread_create(&th[k], 0, worker, (void *)k);
    for (int k = 0; k < 4; k++) pthread_join(th[k], 0);
    printf("%llu %llu %llu %llu %llu %llu\n", (unsigned long long)fle_ld((uint64_t)(uintptr_t)(M + 0), 4), (unsigned long long)fle_ld((uint64_t)(uintptr_t)(M + 8), 8),
           (unsigned long long)fle_ld((uint64_t)(uintptr_t)(M + 16), 1), (unsigned long long)fle_ld((uint64_t)(uintptr_t)(M + 25), 4),
           (unsigned long long)fle_ld((uint64_t)(uintptr_t)(M + 35), 8), (unsigned long long)fle_ld((uint64_t)(uintptr_t)(M + 52), 4));
    return 0;
}
"#;

#[test]
fn rmw_helpers_are_atomic_across_threads() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../out-more/test");
    let rt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
    std::fs::create_dir_all(dir.join("tmp")).unwrap();
    let uname = etxt_common::uniq("lock_rmw_threads");
    let c = dir.join(format!("{uname}.c"));
    let exe = dir.join(&uname);
    std::fs::write(&c, SRC).unwrap();
    let cc = Command::new("clang").args(["-O1", "-std=gnu11", "-w", "-fPIE", "-pthread", "-I"]).arg(&rt).arg("-o").arg(&exe).arg(&c).env("TMPDIR", dir.join("tmp")).output().expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run failed");
    assert!(run.status.success(), "C program failed");
    let out = String::from_utf8(run.stdout).unwrap();
    let n: u64 = 4 * 400_000;
    // the byte counter wraps modulo 256
    assert_eq!(out.trim(), format!("{} {} {} {} {} {}", n, 3 * n, n % 256, n, n, n), "totals (a lost update shows as a smaller number)");
}
