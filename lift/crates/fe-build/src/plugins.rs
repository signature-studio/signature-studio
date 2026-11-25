//! The plugins of the engine (Issue #1369). A plugin is a folder `lift/plugins/<name>/` with one `plugin.tsv`: a header line and one row
//! per DLL of the plugin (the main DLL and its helpers; `role` 本体 or 補助). The tables that the build reads are generated from these
//! files into `lift/out-app/plugins/` (not committed), after the fixed rows of the modules that are not plugins yet (the FIXED_* files).
//! The generated rows of a plugin DLL come after the fixed rows of each table, in the order of `key`.
//!
//! The lists of the direct functions of a plugin DLL are not stored: the `lists` stage finds them by running fe-wasmgen on the DLL
//! rebased at its base (PluginList), and the hook points of the DLL and its `omit` RVAs (the C part keeps those) are left out of them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

pub const PLUGINS_DIR: &str = "lift/plugins";
pub const OUT_DIR: &str = "lift/out-app/plugins";
const HOST_DIR: &str = "lift/out-app/host";
const GUEST_ROOT: &str = "C:\\Program Files\\Image-Line\\FL Studio 21\\";
const SEED_ROOT: &str = "lift/out-idle1/inputs/install";
const EXTRA_IMPORTS: &str = "lift/harness/seeds/ship6-imports.txt";
const HOOK_EXISTING: &str = "lift/harness/hooks/fruity-delphi-effect-hook-points.txt";
const XMOD_DIR: &str = "lift/out-app/xmod";
const N0_ARGS: &str = "lift/harness/hyb/n0.args";
const ALIGN: u64 = 0x1_0000;
/// The end of the image area (Issue #1379): Balance's base 0xC1D0000 + 750 MiB, which holds the 160 second-wave images (748,158,976 B of
/// SizeOfImage, with 5% to spare). Must be the -sGLOBAL_BASE of link.rsp (the test the_global_base_is_the_one_of_the_link).
const GLOBAL_BASE: u64 = 0x3AFD_0000;
const FLENGINE_PE: &str = "binaries/win64/FLEngine_x64.dll";
/// The sizes that bound the number of modules (realmods): the runtime's table, the handles of the cursor, and the harness's arrays.
const LIMITS: [(&str, &str); 3] = [("lift/runtime/fle_realmod.c", "#define RM_MAX "), ("lift/runtime/fle_cursor.c", "#define SHARED_MODULE_REALMODS "), ("lift/harness/two/main_two.c", "MAIN_REAL_MAX = ")];
const FIXED_EXTRACT: &str = "ci/extract-dlls.tsv";
const FIXED_SEED: &str = "lift/harness/seeds/vfs-seed-idle1.tsv";
const FIXED_SEED_ARGS: &str = "lift/harness/web/seed.args";
const FIXED_MODULES: &str = "lift/harness/b2d2/modules.tsv";
const FIXED_HYB_ARGS: &str = "lift/harness/hyb/hyb-args.tsv";
const FIXED_HYBRID_CMD: &str = "lift/harness/hyb/hybrid.cmd";
const FIXED_PACK_ARGS: &str = "lift/harness/web/pack.args";
const FIXED_LISTS: &str = "lift/harness/hyb/lists";
const COLUMNS: [&str; 8] = ["key", "guest", "role", "sha256", "size", "hook", "args", "omit"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hook {
    None,
    /// the three hook points of the Delphi runtime that the Fruity plugins share (lift/harness/hooks/fruity-delphi-effect-hook-points.txt)
    Existing,
    Rva(Vec<u64>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dll {
    pub plugin: String,
    pub key: String,
    /// the path in the installer, without the install folder: `Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll`
    pub guest: String,
    pub main: bool,
    pub sha256: String,
    pub size: u64,
    pub hook: Hook,
    pub args: Vec<String>,
    /// RVAs that are left out of the direct list (the C part lifts them). A setjmp's entry is one: the C caller registers the landing.
    pub omit: Vec<u64>,
}

impl Dll {
    /// The DLL's name in the build: its key (`effects.fruity-balance.fruity_balance_x64.dll`). The file names of the plugins are not unique
    /// (Issue #1379: 18 DLLs are `engine_x64.dll`), the keys are; the name is the realmod's, and the stem of the host file and the rebased file.
    pub fn name(&self) -> String {
        self.key.clone()
    }

    pub fn basename(&self) -> &str {
        self.guest.rsplit('\\').next().unwrap_or(&self.guest)
    }

    pub fn stem(&self) -> String {
        self.name().trim_end_matches(".dll").to_string()
    }

    /// the file name of the DLL at its base: `fruity_balance_x64.7b90000.dll`
    pub fn rebased(&self, base: u64) -> String {
        format!("{}.{:x}.dll", self.stem(), base)
    }

    /// the place of the DLL in the host tree (relative to HOST_DIR), where the extraction puts it
    pub fn host_file(&self) -> String {
        format!("lift/out-plugins/{}", self.name())
    }

    /// the name the installer's file list is matched with (a case-insensitive suffix; lower case here)
    pub fn installer_name(&self) -> String {
        self.basename().to_lowercase()
    }

    pub fn guest_path(&self) -> String {
        format!("{GUEST_ROOT}{}", self.guest)
    }

    /// the file of the seed list (relative to HOST_DIR): the folders of the DLL's path above its own folder, and the name (the key, unique)
    pub fn seed_file(&self) -> String {
        let parts: Vec<&str> = self.guest.split('\\').collect();
        format!("{SEED_ROOT}/{}/{}", parts[..parts.len() - 2].join("/"), self.name())
    }

    /// the lower-case path suffix that LoadLibrary of the full path is matched with (fle_realmod_find_path)
    pub fn suffix(&self) -> String {
        self.guest.replace('\\', "/").to_lowercase()
    }
}

/// `lift/plugins/<name>/plugin.tsv`: the header, then the rows of one plugin.
pub fn parse_plugin(plugin: &str, text: &str) -> Result<Vec<Dll>, String> {
    let mut out = Vec::new();
    let mut header = false;
    for (n, line) in text.lines().enumerate() {
        let at = |m: &str| format!("{plugin}/plugin.tsv line {}: {m}", n + 1);
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if !header {
            if f.as_slice() != COLUMNS {
                return Err(at("the header must be key, guest, role, sha256, size, hook, args (TAB separated)"));
            }
            header = true;
            continue;
        }
        if f.len() != 8 {
            return Err(at(&format!("{} fields, want 8", f.len())));
        }
        let main = match f[2] {
            "本体" => true,
            "補助" => false,
            other => return Err(at(&format!("role {other}: want 本体 or 補助"))),
        };
        let size = f[4].parse::<u64>().ok().filter(|v| *v > 0).ok_or_else(|| at("size must be a positive integer"))?;
        let hook = parse_hook(f[5]).map_err(|e| at(&e))?;
        let args = if f[6].trim().is_empty() { vec!["--pdata".to_string()] } else { f[6].split_whitespace().map(String::from).collect() };
        let omit = rvas_or_none(f[7]).map_err(|e| at(&format!("omit: {e}")))?;
        out.push(Dll { plugin: plugin.to_string(), key: f[0].to_string(), guest: f[1].to_string(), main, sha256: f[3].to_string(), size, hook, args, omit });
    }
    if !header {
        return Err(format!("{plugin}/plugin.tsv: no header"));
    }
    if out.is_empty() {
        return Err(format!("{plugin}/plugin.tsv: no DLL row"));
    }
    Ok(out)
}

fn parse_hook(text: &str) -> Result<Hook, String> {
    match text {
        "none" => Ok(Hook::None),
        "existing" => Ok(Hook::Existing),
        t if t.starts_with("rva:") => {
            let rvas = t["rva:".len()..].split(',').map(hex).collect::<Result<Vec<u64>, String>>()?;
            if rvas.len() != 3 {
                return Err(format!("hook {t}: three RVAs are wanted"));
            }
            Ok(Hook::Rva(rvas))
        }
        other => Err(format!("hook {other}: want none, existing or rva:0x..,0x..,0x..")),
    }
}

fn hex(text: &str) -> Result<u64, String> {
    u64::from_str_radix(text.trim().trim_start_matches("0x"), 16).map_err(|e| format!("bad hex {text}: {e}"))
}

/// The RVAs of an `omit` field: comma separated hex, or `-` for none.
fn rvas_or_none(text: &str) -> Result<Vec<u64>, String> {
    if text == "-" {
        return Ok(Vec::new());
    }
    text.split(',').map(hex).collect()
}

/// The VAs of a plugins.tsv column (`vas_text`): comma separated hex, or `-` for none.
fn vas_of(text: &str) -> Result<BTreeSet<u64>, String> {
    Ok(rvas_or_none(text)?.into_iter().collect())
}

fn vas_text(vas: &[u64]) -> String {
    if vas.is_empty() {
        return "-".to_string();
    }
    vas.iter().map(|v| format!("0x{v:x}")).collect::<Vec<String>>().join(",")
}

fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// The checks of the rows of all plugins: the keys (they are the realmod names) and the installer paths must be unique and well formed, and the
/// hashes. The file names need not be unique (Issue #1379: the realmod's name is the key; the file name is only the installer's name).
pub fn validate(dlls: &[Dll]) -> Result<(), String> {
    let (mut keys, mut guests) = (BTreeSet::new(), BTreeSet::new());
    for d in dlls {
        let at = |m: &str| format!("plugin {} ({}): {m}", d.plugin, d.key);
        if !valid_key(&d.key) {
            return Err(at("the key is lower case letters, digits, '.', '-' and '_' only"));
        }
        if !d.key.ends_with(".dll") {
            return Err(at("the key ends with .dll (it is the realmod name and the stem of the rebased file)"));
        }
        if !keys.insert(d.key.clone()) {
            return Err(at("the key is not unique"));
        }
        if d.guest.split('\\').count() < 3 || d.guest.contains('/') {
            return Err(at("the guest path is Plugins\\...\\file (backslashes, at least three parts)"));
        }
        if !guests.insert(d.guest.to_lowercase()) {
            return Err(at("the guest path is not unique"));
        }
        if !d.basename().to_lowercase().ends_with(".dll") {
            return Err(at("the file is not a DLL"));
        }
        if d.sha256.len() != 64 || !d.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(at("sha256 is 64 lower case hex digits"));
        }
    }
    Ok(())
}

fn valid_folder(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// All plugin rows of a plugins folder, sorted by key, after the checks.
pub fn read_plugins_from(dir: &str) -> Result<Vec<Dll>, String> {
    let dlls = read_rows_from(dir)?;
    validate(&dlls)?;
    Ok(dlls)
}

/// The plugin rows of a plugins folder, sorted by key, without the checks of `validate` (read_plugins_from adds them).
pub fn read_rows_from(dir: &str) -> Result<Vec<Dll>, String> {
    let mut folders = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("{dir}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            folders.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    folders.sort();
    let mut dlls = Vec::new();
    for folder in &folders {
        if !valid_folder(folder) {
            return Err(format!("{dir}/{folder}: the folder name is lower case letters, digits and '-' only"));
        }
        let path = format!("{dir}/{folder}/plugin.tsv");
        let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        dlls.extend(parse_plugin(folder, &text)?);
    }
    dlls.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(dlls)
}

pub fn read_plugins() -> Result<Vec<Dll>, String> {
    read_plugins_from(PLUGINS_DIR)
}

/// The keys in the order of the plan (the `lists` stage has one step for each).
pub fn keys() -> Result<Vec<String>, String> {
    Ok(read_plugins()?.into_iter().map(|d| d.key).collect())
}

pub fn align_up(value: u64, align: u64) -> u64 {
    value.div_ceil(align) * align
}

/// The bases of the plugin DLLs in the order of `sizes`: each one is the next 64 KiB after the one before (the first after `static_end`).
pub fn assign_bases(static_end: u64, sizes: &[u64]) -> Result<Vec<u64>, String> {
    let mut cur = align_up(static_end, ALIGN);
    let mut out = Vec::new();
    for size in sizes {
        out.push(cur);
        cur += align_up(*size, ALIGN);
    }
    if cur > GLOBAL_BASE {
        return Err(format!("the plugins end at {cur:#x}, past GLOBAL_BASE {GLOBAL_BASE:#x} (the image area is full)"));
    }
    Ok(out)
}

/// (ImageBase, SizeOfImage) of a PE file (PE32 or PE32+).
pub fn pe_image(data: &[u8]) -> Result<(u64, u64), String> {
    let u = |at: usize, n: usize| -> Result<u64, String> {
        let b = data.get(at..at + n).ok_or("truncated PE header")?;
        Ok(b.iter().rev().fold(0u64, |acc, &x| (acc << 8) | x as u64))
    };
    if u(0, 2)? != 0x5a4d {
        return Err("not a PE file (no MZ)".into());
    }
    let e = u(0x3c, 4)? as usize;
    if data.get(e..e + 4) != Some(&b"PE\0\0"[..]) {
        return Err("not a PE file (no PE signature)".into());
    }
    let opt = e + 24;
    let base = match u(opt, 2)? {
        0x20b => u(opt + 24, 8)?,
        0x10b => u(opt + 28, 4)?,
        magic => return Err(format!("unknown optional header magic {magic:#x}")),
    };
    Ok((base, u(opt + 56, 4)?))
}

fn size_of_image(host_rel: &str) -> Result<u64, String> {
    let path = format!("{HOST_DIR}/{host_rel}");
    let data = fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
    pe_image(&data).map(|(_, size)| size).map_err(|e| format!("{path}: {e}"))
}

fn read_text(path: &str) -> Result<String, String> {
    let mut text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

fn fixed_rows(path: &str) -> Result<Vec<Vec<String>>, String> {
    Ok(read_text(path)?.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).map(|l| l.split('\t').map(String::from).collect()).collect())
}

/// The hook points of a DLL as VAs at its base (lists stage: these are left out of the direct list).
fn hook_vas(d: &Dll, base: u64) -> Result<Vec<u64>, String> {
    let rvas = match &d.hook {
        Hook::None => return Ok(Vec::new()),
        Hook::Existing => existing_hook_rvas()?,
        Hook::Rva(v) => v.clone(),
    };
    Ok(rvas.into_iter().map(|r| base + r).collect())
}

fn existing_hook_rvas() -> Result<Vec<u64>, String> {
    rvas_of_hook_file(&read_text(HOOK_EXISTING)?).map_err(|e| format!("{HOOK_EXISTING}: {e}"))
}

/// The RVAs of a hook file (its `rva:` lines; exactly three).
fn rvas_of_hook_file(text: &str) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("rva:") {
            out.push(hex(rest.split_whitespace().next().unwrap_or(""))?);
        }
    }
    if out.len() != 3 {
        return Err(format!("{} rva lines, want 3", out.len()));
    }
    Ok(out)
}

/// The hook file of a DLL for fe-lift (`--hook-points`): the shared one, or one written for the plugin's own RVAs.
fn hook_file(d: &Dll) -> Result<Option<String>, String> {
    match &d.hook {
        Hook::None => Ok(None),
        Hook::Existing => Ok(Some(HOOK_EXISTING.to_string())),
        Hook::Rva(v) => {
            fs::create_dir_all(format!("{OUT_DIR}/hooks")).map_err(|e| format!("{OUT_DIR}/hooks: {e}"))?;
            let path = format!("{OUT_DIR}/hooks/{}.txt", d.key);
            let text: String = v.iter().map(|r| format!("rva:0x{r:x}\n")).collect();
            fs::write(&path, text).map_err(|e| format!("{path}: {e}"))?;
            Ok(Some(path))
        }
    }
}

/// The module row (modules.tsv, gen-all): realmod name, file (under the host), base, lift arguments, path suffix.
pub fn module_row(d: &Dll, base: u64, hook: Option<&str>) -> String {
    let mut args = d.args.clone();
    if let Some(h) = hook {
        args.push("--hook-points".into());
        args.push(h.into());
    }
    args.push("--extra-imports".into());
    args.push(EXTRA_IMPORTS.into());
    format!("{}\t{}\t0x{:x}\t{}\t{}", d.name(), d.host_file(), base, args.join(" "), d.suffix())
}

pub fn hyb_row(d: &Dll, base: u64) -> String {
    format!("{}\t{}\t{}", d.key, d.rebased(base), d.args.join(" "))
}

pub fn extract_row(d: &Dll) -> String {
    format!("{}\t{}\t{}", d.installer_name(), d.host_file(), d.sha256)
}

pub fn seed_row(d: &Dll) -> String {
    format!("{}\t{}\t{}", d.guest_path(), d.seed_file(), d.sha256)
}

/// pack.args: the value of --args gets the DLLs at their bases; the DLL lazy files, their seed files and their separate modules are appended.
pub fn pack_args(fixed: &str, dlls: &[(&Dll, u64)]) -> Result<String, String> {
    let mut lines: Vec<String> = fixed.lines().map(String::from).collect();
    let at = lines.iter().position(|l| l == "--args").ok_or("pack.args has no --args line")?;
    let value = lines.get_mut(at + 1).ok_or("pack.args: --args has no value")?;
    for (d, base) in dlls {
        value.push_str(&format!(":/data/{}", d.rebased(*base)));
    }
    for (d, base) in dlls {
        lines.push("--lazy-dll".into());
        lines.push(d.rebased(*base));
    }
    for (d, _) in dlls {
        let seed = d.seed_file();
        lines.push("--lazy-file".into());
        lines.push(format!("{HOST_DIR}/{seed}=seed/{seed}"));
    }
    for (d, base) in dlls {
        let wasm = format!("{}.{:x}.wasm", d.stem(), base);
        lines.push("--lazy-file".into());
        lines.push(format!("{XMOD_DIR}/{wasm}={wasm}"));
    }
    Ok(lines.join("\n") + "\n")
}

fn write(path: &str, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("{path}: {e}"))
}

/// The tables of the build, from the plugin rows and the fixed rows. `with_base` is false in the extraction (no host yet): the
/// extraction table and the seed table do not depend on the bases, and the lists are left alone. With the bases, lists/ gets the
/// fixed lists again (the lists stage then writes the plugins' lists, so no list of a removed plugin is left behind).
pub fn generate(with_base: bool) -> Result<(), String> {
    let dlls = read_plugins()?;
    fs::create_dir_all(OUT_DIR).map_err(|e| format!("{OUT_DIR}: {e}"))?;
    let mut extract = read_text(FIXED_EXTRACT)?;
    let mut seed = read_text(FIXED_SEED)?;
    let mut seed_args = read_text(FIXED_SEED_ARGS)?;
    for d in &dlls {
        extract.push_str(&format!("{}\n", extract_row(d)));
        seed.push_str(&format!("{}\n", seed_row(d)));
        seed_args.push_str(&format!("--lazy\n{}\n", d.seed_file()));
    }
    write(&format!("{OUT_DIR}/extract-dlls.tsv"), &extract)?;
    write(&format!("{OUT_DIR}/vfs-seed-idle1.tsv"), &seed)?;
    write(&format!("{OUT_DIR}/seed.args"), &seed_args)?;
    if !with_base {
        return Ok(());
    }
    let lists = format!("{OUT_DIR}/lists");
    let _ = fs::remove_dir_all(&lists);
    fs::create_dir_all(&lists).map_err(|e| format!("{lists}: {e}"))?;
    for entry in fs::read_dir(FIXED_LISTS).map_err(|e| format!("{FIXED_LISTS}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("lowered-") && name.ends_with(".txt") {
            fs::copy(entry.path(), format!("{lists}/{name}")).map_err(|e| format!("{name}: {e}"))?;
        }
    }

    let fixed = fixed_rows(FIXED_MODULES)?;
    let (fe_base, fe_size) = pe_image(&fs::read(format!("{HOST_DIR}/{FLENGINE_PE}")).map_err(|e| format!("{FLENGINE_PE}: {e}"))?)?;
    let mut static_end = fe_base + fe_size;
    let mut fixed_names = BTreeSet::new();
    for row in &fixed {
        if row.len() != 5 {
            return Err(format!("{FIXED_MODULES}: a row of {} fields, want 5", row.len()));
        }
        let base = hex(&row[2])?;
        static_end = static_end.max(base + align_up(size_of_image(&row[1])?, ALIGN));
        fixed_names.insert(row[0].clone());
    }
    let sizes = dlls.iter().map(|d| size_of_image(&d.host_file())).collect::<Result<Vec<u64>, String>>()?;
    let bases = assign_bases(static_end, &sizes)?;
    for d in &dlls {
        if fixed_names.contains(&d.name()) {
            return Err(format!("plugin {} ({}): the file name {} is a fixed module's", d.plugin, d.key, d.name()));
        }
    }
    let mut limit = usize::MAX;
    for (path, needle) in LIMITS {
        let n = limit_of(path, needle)?;
        limit = limit.min(n);
    }
    if fixed.len() + dlls.len() > limit {
        return Err(format!("{} modules, the runtime holds {limit} (the limits of {LIMITS:?})", fixed.len() + dlls.len()));
    }

    let pairs: Vec<(&Dll, u64)> = dlls.iter().zip(bases.iter().copied()).collect();
    let mut modules = read_text(FIXED_MODULES)?;
    let mut hyb = read_text(FIXED_HYB_ARGS)?;
    let mut plan = String::new();
    for (d, base) in &pairs {
        let hook = hook_file(d)?;
        modules.push_str(&format!("{}\n", module_row(d, *base, hook.as_deref())));
        hyb.push_str(&format!("{}\n", hyb_row(d, *base)));
        let hooks = vas_text(&hook_vas(d, *base)?);
        let omit = vas_text(&d.omit.iter().map(|r| base + r).collect::<Vec<u64>>());
        plan.push_str(&format!("{}\t{}\t{}\t{}\t0x{:x}\t{}\t{hooks}\t{omit}\n", d.key, d.name(), d.rebased(*base), d.host_file(), base, d.args.join(" ")));
    }
    write(&format!("{OUT_DIR}/modules.tsv"), &modules)?;
    write(&format!("{OUT_DIR}/hyb-args.tsv"), &hyb)?;
    write(&format!("{OUT_DIR}/plugins.tsv"), &plan)?;
    let keys: Vec<&str> = dlls.iter().map(|d| d.key.as_str()).collect();
    let mut hybrid_cmd = read_text(FIXED_HYBRID_CMD)?;
    if !keys.is_empty() {
        hybrid_cmd.push_str(&format!("--separate\n{}\n", keys.join(",")));
    }
    hybrid_cmd.push_str(&format!("--separate-dir\n{XMOD_DIR}\n"));
    write(&format!("{OUT_DIR}/hybrid.cmd"), &hybrid_cmd)?;
    let mut pack = pack_args(&read_text(FIXED_PACK_ARGS)?, &pairs)?;
    if Path::new(super::OFFLINE_PAGE_SRC).is_file() {
        pack.push_str(&format!("--lazy-file\n{}={}\n", super::OFFLINE_PAGE_SRC, super::OFFLINE_PAGE_PATH));
    }
    write(&format!("{OUT_DIR}/pack.args"), &pack)?;
    println!("plugins: {} DLLs; bases {}", dlls.len(), bases.iter().map(|b| format!("0x{b:x}")).collect::<Vec<_>>().join(" "));
    Ok(())
}

fn limit_of(path: &str, needle: &str) -> Result<usize, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let line = text.lines().find(|l| l.contains(needle)).ok_or_else(|| format!("{path}: no line with {needle:?}"))?;
    let digits: String = line[line.find(needle).unwrap_or(0) + needle.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<usize>().map_err(|_| format!("{path}: no number after {needle:?}"))
}

/// The direct list of a DLL: the functions fe-wasmgen emitted ok, without the left-out VAs (the hook points and the omit VAs).
fn direct_list(ok: &BTreeSet<u64>, left_out: &BTreeSet<u64>) -> Vec<u64> {
    ok.iter().copied().filter(|v| !left_out.contains(v)).collect()
}

/// The lists stage (one plugin DLL): the DLL rebased at its base, then the fe-wasmgen report of it (the direct functions: the ones
/// that are emitted, `status` ok), without the hook points and the omit VAs. The result is lists/lowered-<key>.txt.
pub fn run_list(key: &str, bin: &str) -> Result<(), String> {
    let text = read_text(&format!("{OUT_DIR}/plugins.tsv"))?;
    let row: Vec<&str> = text.lines().map(|l| l.split('\t').collect::<Vec<&str>>()).find(|f| f[0] == key).ok_or_else(|| format!("{key}: not in {OUT_DIR}/plugins.tsv"))?;
    let (rebased_name, host, base_hex, args, hooks, omit) = (row[2], row[3], row[4], row[5], row[6], row[7]);
    let work = format!("{OUT_DIR}/work");
    fs::create_dir_all(&work).map_err(|e| format!("{work}: {e}"))?;
    let rebased = format!("{work}/{rebased_name}");
    let _ = fs::remove_file(&rebased);
    run_cmd(&[format!("{bin}/fe-pe"), "rebase".into(), format!("{HOST_DIR}/{host}"), base_hex.into(), rebased.clone()], "fe-pe rebase")?;
    let wasm = format!("{work}/{key}.wasm");
    let report = format!("{work}/{key}.tsv");
    let mut argv = vec![format!("{bin}/fe-wasmgen"), rebased, wasm];
    argv.extend(args.split_whitespace().map(String::from));
    argv.extend(fs::read_to_string(N0_ARGS).map_err(|e| format!("{N0_ARGS}: {e}"))?.split_whitespace().map(String::from));
    argv.extend(["--report".to_string(), report.clone()]);
    run_cmd(&argv, "fe-wasmgen")?;
    let ok = report_ok(&fs::read_to_string(&report).map_err(|e| format!("{report}: {e}"))?)?;
    let omit = vas_of(omit)?;
    if let Some(va) = omit.iter().find(|v| !ok.contains(v)) {
        return Err(format!("{key}: the omit VA {va:#x} is not emitted ok (the omit column of plugin.tsv does not name a direct function)"));
    }
    let mut left_out = vas_of(hooks)?;
    left_out.extend(omit.iter().copied());
    let listed = direct_list(&ok, &left_out);
    if listed.is_empty() {
        return Err(format!("{key}: fe-wasmgen emitted no function"));
    }
    let out: String = listed.iter().map(|v| format!("0x{v:x}\n")).collect();
    let path = format!("{OUT_DIR}/lists/lowered-{key}.txt");
    write(&path, &out)?;
    println!("list {key}: emitted ok {}, left out {} (omit {}), listed {}", ok.len(), ok.iter().filter(|v| left_out.contains(v)).count(), omit.len(), listed.len());
    Ok(())
}

/// The VAs of the functions a fe-wasmgen report shows as emitted (column 8 is the status; the first column the VA).
fn report_ok(text: &str) -> Result<BTreeSet<u64>, String> {
    let mut ok = BTreeSet::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f[0] == "va" || f.len() < 8 {
            continue;
        }
        if f[7] == "ok" {
            ok.insert(hex(f[0])?);
        }
    }
    Ok(ok)
}

/// The union of the lists in lists/ (the fixed ones and the plugins'): the omit list of gen-all (lowered-all.txt).
pub fn merge_lists() -> Result<(), String> {
    let lists = format!("{OUT_DIR}/lists");
    let mut all = BTreeMap::new();
    for entry in fs::read_dir(&lists).map_err(|e| format!("{lists}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let text = fs::read_to_string(entry.path()).map_err(|e| format!("{}: {e}", entry.path().display()))?;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let va = hex(line.split_whitespace().next().unwrap_or(""))?;
            all.insert(va, ());
        }
    }
    let out: String = all.keys().map(|v| format!("0x{v:x}\n")).collect();
    write(&format!("{OUT_DIR}/lowered-all.txt"), &out)?;
    println!("lowered-all: {} VAs", all.len());
    Ok(())
}

/// The units the lift wrote (lift/out-app/gen/src/*.c) must be the ones of the compile lists: a unit that is not named is not built.
pub fn check_units(listed: &[String]) -> Result<(), String> {
    let mut found = BTreeSet::new();
    for entry in fs::read_dir("lift/out-app/gen/src").map_err(|e| format!("lift/out-app/gen/src: {e}"))? {
        let name = entry.map_err(|e| e.to_string())?.file_name().to_string_lossy().into_owned();
        if name.ends_with(".c") && name != "common.c" {
            found.insert(name);
        }
    }
    let want: BTreeSet<String> = listed.iter().map(|p| Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).collect();
    if found == want {
        Ok(())
    } else {
        let missing: Vec<&String> = found.difference(&want).collect();
        let absent: Vec<&String> = want.difference(&found).collect();
        Err(format!("the units of gen differ from the compile lists: not listed {missing:?}, not written {absent:?}"))
    }
}

fn run_cmd(argv: &[String], label: &str) -> Result<(), String> {
    let status = Command::new(&argv[0]).args(&argv[1..]).status().map_err(|e| format!("{label}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label}: {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BALANCE: &str = include_str!("../../../plugins/fruity-balance/plugin.tsv");

    fn balance() -> Dll {
        parse_plugin("fruity-balance", BALANCE).unwrap().remove(0)
    }

    #[test]
    fn the_balance_rows_are_the_ones_the_tables_had() {
        let d = balance();
        assert_eq!(d.name(), "effects.fruity-balance.fruity_balance_x64.dll");
        assert_eq!(d.installer_name(), "fruity balance_x64.dll");
        assert_eq!(extract_row(&d), "fruity balance_x64.dll\tlift/out-plugins/effects.fruity-balance.fruity_balance_x64.dll\ta399cbe060ec3002a920f62fdcc2a2fb36bb1f3918281e01cdd5fc70724b8064");
        assert_eq!(d.seed_file(), "lift/out-idle1/inputs/install/Plugins/Fruity/Effects/effects.fruity-balance.fruity_balance_x64.dll");
        assert_eq!(seed_row(&d), "C:\\Program Files\\Image-Line\\FL Studio 21\\Plugins\\Fruity\\Effects\\Fruity Balance\\Fruity Balance_x64.dll\tlift/out-idle1/inputs/install/Plugins/Fruity/Effects/effects.fruity-balance.fruity_balance_x64.dll\ta399cbe060ec3002a920f62fdcc2a2fb36bb1f3918281e01cdd5fc70724b8064");
        assert_eq!(d.suffix(), "plugins/fruity/effects/fruity balance/fruity balance_x64.dll");
        assert_eq!(module_row(&d, 0x6800000, Some("lift/harness/hooks/fruity-delphi-effect-hook-points.txt")), "effects.fruity-balance.fruity_balance_x64.dll\tlift/out-plugins/effects.fruity-balance.fruity_balance_x64.dll\t0x6800000\t--pdata --hook-points lift/harness/hooks/fruity-delphi-effect-hook-points.txt --extra-imports lift/harness/seeds/ship6-imports.txt\tplugins/fruity/effects/fruity balance/fruity balance_x64.dll");
        assert_eq!(hyb_row(&d, 0x6800000), format!("{}\teffects.fruity-balance.fruity_balance_x64.6800000.dll\t--pdata", d.key));
        assert_eq!(d.hook, Hook::Existing);
        assert_eq!(d.omit, vec![0x2c13a0]);
    }

    #[test]
    fn the_hook_points_of_balance_are_the_three_of_the_delphi_runtime() {
        let rvas = rvas_of_hook_file(include_str!("../../../harness/hooks/fruity-delphi-effect-hook-points.txt")).unwrap();
        assert_eq!(rvas.iter().map(|r| 0x6800000 + r).collect::<Vec<u64>>(), vec![0x6817bc0, 0x6831d70, 0x6831d90]);
    }

    #[test]
    fn fixture_plugins_read_in_key_order_and_their_bases_follow_the_fixed_end() {
        let dlls = read_plugins_from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/plugins")).unwrap();
        assert_eq!(dlls.iter().map(|d| d.key.as_str()).collect::<Vec<_>>(), ["effects.dummy-one.dummy_one_x64.dll", "effects.dummy-two.dummy_two_x64.dll", "generators.dummy-two.engine_x64.dll"]);
        assert_eq!(dlls[2].hook, Hook::Rva(vec![0x17bc0, 0x31d70, 0x31d90]));
        assert_eq!(dlls[2].args, vec!["--pdata".to_string()]);
        assert_eq!(dlls[2].omit, vec![0x5000]);
        assert!(dlls[0].omit.is_empty());
        assert_eq!(dlls[0].hook, Hook::None);
        let bases = assign_bases(0x7B8F200, &[0x3A1000, 0x10001, 0x20000]).unwrap();
        assert_eq!(bases, vec![0x7B90000, 0x7F40000, 0x7F60000]);
    }

    #[test]
    fn bad_rows_stop_the_plugins() {
        let good = "key\tguest\trole\tsha256\tsize\thook\targs\tomit\n";
        let row = |k: &str, role: &str, sha: &str, size: &str, hook: &str| format!("{good}{k}\tPlugins\\Fruity\\Effects\\X\\X_x64.dll\t{role}\t{sha}\t{size}\t{hook}\t\t-\n");
        let sha = "a".repeat(64);
        assert!(parse_plugin("x", &row("a.b", "本体", &sha, "10", "none")).is_ok());
        assert!(parse_plugin("x", &row("a.b", "主", &sha, "10", "none")).is_err());
        assert!(parse_plugin("x", &row("a.b", "本体", &sha, "0", "none")).is_err());
        assert!(parse_plugin("x", &row("a.b", "本体", &sha, "10", "rva:0x1,0x2")).is_err());
        assert!(parse_plugin("x", &row("a.b", "本体", &sha, "10", "maybe")).is_err());
        let twice = parse_plugin("x", &row("a.b", "本体", &sha, "10", "none")).unwrap();
        let mut both = twice.clone();
        both.extend(twice);
        assert!(validate(&both).is_err());
        assert!(validate(&parse_plugin("x", &row("A.b", "本体", &sha, "10", "none")).unwrap()).is_err());
        assert!(validate(&parse_plugin("x", &row("a.b", "本体", "XY", "10", "none")).unwrap()).is_err());
    }

    #[test]
    fn the_omit_column_is_rvas_or_a_dash_and_the_row_has_eight_fields() {
        let row = |omit: &str| format!("key\tguest\trole\tsha256\tsize\thook\targs\tomit\na.b\tPlugins\\Fruity\\Effects\\X\\X_x64.dll\t本体\t{}\t10\tnone\t\t{omit}\n", "a".repeat(64));
        assert_eq!(parse_plugin("x", &row("0x2c13a0,0x10")).unwrap()[0].omit, vec![0x2c13a0, 0x10]);
        assert!(parse_plugin("x", &row("-")).unwrap()[0].omit.is_empty());
        assert!(parse_plugin("x", &row("")).is_err());
        assert!(parse_plugin("x", &row("zz")).is_err());
        let seven = format!("key\tguest\trole\tsha256\tsize\thook\targs\tomit\na.b\tPlugins\\Fruity\\Effects\\X\\X_x64.dll\t本体\t{}\t10\tnone\t\n", "a".repeat(64));
        assert!(parse_plugin("x", &seven).is_err());
        let no_column = format!("key\tguest\trole\tsha256\tsize\thook\targs\na.b\tPlugins\\Fruity\\Effects\\X\\X_x64.dll\t本体\t{}\t10\tnone\t\t-\n", "a".repeat(64));
        assert!(parse_plugin("x", &no_column).is_err());
    }

    #[test]
    fn the_omit_vas_leave_the_direct_list() {
        let ok: BTreeSet<u64> = [0x10, 0x20, 0x30].into_iter().collect();
        let left_out = vas_of("0x20,0x99").unwrap();
        assert_eq!(direct_list(&ok, &left_out), vec![0x10, 0x30]);
        assert_eq!(direct_list(&ok, &vas_of("-").unwrap()), vec![0x10, 0x20, 0x30]);
        assert_eq!(vas_text(&[0x2c13a0, 0x6ac13a0]), "0x2c13a0,0x6ac13a0");
        assert_eq!(vas_text(&[]), "-");
    }

    #[test]
    fn the_global_base_is_the_one_of_the_link() {
        let link = include_str!("../../../harness/web/link.rsp");
        assert!(link.lines().any(|l| l == format!("-sGLOBAL_BASE={GLOBAL_BASE}")), "link.rsp must have -sGLOBAL_BASE={GLOBAL_BASE}");
    }

    /// The second wave (Issue #1379, tools/plugin-layout/wave2-plugins.mjs): 160 DLLs of the plan, with the first wave's Balance. Their file names are
    /// not unique, and their keys are. With SizeOfImage (inputs/plugin-image-sizes.tsv, by SHA-256) they fit the image area: each base is 64 KiB
    /// aligned, after the one before it by that one's size, from Balance's base (0xC1D0000, after oggio's end 0xC1CF000) to below GLOBAL_BASE.
    #[test]
    fn the_second_wave_is_160_dlls_and_fits_the_image_area() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../.tmp-wave2-plugins");
        let tool = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tools/plugin-layout/wave2-plugins.mjs");
        assert!(Command::new("node").args([tool, dir]).status().expect("node").success());
        let wave = read_rows_from(dir);
        let _ = fs::remove_dir_all(dir);
        let wave = wave.unwrap();
        assert_eq!(wave.len(), 160);
        assert_eq!(wave.iter().filter(|d| d.main).count(), 132);
        assert_eq!(wave.iter().map(|d| d.size).sum::<u64>(), 608_521_016);
        let mut dlls = read_rows_from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins")).unwrap();
        dlls.extend(wave);
        dlls.sort_by(|a, b| a.key.cmp(&b.key));
        validate(&dlls).unwrap();
        let sizes_text = include_str!("../../../../inputs/plugin-image-sizes.tsv");
        let image_of = |d: &Dll| -> u64 {
            let line = sizes_text.lines().find(|l| l.split('\t').next() == Some(d.sha256.as_str())).unwrap_or_else(|| panic!("{}: no SizeOfImage row", d.key));
            line.split('\t').nth(2).unwrap().parse().unwrap()
        };
        let sizes: Vec<u64> = dlls.iter().map(image_of).collect();
        let bases = assign_bases(0xC1CF000, &sizes).expect("the images fit below GLOBAL_BASE");
        assert_eq!(bases[0], 0xC1D0000);
        let wave_total: u64 = dlls.iter().filter(|d| d.plugin != "fruity-balance").map(|d| align_up(image_of(d), ALIGN)).sum();
        assert_eq!(wave_total, 748_158_976);
        for pair in bases.windows(2).zip(sizes.iter()) {
            assert!(pair.0[1] >= pair.0[0] + align_up(*pair.1, ALIGN), "the images at {:#x} and {:#x} overlap", pair.0[0], pair.0[1]);
        }
        assert!(bases.iter().all(|b| b % ALIGN == 0));
        assert!(*bases.last().unwrap() + align_up(*sizes.last().unwrap(), ALIGN) <= GLOBAL_BASE);
    }

    #[test]
    fn a_pe_header_gives_its_base_and_size() {
        let mut data = vec![0u8; 0x200];
        data[0] = b'M';
        data[1] = b'Z';
        data[0x3c] = 0x80;
        data[0x80..0x84].copy_from_slice(b"PE\0\0");
        data[0x80 + 24..0x80 + 26].copy_from_slice(&0x20bu16.to_le_bytes());
        data[0x80 + 24 + 24..0x80 + 24 + 32].copy_from_slice(&0x6800000u64.to_le_bytes());
        data[0x80 + 24 + 56..0x80 + 24 + 60].copy_from_slice(&0x3a1000u32.to_le_bytes());
        assert_eq!(pe_image(&data).unwrap(), (0x6800000, 0x3a1000));
    }
}
