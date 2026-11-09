use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: fe-wasmgen FILE OUT.wasm [--pdata] [--seeds SEEDS.txt] [--teb ADDR] [--hybrid-r ADDR --hybrid-x ADDR --hybrid-mx MXCSR BUF --hybrid-progress ADDR [--hybrid-dispatch LIMIT LANDING HOOK] [--tls-ctx OFFSET] --hybrid-only LIST --c-imports FILE --mem-import MODULE.NAME] [--report OUT.tsv]   (test and measurement entries: --export-all | --export VA[,VA..], --roots VA,.., --select-file FILE, --rel-sites FILE, --adapt-all, --census, --mx-force-slow)");
    ExitCode::from(64)
}

fn parse_hex(s: &str) -> Option<u64> {
    u64::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return usage();
    }
    let mut conf = fe_wasmgen::Config::default();
    let mut pdata = false;
    let mut seeds_file: Option<String> = None;
    let mut rel_sites: Option<String> = None;
    let mut report: Option<String> = None;
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--pdata" => pdata = true,
            "--adapt-all" => conf.adapt_all = true,
            "--census" => conf.census = true,
            "--hybrid-only" if i + 1 < args.len() => {
                let t = std::fs::read_to_string(&args[i + 1]).expect("hybrid-only file");
                conf.hybrid_only = Some(t.lines().filter_map(parse_hex).collect());
                i += 1;
            }
            "--c-imports" if i + 1 < args.len() => {
                let t = std::fs::read_to_string(&args[i + 1]).expect("c-imports file");
                conf.c_imports = Some(t.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).map(|l| match l.split_once('=') { Some((a, b)) => (a.to_string(), b.to_string()), None => (l.to_string(), format!("fle_imp_{l}")) }).collect());
                i += 1;
            }
            "--tls-ctx" if i + 1 < args.len() => {
                conf.tls_ctx = parse_hex(&args[i + 1]).map(|v| v as u32);
                i += 1;
            }
            "--mem-import" if i + 1 < args.len() => {
                let (m, n) = args[i + 1].split_once('.').expect("--mem-import MODULE.NAME");
                conf.mem_import = Some((m.to_string(), n.to_string()));
                i += 1;
            }
            "--hybrid-x" if i + 1 < args.len() => {
                conf.hybrid_x = parse_hex(&args[i + 1]).map(|v| v as u32);
                i += 1;
            }
            "--hybrid-mx" if i + 2 < args.len() => {
                conf.hybrid_mx = Some((parse_hex(&args[i + 1]).expect("--hybrid-mx MXCSR BUF") as u32, parse_hex(&args[i + 2]).expect("--hybrid-mx MXCSR BUF") as u32));
                i += 2;
            }
            "--hybrid-dispatch" if i + 3 < args.len() => {
                let p = |k: usize| parse_hex(&args[i + k]).expect("--hybrid-dispatch LIMIT LANDING HOOK") as u32;
                conf.hybrid_dispatch = Some((p(1), p(2), p(3)));
                i += 3;
            }
            "--hybrid-progress" if i + 1 < args.len() => {
                conf.hybrid_progress = parse_hex(&args[i + 1]).map(|v| v as u32);
                i += 1;
            }
            "--hybrid-r" if i + 1 < args.len() => {
                conf.hybrid_r = parse_hex(&args[i + 1]).map(|v| v as u32);
                i += 1;
            }
            "--mx-force-slow" => fe_wasmgen::lower::MX_FORCE_SLOW.store(true, std::sync::atomic::Ordering::Relaxed),
            "--teb" if i + 1 < args.len() => {
                conf.teb = parse_hex(&args[i + 1]).map(|v| v as u32);
                i += 1;
            }
            "--export-all" => conf.export = Some(Default::default()),
            "--export" if i + 1 < args.len() => {
                conf.export = Some(args[i + 1].split(',').filter_map(parse_hex).collect());
                i += 1;
            }
            "--roots" if i + 1 < args.len() => {
                conf.roots = Some(args[i + 1].split(',').filter_map(parse_hex).collect());
                i += 1;
            }
            "--select-file" if i + 1 < args.len() => {
                let t = std::fs::read_to_string(&args[i + 1]).expect("select file");
                conf.roots = Some(t.lines().filter_map(parse_hex).collect());
                i += 1;
            }
            "--rel-sites" if i + 1 < args.len() => {
                rel_sites = Some(args[i + 1].clone());
                i += 1;
            }
            "--seeds" if i + 1 < args.len() => {
                seeds_file = Some(args[i + 1].clone());
                i += 1;
            }
            "--report" if i + 1 < args.len() => {
                report = Some(args[i + 1].clone());
                i += 1;
            }
            _ => return usage(),
        }
        i += 1;
    }
    let data = match std::fs::read(&args[1]) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {e}", args[1]);
            return ExitCode::FAILURE;
        }
    };
    let mut extra: Vec<u64> = Vec::new();
    if pdata {
        let pe = fe_pe::Pe::parse(&data).expect("pe");
        extra.extend(fe_decode::pdata_seeds(&pe).expect("pdata"));
        extra.extend(fe_decode::adjustor::adjustor_seeds(&pe).expect("adjustors"));
    }
    if let Some(p) = seeds_file {
        let s = std::fs::read_to_string(&p).expect("seeds");
        extra.extend(s.lines().filter_map(parse_hex));
    }
    let t0 = std::time::Instant::now();
    let mut specs: Vec<fe_decode::Spec> = fe_decode::jumptab::FLENGINE.iter().chain(fe_decode::jumptab::ILTOOLS_6000000).copied().collect();
    let mut rel_targets: std::collections::HashSet<u64> = std::collections::HashSet::new();
    if let Some(p) = rel_sites {
        let rs = rel_site_specs(&data, &p);
        // the case targets are not entries of their own here: the parent holds them as blocks of its switch (the seeds file made them entries for the C path, where they are separate functions)
        let pe = fe_pe::Pe::parse(&data).expect("pe");
        for sp in &rs {
            if let Ok(rva) = pe.va_to_rva(sp.table) {
                if let Ok(raw) = pe.slice_at_rva(rva, 4 * sp.count) {
                    for k in 0..sp.count {
                        let off = i32::from_le_bytes(raw[4 * k..4 * k + 4].try_into().unwrap());
                        rel_targets.insert(sp.table.wrapping_add(off as i64 as u64));
                    }
                }
            }
        }
        specs.extend(rs);
    }
    if !rel_targets.is_empty() {
        let n0 = extra.len();
        extra.retain(|v| !rel_targets.contains(v));
        eprintln!("rel-sites: {} of {} seeds dropped (case targets of the tables)", n0 - extra.len(), n0);
    }
    let r = fe_wasmgen::compile(&data, &extra, &specs, &conf);
    let dt = t0.elapsed();
    match r {
        Ok(c) => {
            std::fs::write(&args[2], &c.wasm).expect("write");
            let rep = &c.report;
            let total_insns: usize = rep.rows.iter().map(|r| r.insns).sum();
            let ok_rows: Vec<_> = rep.rows.iter().filter(|r| r.status == "ok").collect();
            let ok_insns: usize = ok_rows.iter().map(|r| r.insns).sum();
            let ok_bytes: usize = ok_rows.iter().map(|r| r.native_bytes).sum();
            let ok_wasm: usize = ok_rows.iter().map(|r| r.wasm_bytes).sum();
            println!("entries                {}", rep.entries);
            println!("functions_total        {}", rep.rows.len());
            println!("functions_ok           {}", ok_rows.len());
            println!("irreducible            {}", rep.irreducible);
            println!("demoted_resolved_jumps {} (resolved to an opaque function: looked up at run time)", rep.demoted_resolved_jumps);
            for e in &rep.demoted_fns {
                println!("demoted_fn 0x{e:x}");
            }
            println!("insns_total            {}", total_insns);
            println!("insns_ok               {}", ok_insns);
            println!("native_bytes_ok        {}", ok_bytes);
            println!("code_bytes_ok          {}", ok_wasm);
            println!("module_bytes           {}", rep.module_bytes);
            println!("indirect_adapter_bytes {} ({} adapters, {} functions do not fit the indirect signature)", rep.adapter_bytes, rep.indirect_adapters, rep.nonconforming);
            println!("test_adapter_bytes     {}", rep.test_adapter_bytes);
            if conf.hybrid_r.is_some() {
                println!("typed_indirect         {} adapters, table {} slots, {} buckets (largest {}), {} bytes of adapters and table", rep.indirect_adapters, rep.typed_slots, rep.typed_buckets, rep.typed_max_bucket, if rep.typed_slots > 0 { rep.adapter_bytes } else { 0 });
                println!("indirect_fit_diag      {} fit, {} do not fit the uniform indirect signature (decision 0285 phase 1, diagnostic only: nothing is generated); standalone adapters of the fitting ones would take {} code bytes", rep.indirect_fit, rep.indirect_nofit, rep.indirect_fit_adapter_bytes);
                for (k, v) in &rep.indirect_nofit_reasons {
                    println!("  indirect_nofit {v:>6}  {k}");
                }
            }
            println!("hybrid_ext_functions   {} (left to the C path), wrapper_bytes {}", rep.ext, rep.wrapper_bytes);
            println!("noreturn_functions     {}", rep.noreturn);
            println!("unbalanced_rsp_functions {} (not compiled: RSP at a return is not the entry value)", rep.unbalanced);
            println!("indirect_violations    {} (functions with a value live in a volatile register across an indirect call; counted, not excluded)", rep.indirect_violations);
            println!("indirect_call_sites    {}", rep.stats.indirect_calls);
            println!("import_call_sites      {} ({} distinct imports)", rep.stats.import_calls, rep.imports);
            println!("hi_checks              {}", rep.stats.hi_checks);
            println!("mem_ops                {}", rep.stats.mem_ops);
            println!("trapped_vex_sites      {}", rep.stats.trapped_vex);
            println!("pres_iterations        {}", rep.pres_iterations);
            println!("sig_iterations         {}", rep.sig_iterations);
            println!("compile_ms             {}", dt.as_millis());
            println!("nonconforming (not an indirect target):");
            for (k, v) in &rep.nonconf_reasons {
                println!("  {v:>6}  {k}");
            }
            if conf.census {
                println!("unsupported_instructions (reason, occurrences; only in functions that passed the flag analysis)");
                let mut u: Vec<_> = rep.unsupported.iter().collect();
                u.sort_by(|a, b| b.1.cmp(a.1));
                for (k, v) in u.iter().take(60) {
                    println!("  {v:>8}  {k}");
                }
            }
            println!("reasons");
            let mut rs: Vec<_> = rep.reasons.iter().collect();
            rs.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
            for (k, (n, ins)) in rs {
                println!("  {n:>6} funcs {ins:>7} insns  {k}");
            }
            println!("blocked_by_root_reason (root functions, functions reached backwards [overlaps between roots], their instructions)");
            let mut bl: Vec<_> = rep.blocked.iter().collect();
            bl.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
            for (k, (r, f, i)) in bl {
                println!("  {r:>5} roots {f:>6} funcs {i:>8} insns  {k}");
            }
            if let Some(p) = report {
                let mut s = String::from("va\tinsns\tnative_bytes\twasm_code_bytes\tsig_in\tsig_out\telided\tstatus");
                if conf.census {
                    s.push_str("\tdirect\tcallees");
                }
                if conf.hybrid_r.is_some() {
                    s.push_str("\tindirect");
                }
                s.push('\n');
                for r in &rep.rows {
                    s.push_str(&format!("{:#x}\t{}\t{}\t{}\t{:#06x}\t{:#06x}\t{:#06x}\t{}", r.va, r.insns, r.native_bytes, r.wasm_bytes, r.sig_in, r.sig_out, r.elided, r.status));
                    if conf.census {
                        let c: Vec<String> = r.callees.iter().map(|x| format!("{x:x}")).collect();
                        s.push_str(&format!("\t{}\t{}", r.direct.join(" ## "), c.join(",")));
                    }
                    if conf.hybrid_r.is_some() {
                        s.push_str(&format!("\t{}", r.indirect));
                    }
                    s.push('\n');
                }
                std::fs::write(p, s).expect("report");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// decision 0181 (hyb2, --rel-sites): `Spec`s for the table-relative jump tables of the third-party plug-in (`lea B,[rip+T]; movsxd X,[B+I*4]; add X,B; jmp X`: the entries are signed 32-bit
/// offsets from T) from the sites file of lift/harness/vst3a/rel_table_seeds.py (jmp_va, table_va, entries, how; tab separated, one header line). The annotated function is the span of the
/// pieces of the `.pdata` function that holds the jump; the checks of `jumptab::resolve` (the bytes of the jump, the targets in .text and in the span) still apply.
fn rel_site_specs(data: &[u8], path: &str) -> Vec<fe_decode::Spec> {
    use fe_decode::jumptab::{FuncMap, Kind};
    let pe = fe_pe::Pe::parse(data).expect("pe");
    let fm = FuncMap::from_pe(&pe).expect("function map");
    let txt = std::fs::read_to_string(path).expect("rel sites");
    let mut out = Vec::new();
    for l in txt.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 3 {
            continue;
        }
        let (Some(jmp), Some(table), Ok(count)) = (parse_hex(f[0]), parse_hex(f[1]), f[2].trim().parse::<usize>()) else { continue };
        let Ok(rva) = pe.va_to_rva(jmp) else { continue };
        let Some((_, pieces)) = fm.function_at(rva) else { continue };
        let begin = pieces.iter().map(|p| p.0).min().unwrap();
        let end = pieces.iter().map(|p| p.1).max().unwrap();
        let Ok(bytes) = pe.slice_at_rva(rva, 15) else { continue };
        let mut d = iced_x86::Decoder::with_ip(64, bytes, jmp, iced_x86::DecoderOptions::NONE);
        let ins = d.decode();
        if ins.is_invalid() || ins.flow_control() != iced_x86::FlowControl::IndirectBranch {
            continue;
        }
        let code: &'static [u8] = Box::leak(bytes[..ins.len()].to_vec().into_boxed_slice());
        out.push(fe_decode::Spec { jmp_va: jmp, code, func: (pe.rva_to_va(begin), pe.rva_to_va(end)), table, count, kind: Kind::Rel32 { base: table } });
    }
    eprintln!("rel-sites: {} specs from {}", out.len(), path);
    out
}
