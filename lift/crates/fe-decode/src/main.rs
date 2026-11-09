use fe_decode::{analyze, analyze_with_pdata, analyze_with_pdata_adjustors, analyze_with_seeds, relocseeds, Text};
use fe_pe::Pe;
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: fe-decode closure FILE [--mnemonics] [--pdata [--adjustors]] [--reloc-seeds] | fe-decode reloc-seeds FILE [--not-function] | fe-decode vcall-thunks FILE");
    ExitCode::from(64)
}

fn run(path: &str, mnemonics: bool, pdata: bool, adjustors: bool, reloc: bool) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let pe = Pe::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    let text = Text::from_pe(&pe)?;
    let rs = if reloc { Some(relocseeds::reloc_seeds(&pe)?) } else { None };
    let a = match (&rs, pdata, adjustors) {
        (None, true, true) => analyze_with_pdata_adjustors(&pe)?,
        (None, true, false) => analyze_with_pdata(&pe)?,
        (None, false, _) => analyze(&pe)?,
        (Some(r), _, _) => {
            let mut extra = Vec::new();
            if pdata {
                extra.extend(fe_decode::pdata_seeds(&pe)?);
                if adjustors {
                    extra.extend(fe_decode::adjustor::adjustor_seeds(&pe)?);
                }
            }
            extra.extend(r.seeds.iter().copied());
            analyze_with_seeds(&pe, &extra)?
        }
    };
    let c = &a.closure;
    let first = &text.ranges()[0];
    println!("text_va            {:#x}", first.va);
    println!("text_size          {:#x}", first.bytes.len());
    println!("text_ranges        {}", text.ranges().len());
    for r in text.ranges() {
        println!("  range            {:#x} size {:#x}", r.va, r.bytes.len());
    }
    if let Some(r) = &rs {
        println!("reloc_slots        {}", r.slots);
        println!("reloc_unreadable   {}", r.unreadable_slots);
        println!("reloc_in_code      {}", r.in_code);
        println!("reloc_seeds        {}", r.seeds.len());
        println!("reloc_not_function {}", r.not_function.len());
    }
    println!("entry_va           {:#x}", pe.rva_to_va(pe.optional.address_of_entry_point));
    println!("seeds_code         {}", c.seeds_code.len());
    for (n, v) in &c.seeds_code {
        println!("  {n:<28} {v:#x}");
    }
    println!("seeds_skipped      {}", c.seeds_skipped.len());
    for s in &c.seeds_skipped {
        println!("  {:<28} {:#x} ({})", s.name, s.va, s.reason);
    }
    println!("functions          {}", c.functions);
    println!("blocks             {}", c.blocks);
    println!("instructions       {}", c.instructions);
    println!("decode_failures    {}", c.decode_failures);
    for va in &c.fail_samples {
        println!("  fail_at {va:#x}");
    }
    println!("out_of_text_flow   {}", c.out_of_text_flow);
    println!("out_of_text_calls  {}", c.out_of_text_calls);
    println!("direct_calls       {}", c.direct_calls);
    println!("direct_jumps       {}", c.direct_jumps);
    println!("cond_branches      {}", c.conditional_branches);
    println!("returns            {}", c.returns);
    println!("import_calls       {}", c.import_calls);
    println!("import_jumps       {}", c.import_jumps);
    println!("indirect_calls_oth {}", c.indirect_calls_other);
    println!("indirect_jumps_oth {}", c.indirect_jumps_other);
    println!("far_transfers      {}", c.far_transfers);
    println!("interrupts         {}", c.interrupts);
    if mnemonics {
        let mut v: Vec<_> = c.mnemonics.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        println!("mnemonics          {}", v.len());
        for (m, n) in v {
            println!("  {m:<16} {n}");
        }
    }
    Ok(())
}

/// `reloc-seeds FILE`: the seed VAs (one `0x` hex VA per line, the `fe-lift --seeds` file format) on stdout;
/// the counts on stderr. The values that are in code but not a `.pdata` start are counted, and are written
/// (instead of the seeds, for inspection only) with `--not-function`.
fn reloc_seeds(path: &str, list_not_function: bool) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let pe = Pe::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    let r = relocseeds::reloc_seeds(&pe)?;
    eprintln!("reloc_slots        {}", r.slots);
    eprintln!("reloc_unreadable   {}", r.unreadable_slots);
    eprintln!("reloc_in_code      {}", r.in_code);
    eprintln!("reloc_seeds        {}", r.seeds.len());
    eprintln!("reloc_not_function {}", r.not_function.len());
    let list = if list_not_function { &r.not_function } else { &r.seeds };
    for v in list {
        println!("{v:#x}");
    }
    Ok(())
}

/// `vcall-thunks FILE`: the candidates of decision 0100 (shape, relocation reference, outside `.pdata`, anchored start) one per line
/// on stdout (`0xVA len pre shape reached|seed`), where `reached` is a thunk that a function of the closure of the `.pdata` starts and
/// the 0051 thunks flows into (the guard leaves it out) and `seed` is one `fe-lift --pdata` takes. The counts go to stderr.
fn vcall_thunks(path: &str) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let pe = Pe::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    let all = fe_decode::adjustor::vcall_thunks(&pe)?;
    let base = fe_decode::analyze_with_pdata_adjustors(&pe)?;
    let seeds = fe_decode::adjustor::vcall_seeds(&pe, &base)?;
    eprintln!("vcall_candidates   {}", all.len());
    eprintln!("vcall_seeds        {}", seeds.len());
    for t in &all {
        let k = if seeds.contains(&t.va) { "seed" } else { "reached" };
        println!("{:#x} {} {} {} {k}", t.va, t.len, t.pre, t.shape);
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "vcall-thunks" {
        return match vcall_thunks(&args[2]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let not_fn = args.len() == 4 && args[3] == "--not-function";
    if (args.len() == 3 || not_fn) && args[1] == "reloc-seeds" {
        return match reloc_seeds(&args[2], not_fn) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let mnemonics = args.iter().any(|a| a == "--mnemonics");
    let pdata = args.iter().any(|a| a == "--pdata");
    let adjustors = args.iter().any(|a| a == "--adjustors");
    let reloc = args.iter().any(|a| a == "--reloc-seeds");
    let pos: Vec<&String> = args.iter().skip(1).filter(|a| !a.starts_with("--")).collect();
    if pos.len() != 2 || pos[0] != "closure" || (adjustors && !pdata) {
        return usage();
    }
    match run(pos[1], mnemonics, pdata, adjustors, reloc) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
