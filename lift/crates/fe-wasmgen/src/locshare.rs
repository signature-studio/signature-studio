//! Decision 0264 (mt3z): the locals that hold the saved operands of a flag definition (`Def`, `need_save`) are shared by definitions whose live ranges do not meet.
//!
//! Without this every flag definition that a reader builds its condition from owns two locals for the whole function. A large function of the real DLLs has
//! 300-430 such definitions (Blend2D f_4776f60: 426 of its 892 locals pairs, dsp f_20a3bd0: 294 of 626), and the size rule of decision 0185 is the product
//! `wasm bytes x locals`, so the locals are half of the reason that such a function stays on the C path.
//!
//! The range of a definition `d` is, at block granularity, the set of blocks on the paths from `d` to its readers that do not pass `d` again (the definition rewrites
//! the locals): the backward closure of the blocks of the readers (the instructions whose `FlagInfo::reader` entry names `Src::Def(d)` for one flag) that stops at the
//! block of `d`, plus that block. A reader that comes before `d` in its own block (or is `d` itself, the `adc` of a loop) is a use that enters the block from its
//! predecessors. The range is a superset of the points where the saved operands are needed (whole blocks; a `weak` definition lies inside the range of the definition
//! it does not kill). Two definitions share a pair of locals only if their ranges have no block in common and the pair has the same type. The lowering keeps writing
//! the pair at the definition and reading it at the readers, as before; only which locals it names changes.
//!
//! `LOCSHARE_OFF` is a test switch (like `lower::MX_FORCE_SLOW`): the tests lower a function with and without the sharing and run both.

use crate::flags::{FlagInfo, Src};
use crate::ir::Fun;
use std::collections::{HashMap, VecDeque};

pub static LOCSHARE_OFF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub const NO_SLOT: u32 = u32::MAX;

fn words(n: usize) -> usize {
    (n + 63) / 64
}

fn test(b: &[u64], i: usize) -> bool {
    b[i / 64] >> (i % 64) & 1 != 0
}

fn set(b: &mut [u64], i: usize) {
    b[i / 64] |= 1 << (i % 64);
}

/// The slot of each definition (indexed by definition id; `NO_SLOT` for a definition that nothing reads and that is not saved) and the number of slots.
/// `key` classifies the type of the pair of locals: definitions with different keys never share.
pub fn assign<K: Eq>(f: &Fun, preds: &[Vec<usize>], reachable: &[bool], fl: &FlagInfo, key: impl Fn(u32) -> K) -> (Vec<u32>, usize) {
    let n = f.blocks.len();
    let w = words(n);
    let mut block_of: HashMap<u64, usize> = HashMap::new();
    for (b, blk) in f.blocks.iter().enumerate() {
        for &va in &blk.insns {
            block_of.insert(va, b);
        }
    }
    let nd = fl.defs.len();
    let mut readers: Vec<Vec<(usize, u64)>> = vec![Vec::new(); nd];
    for (va, srcs) in &fl.reader {
        let Some(&rb) = block_of.get(va) else { continue };
        for s in srcs.iter().flatten() {
            if let Src::Def(d) = s {
                if (*d as usize) < nd && !readers[*d as usize].contains(&(rb, *va)) {
                    readers[*d as usize].push((rb, *va));
                }
            }
        }
    }
    let pos = |b: usize, va: u64| f.blocks[b].insns.iter().position(|&x| x == va);
    let mut slot_of = vec![NO_SLOT; nd];
    // the definitions that can own locals, in a fixed order (block, then address)
    let mut order: Vec<(usize, u64, u32)> = Vec::new();
    for (id, d) in fl.defs.iter().enumerate() {
        let Some(&db) = block_of.get(&d.va) else { continue };
        if !reachable[db] || !(d.need_save || !readers[id].is_empty()) {
            continue;
        }
        order.push((db, d.va, id as u32));
    }
    order.sort();
    struct Slot<K> {
        key: K,
        live: Vec<u64>,
    }
    let mut slots: Vec<Slot<K>> = Vec::new();
    for &(db, _, id) in &order {
        // the blocks where the saved operands of `d` can be needed: the backward closure of the readers that stops at the block of the definition (the definition
        // rewrites the locals), where a reader that comes before the definition in its own block (or is the definition itself, an `adc` of a loop) is a use that
        // enters the block from its predecessors; the block of the definition always belongs to the range
        let mut live = vec![0u64; w];
        set(&mut live, db);
        let mut q: VecDeque<usize> = VecDeque::new();
        let dva = fl.defs[id as usize].va;
        for &(rb, rva) in &readers[id as usize] {
            if rb != db {
                if !test(&live, rb) {
                    set(&mut live, rb);
                    q.push_back(rb);
                }
            } else if let (Some(pr), Some(pd)) = (pos(rb, rva), pos(db, dva)) {
                if pr <= pd {
                    q.push_back(db);
                }
            } else {
                q.push_back(db);
            }
        }
        while let Some(b) = q.pop_front() {
            for &p in &preds[b] {
                if !test(&live, p) {
                    set(&mut live, p);
                    if p != db {
                        q.push_back(p);
                    }
                }
            }
        }
        let k = key(id);
        let mut found = None;
        for (si, s) in slots.iter().enumerate() {
            if s.key == k && s.live.iter().zip(&live).all(|(a, b)| a & b == 0) {
                found = Some(si);
                break;
            }
        }
        let si = match found {
            Some(si) => si,
            None => {
                slots.push(Slot { key: k, live: vec![0u64; w] });
                slots.len() - 1
            }
        };
        for (a, b) in slots[si].live.iter_mut().zip(&live) {
            *a |= *b;
        }
        slot_of[id as usize] = si as u32;
    }
    (slot_of, slots.len())
}

/// Test switch: `lower_fn` runs `verify` on every assignment and panics on a violation (the tests set it; the census tool sets it from the environment).
pub static LOCSHARE_VERIFY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// An independent check of an assignment, by walking the instructions instead of the blocks' ranges: for every reader of a definition `d` that owns a shared slot, every path
/// backwards from the reader to the instruction of `d` meets no instruction that writes the same slot (the instruction of another definition with the slot of `d`), and the
/// reader is not itself such an instruction. A path that reaches the entry without meeting `d` ends the walk (the flag analysis has no such reader).
pub fn verify(f: &Fun, preds: &[Vec<usize>], fl: &FlagInfo, slot_of: &[u32]) -> Result<(), String> {
    let mut writers: HashMap<u64, Vec<u32>> = HashMap::new();
    for (id, d) in fl.defs.iter().enumerate() {
        if slot_of.get(id).copied().unwrap_or(NO_SLOT) != NO_SLOT {
            writers.entry(d.va).or_default().push(id as u32);
        }
    }
    let mut block_of: HashMap<u64, (usize, usize)> = HashMap::new();
    for (b, blk) in f.blocks.iter().enumerate() {
        for (k, &va) in blk.insns.iter().enumerate() {
            block_of.insert(va, (b, k));
        }
    }
    for (&rva, srcs) in &fl.reader {
        let Some(&(rb, rk)) = block_of.get(&rva) else { continue };
        for s in srcs.iter().flatten() {
            let Src::Def(d) = *s else { continue };
            let slot = slot_of.get(d as usize).copied().unwrap_or(NO_SLOT);
            if slot == NO_SLOT {
                continue;
            }
            let dva = fl.defs[d as usize].va;
            let clash = |va: u64| -> Option<u32> {
                writers.get(&va).and_then(|w| w.iter().copied().find(|&e| e != d && slot_of[e as usize] == slot))
            };
            if let Some(e) = clash(rva) {
                return Err(format!("reader {rva:#x} of def {d} (slot {slot}) is also the definition {e} of the same slot"));
            }
            let mut entered: Vec<bool> = vec![false; f.blocks.len()];
            let mut work: Vec<(usize, usize)> = vec![(rb, rk)];
            while let Some((b, from)) = work.pop() {
                let mut ended = false;
                for k in (0..from).rev() {
                    let va = f.blocks[b].insns[k];
                    if va == dva {
                        ended = true;
                        break;
                    }
                    if let Some(e) = clash(va) {
                        return Err(format!("def {e} at {va:#x} writes slot {slot} between def {d} ({dva:#x}) and its reader {rva:#x}"));
                    }
                }
                if !ended {
                    for &p in &preds[b] {
                        if !entered[p] {
                            entered[p] = true;
                            work.push((p, f.blocks[p].insns.len()));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
