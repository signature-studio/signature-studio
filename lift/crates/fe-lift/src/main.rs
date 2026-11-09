use std::fs::File;
use std::io::BufWriter;
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: fe-lift lift FILE OUT.c [--pdata] [--seeds SEEDS.txt] [--hook-points HOOKS.txt] [--site-overrides SITES.txt] [--hle-entries HLE.txt] [--swap-entries SWAP.txt] [--prune-list PRUNE.tsv] [--seh-handler RVA]... [--extra-imports ROWS.txt]");
    ExitCode::from(64)
}

/// Extra seeds: the `.pdata` function starts and the data-referenced adjustor thunks outside `.pdata`
/// (`--pdata`, decision 0051), the VAs in a file, one hex VA per line, and, last, the
/// virtual-call thunks that no function of the closure of all the others reaches (`--pdata`, decision 0100) and the landings of the guest's
/// `setjmp` (the instruction after each call of it, decision 0100).
fn extra_seeds(
    data: &[u8],
    pdata: bool,
    seeds_file: Option<&str>,
    opts: &fe_decode::Options,
) -> Result<Vec<u64>, String> {
    let mut v = Vec::new();
    if pdata {
        let pe = fe_pe::Pe::parse(data).map_err(|e| e.to_string())?;
        v.extend(fe_decode::pdata_seeds(&pe)?);
        v.extend(fe_decode::adjustor::adjustor_seeds(&pe)?);
    }
    if let Some(p) = seeds_file {
        let s = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        for l in s.lines().map(str::trim).filter(|l| !l.is_empty()) {
            v.push(u64::from_str_radix(l.trim_start_matches("0x"), 16).map_err(|e| format!("{p}: bad seed {l}: {e}"))?);
        }
    }
    {
        let pe = fe_pe::Pe::parse(data).map_err(|e| e.to_string())?;
        let base = fe_decode::analyze_with_options(&pe, &v, fe_decode::jumptab::FLENGINE, opts)?;
        let mut add = Vec::new();
        if pdata {
            add.extend(fe_decode::adjustor::vcall_seeds(&pe, &base)?);
        }
        let text = fe_decode::Text::from_pe(&pe)?;
        add.extend(fe_decode::setjmp::landings(&text, &base.insns).into_keys());
        v.extend(add);
    }
    Ok(v)
}

/// The inline hook points of a file (`--hook-points`, decision 0105) as VAs of the image being lifted: one address per line, a VA or `rva:`
/// and a hex RVA (the ImageBase of the image is added), `#` comments.
fn hook_vas(data: &[u8], file: &str) -> Result<Vec<u64>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let addrs = fe_lift::hook::parse_file(&text).map_err(|e| format!("{file}: {e}"))?;
    let pe = fe_pe::Pe::parse(data).map_err(|e| e.to_string())?;
    Ok(fe_lift::hook::to_vas(&addrs, pe.optional.image_base))
}

/// The HLE entries of a file (`--hle-entries`, decision 0162) as entries of the image being lifted (see `fe_lift::hle`).
fn hle_entries(data: &[u8], file: &str) -> Result<Vec<fe_lift::hle::Entry>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let specs = fe_lift::hle::parse_file(&text).map_err(|e| format!("{file}: {e}"))?;
    let pe = fe_pe::Pe::parse(data).map_err(|e| e.to_string())?;
    Ok(fe_lift::hle::resolve(&specs, pe.optional.image_base))
}

/// The pruned functions of a file (`--prune-list`, decision 0168) as entries of the image being lifted (see `fe_lift::prune`).
fn prune_entries(data: &[u8], file: &str) -> Result<Vec<fe_lift::prune::Entry>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let specs = fe_lift::prune::parse_file(&text).map_err(|e| format!("{file}: {e}"))?;
    let pe = fe_pe::Pe::parse(data).map_err(|e| e.to_string())?;
    Ok(fe_lift::prune::resolve(&specs, pe.optional.image_base))
}

/// The swapped entries of a file (`--swap-entries`, decision 0160) as entries of the image being lifted (see `fe_lift::swap`).
fn swap_entries(data: &[u8], file: &str) -> Result<Vec<fe_lift::swap::Entry>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let specs = fe_lift::swap::parse_file(&text).map_err(|e| format!("{file}: {e}"))?;
    let pe = fe_pe::Pe::parse(data).map_err(|e| e.to_string())?;
    Ok(fe_lift::swap::resolve(&specs, pe.optional.image_base))
}

/// The call-site memory overrides of a file (`--site-overrides`, decision 0120) as sites of the image being lifted (see `fe_lift::site_override`).
fn override_sites(data: &[u8], file: &str) -> Result<Vec<fe_lift::site_override::Site>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let specs = fe_lift::site_override::parse_file(&text).map_err(|e| format!("{file}: {e}"))?;
    let pe = fe_pe::Pe::parse(data).map_err(|e| e.to_string())?;
    Ok(fe_lift::site_override::resolve(&specs, pe.optional.image_base))
}

fn run(
    input: &str,
    output: &str,
    pdata: bool,
    seeds_file: Option<&str>,
    hooks_file: Option<&str>,
    sites_file: Option<&str>,
    hle_file: Option<&str>,
    swap_file: Option<&str>,
    prune_file: Option<&str>,
    seh_handlers: &[u32],
    opts: &fe_decode::Options,
    gopt: &fe_lift::GenOptions,
) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("{input}: {e}"))?;
    let mut extra = extra_seeds(&data, pdata, seeds_file, opts)?;
    // Decision 0132: the except landings of the guest's exception handler(s) named by `--seh-handler` are function entries (seeds, last), and the functions that
    // hold them get the SEH entry wrapper. Without the option nothing here runs and the output is unchanged.
    let seh_l = if seh_handlers.is_empty() {
        fe_decode::seh::SehLandings::default()
    } else {
        let pe = fe_pe::Pe::parse(&data).map_err(|e| e.to_string())?;
        fe_decode::seh::delphi_landings(&pe, seh_handlers)?
    };
    extra.extend(seh_l.landing_vas());
    let seh = fe_lift::SehOptions { funcs: seh_l.by_func.clone() };
    let hooks = match hooks_file {
        Some(p) => hook_vas(&data, p)?,
        None => Vec::new(),
    };
    let sites = match sites_file {
        Some(p) => override_sites(&data, p)?,
        None => Vec::new(),
    };
    let mut gopt = gopt.clone();
    if let Some(p) = hle_file {
        gopt.hle = hle_entries(&data, p)?;
    }
    if let Some(p) = swap_file {
        gopt.swap = swap_entries(&data, p)?;
    }
    if let Some(p) = prune_file {
        gopt.prune = prune_entries(&data, p)?;
    }
    let gopt = &gopt;
    let f = File::create(output).map_err(|e| format!("{output}: {e}"))?;
    let mut w = BufWriter::new(f);
    // The annotated jumps (decision 0036) name VAs: those of FLEngine at 0x400000 and those of ILTools at 0x6000000 (decision 0104). An
    // annotation is used only when an unresolved jump sits at its address, so the other images are not affected.
    let specs: Vec<fe_decode::Spec> = fe_decode::jumptab::FLENGINE.iter().chain(fe_decode::jumptab::ILTOOLS_6000000).copied().collect();
    let st = fe_lift::generate_full(&data, &extra, &specs, &hooks, &sites, opts, &seh, gopt, &mut w)?;
    use std::io::Write;
    w.flush().map_err(|e| e.to_string())?;
    println!("closure_instructions   {}", st.closure_insns);
    println!("functions              {}", st.functions);
    println!("emitted_instructions   {}", st.emitted_insns);
    println!("bad_nodes              {}", st.bad_nodes);
    println!("tail_calls             {}", st.tail_calls);
    println!("gotos                  {}", st.gotos);
    println!("c_lines                {}", st.c_lines);
    println!("c_bytes                {}", st.c_bytes);
    println!("handled");
    for (k, v) in &st.handled {
        println!("  {k:<12} {v}");
    }
    if seh_handlers.is_empty() && gopt.swap.is_empty() {
        println!("outlined_heads         {} (shared instructions before {} after {}; {} heads not promoted: on a tail-call cycle)", st.outlined_heads, st.shared_insns_before, st.shared_insns_after, st.outline_banned);
    }
    if gopt.swap.is_empty() {
        println!(
            "dead_flags             writers {} fully_dead {} partly_dead {} substituted {} rounds {}",
            st.flag_writers, st.flag_dead_full, st.flag_dead_part, st.flag_substituted, st.flag_rounds
        );
    }
    println!("register_jmp_sites     {}", st.register_jmp_sites);
    println!("jumptabs               {}", st.jumptabs);
    println!("msvc_tables            {}", st.msvc_tables);
    println!("setjmp_landings        {}", st.setjmp_landings);
    if st.hook_points > 0 {
        println!("hook_points            {}", st.hook_points);
    }
    if st.site_overrides > 0 {
        println!("site_overrides         {}", st.site_overrides);
    }
    if st.hle_entries > 0 {
        println!("hle_entries            {}", st.hle_entries);
    }
    if st.swap_entries > 0 {
        println!("swap_entries           {}", st.swap_entries);
    }
    if st.pruned_listed > 0 {
        println!(
            "pruned                 listed {} stubs {} seeds_dropped {} cond_branch_targets {}",
            st.pruned_listed, st.pruned_stubs, st.pruned_seeds_dropped, st.pruned_cond_targets
        );
    }
    if !seh_handlers.is_empty() {
        println!(
            "seh                    handler_functions {} scopes {} (finally {} bare except {} typed except {}, {} on entries) wrapped_functions {} landings {}",
            seh_l.handler_functions, seh_l.scopes, seh_l.finally_scopes, seh_l.bare_except_scopes, seh_l.typed_except_scopes, seh_l.typed_entries, st.seh_wrapped, st.seh_landings
        );
    }
    for (va, e) in &st.jumptab_rejected {
        println!("jumptab_rejected       {va:#x}: {e}");
    }
    println!("msvc_rejected          {}", st.msvc_rejected.len());
    for (va, e) in &st.msvc_rejected {
        println!("msvc_rejected          {va:#x}: {e}");
    }
    println!("register_jmps");
    for (k, v) in &st.register_jmps {
        println!("  {k:<22} {v}");
    }
    println!("unsupported_total      {}", st.unsupported_total);
    let mut u: Vec<_> = st.unsupported.iter().collect();
    u.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("unsupported            {}", u.len());
    for (k, v) in u {
        println!("  {v:>8}  {k}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 || args[1] != "lift" {
        return usage();
    }
    let mut pdata = false;
    let mut seeds_file: Option<&str> = None;
    let mut hooks_file: Option<&str> = None;
    let mut sites_file: Option<&str> = None;
    let mut hle_file: Option<&str> = None;
    let mut swap_file: Option<&str> = None;
    let mut prune_file: Option<&str> = None;
    let mut seh_handlers: Vec<u32> = Vec::new();
    // The base proof is decision 0094's (reaching definitions).
    let opts = fe_decode::Options::default();
    let gopt = fe_lift::GenOptions::default();
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--pdata" => pdata = true,
            "--seeds" if i + 1 < args.len() => {
                seeds_file = Some(args[i + 1].as_str());
                i += 1;
            }
            "--hook-points" if i + 1 < args.len() => {
                hooks_file = Some(args[i + 1].as_str());
                i += 1;
            }
            "--site-overrides" if i + 1 < args.len() => {
                sites_file = Some(args[i + 1].as_str());
                i += 1;
            }
            "--hle-entries" if i + 1 < args.len() => {
                hle_file = Some(args[i + 1].as_str());
                i += 1;
            }
            "--swap-entries" if i + 1 < args.len() => {
                swap_file = Some(args[i + 1].as_str());
                i += 1;
            }
            "--prune-list" if i + 1 < args.len() => {
                prune_file = Some(args[i + 1].as_str());
                i += 1;
            }
            "--extra-imports" if i + 1 < args.len() => {
                // Decision 0156 (PROVISIONAL): rows appended after the import table; absent, the output is the one without the option.
                match std::fs::read_to_string(&args[i + 1]).map_err(|e| e.to_string()).and_then(|t| fe_lift::imports_table::set_extra(&t)) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("error: --extra-imports {}: {e}", args[i + 1]);
                        return ExitCode::FAILURE;
                    }
                }
                i += 1;
            }
            "--seh-handler" if i + 1 < args.len() => {
                match u32::from_str_radix(args[i + 1].trim_start_matches("0x"), 16) {
                    Ok(v) => seh_handlers.push(v),
                    Err(_) => return usage(),
                }
                i += 1;
            }
            _ => return usage(),
        }
        i += 1;
    }
    match run(&args[2], &args[3], pdata, seeds_file, hooks_file, sites_file, hle_file, swap_file, prune_file, &seh_handlers, &opts, &gopt) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
