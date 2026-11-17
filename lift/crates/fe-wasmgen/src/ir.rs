//! Function bodies and basic blocks over the fe-decode closure. The traversal is the one of fe-lift (a function is every node reachable
//! from its entry without passing another entry).

use fe_decode::{Analysis, Decoded, Resolved, Text};
use iced_x86::{ConditionCode, FlowControl, Instruction, Mnemonic, OpKind};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub enum Node {
    Insn(Instruction),
    Bad,
}

pub fn is_near(i: &Instruction) -> bool {
    matches!(i.op_kind(0), OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64)
}

pub fn traverse(
    f: u64,
    text: &Text<'_>,
    entries: &BTreeSet<u64>,
    resolved: &BTreeMap<u64, Resolved>,
    tabs: &BTreeMap<u64, fe_decode::JumpTable>,
) -> BTreeMap<u64, Node> {
    let boundary = |s: u64| s != f && entries.contains(&s);
    let mut nodes: BTreeMap<u64, Node> = BTreeMap::new();
    let mut work = vec![f];
    while let Some(start) = work.pop() {
        let mut va = start;
        loop {
            if nodes.contains_key(&va) || boundary(va) {
                break;
            }
            let i = match text.decode(va) {
                Decoded::Ok(i) => i,
                _ => {
                    nodes.insert(va, Node::Bad);
                    break;
                }
            };
            let next = va + i.len() as u64;
            let near = is_near(&i);
            let target = i.near_branch_target();
            match i.flow_control() {
                FlowControl::Next | FlowControl::XbeginXabortXend | FlowControl::IndirectCall => {
                    // decision 0169: fe-lift lifts an indirect call that fe-decode resolved to the entry of a function (a slot of the image, or a register holding a constant) as a
                    // direct call of that function, whatever the slot holds at run time; the backend does the same, so that the two paths call the same function
                    let direct = if i.flow_control() == FlowControl::IndirectCall {
                        match resolved.get(&va) {
                            Some(Resolved::Code { target, .. }) if entries.contains(target) => {
                                let mut c = Instruction::default();
                                c.set_code(iced_x86::Code::Call_rel32_64);
                                c.set_op0_kind(OpKind::NearBranch64);
                                c.set_near_branch64(*target);
                                c.set_ip(va);
                                c.set_len(i.len());
                                Some(c)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    nodes.insert(va, Node::Insn(direct.unwrap_or(i)));
                    va = next;
                }
                FlowControl::Call => {
                    nodes.insert(va, Node::Insn(i));
                    if !near {
                        break;
                    }
                    va = next;
                }
                FlowControl::UnconditionalBranch => {
                    nodes.insert(va, Node::Insn(i));
                    if near && !boundary(target) {
                        work.push(target);
                    }
                    break;
                }
                FlowControl::ConditionalBranch => {
                    nodes.insert(va, Node::Insn(i));
                    if near && !boundary(target) {
                        work.push(target);
                    }
                    if !boundary(next) {
                        work.push(next);
                    }
                    break;
                }
                FlowControl::IndirectBranch => {
                    nodes.insert(va, Node::Insn(i));
                    if let Some(Resolved::Code { target, .. }) = resolved.get(&va) {
                        if !boundary(*target) {
                            work.push(*target);
                        }
                    }
                    if let Some(t) = tabs.get(&va) {
                        for &(_, tg) in &t.cases {
                            if !boundary(tg) {
                                work.push(tg);
                            }
                        }
                    }
                    break;
                }
                FlowControl::Return | FlowControl::Interrupt | FlowControl::Exception => {
                    nodes.insert(va, Node::Insn(i));
                    break;
                }
            }
        }
    }
    nodes
}

/// Successor VAs of one node inside a function body (a call is not a successor edge).
fn node_succs(va: u64, n: &Node, a: &Analysis) -> Vec<u64> {
    let Node::Insn(i) = n else { return vec![] };
    let next = va + i.len() as u64;
    match i.flow_control() {
        FlowControl::Next | FlowControl::XbeginXabortXend | FlowControl::IndirectCall => vec![next],
        FlowControl::Call => {
            if is_near(i) {
                vec![next]
            } else {
                vec![]
            }
        }
        FlowControl::UnconditionalBranch => {
            if is_near(i) {
                vec![i.near_branch_target()]
            } else {
                vec![]
            }
        }
        FlowControl::ConditionalBranch => {
            let mut v = vec![next];
            if is_near(i) {
                v.push(i.near_branch_target());
            }
            v
        }
        FlowControl::IndirectBranch => {
            let mut v = vec![];
            if let Some(Resolved::Code { target, .. }) = a.resolved.get(&va) {
                v.push(*target);
            }
            if let Some(t) = a.jumptabs.get(&va) {
                v.extend(t.cases.iter().map(|c| c.1));
            }
            v
        }
        _ => vec![],
    }
}

#[derive(Clone, Debug)]
pub enum Term {
    Jmp(usize),
    Jcc { cc: ConditionCode, taken: usize, fall: usize },
    /// `jrcxz` (wide) / `jecxz`: taken when (r|e)cx is zero; reads no flag
    Jcz { wide: bool, taken: usize, fall: usize },
    Ret,
    /// Jump to another function's entry (a virtual block with no instruction).
    Tail(u64),
    /// Indirect jump through a table; the selector is read by the lowering from the instruction. Targets in case order.
    Switch { cases: Vec<(u64, usize)>, slot: bool },
    /// `jmp [IAT slot]` of an imported function: a tail call of the import.
    TailImport(String),
    /// `jmp reg` / `jmp [mem]` whose target is not known: a tail call through the table of adapters.
    TailIndirect,
    /// int3 / ud2: stop.
    Trap,
    Unsup(String),
}

pub struct Block {
    pub start: u64,
    pub insns: Vec<u64>,
    pub term: Term,
    pub succs: Vec<usize>,
}

pub struct Fun {
    pub entry: u64,
    pub nodes: BTreeMap<u64, Node>,
    pub blocks: Vec<Block>,
    /// indirect jmps that fe-decode resolved to the entry of an opaque function and that were turned back into the run-time lookup (`TailIndirect`): the C path jumps to the target without counting an indirect call
    pub demoted: std::collections::HashSet<u64>,
}

impl Fun {
    pub fn insn(&self, va: u64) -> &Instruction {
        match &self.nodes[&va] {
            Node::Insn(i) => i,
            Node::Bad => unreachable!(),
        }
    }
    pub fn bytes(&self) -> usize {
        self.nodes.values().map(|n| if let Node::Insn(i) = n { i.len() } else { 0 }).sum()
    }
}

/// Builds the blocks of `f`. Block 0 is the entry; real blocks follow in ascending address order; virtual tail-call blocks come last.
pub fn build(f: u64, nodes: BTreeMap<u64, Node>, entries: &BTreeSet<u64>, a: &Analysis, demote: &std::collections::HashSet<u64>) -> Fun {
    let mut leaders: BTreeSet<u64> = BTreeSet::new();
    leaders.insert(f);
    for (&va, n) in &nodes {
        let Node::Insn(i) = n else { continue };
        let next = va + i.len() as u64;
        let ends = match i.flow_control() {
            FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch | FlowControl::IndirectBranch => {
                for s in node_succs(va, n, a) {
                    if nodes.contains_key(&s) {
                        leaders.insert(s);
                    }
                }
                true
            }
            FlowControl::Return | FlowControl::Interrupt | FlowControl::Exception => true,
            FlowControl::Call => !is_near(i),
            _ => false,
        };
        if ends && nodes.contains_key(&next) {
            leaders.insert(next);
        }
    }
    let mut starts: Vec<u64> = vec![f];
    starts.extend(leaders.iter().copied().filter(|&v| v != f));
    let bidx: HashMap<u64, usize> = starts.iter().enumerate().map(|(k, &s)| (s, k)).collect();
    let nreal = starts.len();
    let mut tails: Vec<u64> = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    for &s in &starts {
        let mut insns = Vec::new();
        let mut va = s;
        let mut target = |t: u64, tails: &mut Vec<u64>| -> Result<usize, String> {
            if let Some(&b) = bidx.get(&t) {
                return Ok(b);
            }
            if entries.contains(&t) {
                if let Some(p) = tails.iter().position(|&x| x == t) {
                    return Ok(nreal + p);
                }
                tails.push(t);
                return Ok(nreal + tails.len() - 1);
            }
            Err(format!("branch target {t:#x} is not in the function"))
        };
        let term = loop {
            let Some(Node::Insn(i)) = nodes.get(&va) else {
                break Term::Unsup(format!("undecodable instruction or falls out at {va:#x}"));
            };
            insns.push(va);
            let next = va + i.len() as u64;
            match i.flow_control() {
                FlowControl::Return => break Term::Ret,
                FlowControl::Interrupt | FlowControl::Exception => break Term::Trap,
                FlowControl::UnconditionalBranch => {
                    if !is_near(i) {
                        break Term::Unsup("far jmp".into());
                    }
                    break match target(i.near_branch_target(), &mut tails) {
                        Ok(b) => Term::Jmp(b),
                        Err(e) => Term::Unsup(e),
                    };
                }
                FlowControl::ConditionalBranch => {
                    if i.condition_code() == ConditionCode::None && is_near(i) && matches!(i.mnemonic(), Mnemonic::Jrcxz | Mnemonic::Jecxz) {
                        let t = target(i.near_branch_target(), &mut tails);
                        let fl = target(next, &mut tails);
                        break match (t, fl) {
                            (Ok(t), Ok(fl)) => Term::Jcz { wide: i.mnemonic() == Mnemonic::Jrcxz, taken: t, fall: fl },
                            (Err(e), _) | (_, Err(e)) => Term::Unsup(e),
                        };
                    }
                    if !is_near(i) || i.condition_code() == ConditionCode::None {
                        break Term::Unsup("loop/jcxz/far conditional branch".into());
                    }
                    let t = target(i.near_branch_target(), &mut tails);
                    let fl = target(next, &mut tails);
                    break match (t, fl) {
                        (Ok(t), Ok(fl)) => Term::Jcc { cc: i.condition_code(), taken: t, fall: fl },
                        (Err(e), _) | (_, Err(e)) => Term::Unsup(e),
                    };
                }
                FlowControl::IndirectBranch => {
                    if let Some(Resolved::Import { name }) = a.resolved.get(&va) {
                        break Term::TailImport(name.clone());
                    }
                    let Some(t) = a.jumptabs.get(&va) else {
                        // fe-lift lifts a jmp that fe-decode resolved to code (a delay-load thunk's `jmp [slot]` whose slot holds a stub address,
                        // a constant register) as a plain jump; it is not a table lookup of the target, which need not be a function entry
                        if let Some(Resolved::Code { target: tv, .. }) = a.resolved.get(&va).filter(|_| !demote.contains(&va)) {
                            break match target(*tv, &mut tails) {
                                Ok(b) => Term::Jmp(b),
                                Err(e) => Term::Unsup(e),
                            };
                        }
                        break Term::TailIndirect;
                    };
                    let mut tg = Vec::new();
                    let mut err = None;
                    for &(cv, tv) in &t.cases {
                        match target(tv, &mut tails) {
                            Ok(b) => tg.push((cv, b)),
                            Err(e) => err = Some(e),
                        }
                    }
                    break match err {
                        Some(e) => Term::Unsup(e),
                        None => Term::Switch { cases: tg, slot: t.select == fe_decode::Select::Slot },
                    };
                }
                FlowControl::Call if !is_near(i) => break Term::Unsup("indirect or far call ends the block".into()),
                _ => {
                    if let Some(&b) = bidx.get(&next) {
                        break Term::Jmp(b);
                    }
                    va = next;
                }
            }
        };
        let start = s;
        blocks.push(Block { start, insns, term, succs: vec![] });
    }
    for &t in &tails {
        blocks.push(Block { start: t, insns: vec![], term: Term::Tail(t), succs: vec![] });
    }
    for b in blocks.iter_mut() {
        b.succs = match &b.term {
            Term::Jmp(x) => vec![*x],
            Term::Jcc { taken, fall, .. } | Term::Jcz { taken, fall, .. } => {
                if taken == fall {
                    vec![*taken]
                } else {
                    vec![*taken, *fall]
                }
            }
            Term::Switch { cases, .. } => {
                let mut v: Vec<usize> = cases.iter().map(|c| c.1).collect();
                v.sort();
                v.dedup();
                v
            }
            _ => vec![],
        };
    }
    let mut demoted_here = std::collections::HashSet::new();
    for b in &blocks {
        if let (Term::TailIndirect, Some(&last)) = (&b.term, b.insns.last()) {
            if demote.contains(&last) {
                demoted_here.insert(last);
            }
        }
    }
    Fun { entry: f, nodes, blocks, demoted: demoted_here }
}
