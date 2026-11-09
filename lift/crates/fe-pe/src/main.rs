use fe_pe::{ImportSymbol, Pe};
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: fe-pe pe-info FILE | fe-pe pe-exports FILE | fe-pe pe-pdata-seeds FILE | fe-pe pe-pdata-info FILE | fe-pe pe-init-table FILE [OUT.tsv] | fe-pe rebase FILE NEWBASE OUT");
    ExitCode::from(64)
}

/// `rebase`: decision 0080. NEWBASE is hexadecimal with a 0x prefix. OUT is a new file (an existing one is an error).
fn rebase_cmd(path: &str, newbase: &str, out: &str) -> Result<(), String> {
    let digits = newbase
        .strip_prefix("0x")
        .or_else(|| newbase.strip_prefix("0X"))
        .ok_or_else(|| format!("NEWBASE {newbase}: expected hexadecimal with a 0x prefix"))?;
    let new_base = u64::from_str_radix(digits, 16).map_err(|e| format!("NEWBASE {newbase}: {e}"))?;
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let (bytes, r) = fe_pe::rebase::rebase(&data, new_base).map_err(|e| format!("{path}: {e}"))?;
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(out).map_err(|e| format!("{out}: {e}"))?;
        f.write_all(&bytes).map_err(|e| format!("{out}: {e}"))?;
    }
    println!("old_base            {:#x}", r.old_base);
    println!("new_base            {:#x}", r.new_base);
    println!("delta               {:#x}", r.delta);
    println!("reloc_blocks        {}", r.blocks);
    println!("dir64               {}", r.dir64);
    println!("absolute            {}", r.absolute);
    println!("checksum            {:#x} (unchanged)", r.checksum);
    println!("out                 {out} ({} bytes)", bytes.len());
    Ok(())
}

/// `pe-init-table`: the A4 unit table (docs/investigations/a4-survey-tool.md). Summary on stdout; with OUT.tsv the
/// full list is written there (a new file; an existing one is an error).
fn init_table(pe: &Pe, out: Option<&str>) -> Result<(), String> {
    use fe_pe::inittable::{UnitKind, INIT_COUNT_VA, INIT_CONTEXT_VA};
    let reference = pe.entry_header_reference();
    let header_va = reference.map(|r| r.1).unwrap_or(fe_pe::inittable::INIT_TABLE_HEADER_VA);
    let t = pe.init_table(header_va).map_err(|e| format!("init table: {e}"))?;
    match reference {
        Some((at, va)) => println!("header_from_entry   {va:#x} (lea rdx at {at:#x})"),
        None => println!("header_from_entry   none (default {header_va:#x})"),
    }
    println!("init_context        {INIT_CONTEXT_VA:#x} (+8 InitCount at {INIT_COUNT_VA:#x})");
    println!("header              {:#x}", t.header_va);
    println!("unit_count          {}", t.unit_count);
    println!("units_ptr           {:#x}", t.units_va);
    println!("runnable            {} (distinct proc {})", t.count(UnitKind::Runnable), t.distinct_procs(UnitKind::Runnable));
    println!("empty               {}", t.count(UnitKind::Empty));
    println!("noproc              {}", t.count(UnitKind::NoProc));
    println!("unreadable          {}", t.count(UnitKind::Unreadable));
    if let Some(p) = out {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(p).map_err(|e| format!("{p}: {e}"))?;
        f.write_all(t.to_tsv().as_bytes()).map_err(|e| format!("{p}: {e}"))?;
        println!("tsv                 {p}");
    }
    Ok(())
}

fn run(cmd: &str, path: &str, out: Option<&str>) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let pe = Pe::parse(&data).map_err(|e| format!("{path}: {e}"))?;
    match cmd {
        "pe-init-table" => init_table(&pe, out)?,
        "pe-info" => {
            let (f, o) = (&pe.file_header, &pe.optional);
            println!("file_size        {}", data.len());
            println!("machine          {:#06x}", f.machine);
            println!("characteristics  {:#06x}", f.characteristics);
            println!("timestamp        {:#010x}", f.time_date_stamp);
            println!("image_base       {:#x}", o.image_base);
            println!("size_of_image    {:#x}", o.size_of_image);
            println!("size_of_headers  {:#x}", o.size_of_headers);
            println!("entry_rva        {:#x}", o.address_of_entry_point);
            println!("section_align    {:#x}", o.section_alignment);
            println!("file_align       {:#x}", o.file_alignment);
            println!("subsystem        {}", o.subsystem);
            println!("dll_char         {:#06x}", o.dll_characteristics);
            println!("data_dirs        {}", o.number_of_rva_and_sizes);
            println!("sections         {}", pe.sections.len());
            for s in &pe.sections {
                println!(
                    "  {:<8} va {:#010x} vsize {:#010x} raw {:#010x} rawsize {:#010x} flags {:#010x}",
                    s.name, s.virtual_address, s.virtual_size, s.pointer_to_raw_data, s.size_of_raw_data,
                    s.characteristics
                );
            }
            let exports = pe.exports().map_err(|e| format!("exports: {e}"))?;
            println!("exports          {}", exports.len());
            let imports = pe.imports().map_err(|e| format!("imports: {e}"))?;
            let names = imports
                .iter()
                .flat_map(|i| i.entries.iter())
                .filter(|e| matches!(e.symbol, ImportSymbol::Name { .. }))
                .count();
            let ords = imports.iter().map(|i| i.entries.len()).sum::<usize>() - names;
            println!("import_dlls      {}", imports.len());
            println!("import_names     {names}");
            println!("import_ordinals  {ords}");
            for i in &imports {
                println!("  {} ({} entries, first IAT RVA {:#x})", i.dll, i.entries.len(), i.first_thunk);
            }
        }
        "pe-exports" => {
            for e in pe.exports().map_err(|e| format!("exports: {e}"))? {
                print!(
                    "{}\t{:#x}\t{:#x}\t{}",
                    e.ordinal,
                    e.rva,
                    e.va,
                    e.name.as_deref().unwrap_or("<no name>")
                );
                if let Some(fw) = e.forwarder {
                    print!("\t-> {fw}");
                }
                println!();
            }
        }
        "pe-pdata-seeds" => {
            for va in pe.function_start_vas().map_err(|e| format!("pdata: {e}"))? {
                println!("{va:#x}");
            }
        }
        "pe-pdata-info" => {
            let t = pe.function_table().map_err(|e| format!("pdata: {e}"))?;
            let chained = t.iter().filter(|e| e.is_chained()).count();
            let starts = pe.function_starts().map_err(|e| format!("pdata: {e}"))?;
            println!("entries          {}", t.len());
            println!("chained          {}", chained);
            println!("function_starts  {}", starts.len());
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "rebase" {
        if args.len() != 5 {
            return usage();
        }
        return match rebase_cmd(&args[2], &args[3], &args[4]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let init = args.len() >= 2 && args[1] == "pe-init-table";
    let ok = if init {
        args.len() == 3 || args.len() == 4
    } else {
        args.len() == 3 && matches!(args[1].as_str(), "pe-info" | "pe-exports" | "pe-pdata-seeds" | "pe-pdata-info")
    };
    if !ok {
        return usage();
    }
    match run(&args[1], &args[2], args.get(3).map(String::as_str)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
