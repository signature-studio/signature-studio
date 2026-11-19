//! Dominators, loops and the merge nodes that decide where a wasm `block` ends (the structuring of Ramsey, "Beyond Relooper", 2022).
//! An irreducible function is reported and left to the caller (the production plan is a dispatch loop for those functions only).

use crate::ir::{Block, Term};

pub struct Cfg {
    pub n: usize,
    pub preds: Vec<Vec<usize>>,
    /// Reverse postorder of the reachable blocks, and the position of each block in it (`usize::MAX` = unreachable).
    pub rpo: Vec<usize>,
    pub num: Vec<usize>,
    pub idom: Vec<usize>,
    pub children: Vec<Vec<usize>>,
    pub is_header: Vec<bool>,
    pub is_merge: Vec<bool>,
    /// False when a retreating edge enters a loop that does not dominate its tail (the function is lowered with the dispatch loop).
    pub reducible: bool,
}

impl Cfg {
    pub fn dominates(&self, a: usize, mut b: usize) -> bool {
        loop {
            if a == b {
                return true;
            }
            if b == 0 {
                return false;
            }
            let d = self.idom[b];
            if d == b {
                return false;
            }
            b = d;
        }
    }
}

pub fn analyze(blocks: &[Block]) -> Result<Cfg, String> {
    let n = blocks.len();
    let mut preds = vec![Vec::new(); n];
    for (b, bl) in blocks.iter().enumerate() {
        for &s in &bl.succs {
            preds[s].push(b);
        }
    }
    // iterative DFS for postorder
    let mut post = Vec::with_capacity(n);
    let mut state = vec![0u8; n];
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    state[0] = 1;
    while let Some(&mut (b, ref mut k)) = stack.last_mut() {
        if *k < blocks[b].succs.len() {
            let s = blocks[b].succs[*k];
            *k += 1;
            if state[s] == 0 {
                state[s] = 1;
                stack.push((s, 0));
            }
        } else {
            post.push(b);
            stack.pop();
        }
    }
    let rpo: Vec<usize> = post.iter().rev().copied().collect();
    let mut num = vec![usize::MAX; n];
    for (k, &b) in rpo.iter().enumerate() {
        num[b] = k;
    }
    // Cooper-Harvey-Kennedy
    let mut idom = vec![usize::MAX; n];
    idom[0] = 0;
    let mut changed = true;
    while changed {
        changed = false;
        for &b in rpo.iter().skip(1) {
            let mut new: Option<usize> = None;
            for &p in &preds[b] {
                if num[p] == usize::MAX || idom[p] == usize::MAX {
                    continue;
                }
                new = Some(match new {
                    None => p,
                    Some(mut x) => {
                        let mut y = p;
                        while x != y {
                            while num[x] > num[y] {
                                x = idom[x];
                            }
                            while num[y] > num[x] {
                                y = idom[y];
                            }
                        }
                        x
                    }
                });
            }
            if let Some(nd) = new {
                if idom[b] != nd {
                    idom[b] = nd;
                    changed = true;
                }
            }
        }
    }
    let mut children = vec![Vec::new(); n];
    for &b in rpo.iter().skip(1) {
        children[idom[b]].push(b);
    }
    let mut cfg = Cfg { n, preds, rpo, num, idom, children, is_header: vec![false; n], is_merge: vec![false; n], reducible: true };
    for &u in &cfg.rpo.clone() {
        for &v in &blocks[u].succs {
            if cfg.num[v] <= cfg.num[u] {
                if !cfg.dominates(v, u) {
                    cfg.reducible = false;
                } else {
                    cfg.is_header[v] = true;
                }
            }
        }
    }
    let mut forced = vec![false; n];
    for bl in blocks {
        if let Term::Switch { cases, .. } = &bl.term {
            for &(_, t) in cases {
                forced[t] = true;
            }
        }
    }
    for y in 0..n {
        if cfg.num[y] == usize::MAX {
            continue;
        }
        let mut fwd = 0;
        for &p in &cfg.preds[y] {
            if cfg.num[p] != usize::MAX && cfg.num[p] < cfg.num[y] {
                fwd += 1;
            }
        }
        cfg.is_merge[y] = fwd >= 2 || (forced[y] && y != 0);
    }
    Ok(cfg)
}
