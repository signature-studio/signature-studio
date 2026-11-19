//! Shared helper of the tests that compile a C program and run it (docs/investigations/test-etxtbsy.md).
//!
//! The tests write their sources, objects and executables under out-*/test with fixed names. Two tests of one binary
//! (threads) and two test processes (a second `cargo test` on the same tree) then wrote and ran the same file at the
//! same time: exec of a file that another clang is still writing fails with ETXTBSY ("Text file busy"), and a source
//! that is rewritten while another clang reads it gives a failed compile. `uniq` makes every file name unique per
//! process and per call.
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};

/// `name` followed by the process id and a counter of this process: `name-<pid>-<n>`.
pub fn uniq(name: &str) -> String {
    static N: AtomicUsize = AtomicUsize::new(0);
    format!("{name}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed))
}

/// `name` followed by the process id: `name-<pid>`. For files that one process builds once and shares between its tests.
pub fn per_process(name: &str) -> String {
    format!("{name}-{}", std::process::id())
}
