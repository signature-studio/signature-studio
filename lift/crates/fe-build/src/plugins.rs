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
/// The sources and the objects of the plugins' units (gen's units that the fixed lists do not name), one per line: written by the units step.
pub const EXTRA_UNITS: &str = "lift/out-app/plugins/extra-units.txt";
pub const EXTRA_OBJS: &str = "lift/out-app/plugins/extra-objs.txt";
const HOST_DIR: &str = "lift/out-app/host";
const GUEST_ROOT: &str = "C:\\Program Files\\Image-Line\\FL Studio 21\\";
const SEED_ROOT: &str = "lift/out-idle1/inputs/install";
const EXTRA_IMPORTS: &str = "lift/harness/seeds/ship6-imports.txt";
const HOOK_EXISTING: &str = "lift/harness/hooks/fruity-delphi-effect-hook-points.txt";
const XMOD_DIR: &str = "lift/out-app/xmod";
const N0_ARGS: &str = "lift/harness/hyb/n0.args";
const ALIGN: u64 = 0x1_0000;
/// The end of the image area (Issue #1379): Balance's base 0xC1D0000 + 750 MiB, which holds Balance, Filter (Issue #1370) and the 160 second-wave images (748,158,976 B of
/// SizeOfImage, with 5% to spare). Must be the -sGLOBAL_BASE of link.rsp (the test the_global_base_is_the_one_of_the_link).
const GLOBAL_BASE: u64 = 0x3AFD_0000;
const FLENGINE_PE: &str = "binaries/win64/FLEngine_x64.dll";
/// The sizes that bound the number of modules (realmods): the runtime's table, the handles of the cursor, and the harness's arrays.
const LIMITS: [(&str, &str); 3] = [("lift/runtime/fle_realmod.c", "#define RM_MAX "), ("lift/runtime/fle_cursor.c", "#define SHARED_MODULE_REALMODS "), ("lift/harness/two/main_two.c", "MAIN_REAL_MAX = ")];
const FIXED_EXTRACT: &str = "ci/extract-dlls.tsv";
const FIXED_SEED: &str = "lift/harness/seeds/vfs-seed-idle1.tsv";
/// The Plugin database of the user's data (Issue #1484): the Browser lists the entries of its Installed folder. One `<name>.nfo` per built-in plugin
/// is put there from the plugins' rows; the files are made in the host tree (`PLUGIN_DB_HOST`, under HOST_DIR) for the seed pack.
const PLUGIN_DB_GUEST: &str = "C:\\Users\\user\\Documents\\Image-Line\\FL Studio\\Presets\\Plugin database\\Installed\\";
const PLUGIN_DB_HOST: &str = "lift/out-plugindb";
const FIXED_SEED_ARGS: &str = "lift/harness/web/seed.args";
const FIXED_MODULES: &str = "lift/harness/b2d2/modules.tsv";
const FIXED_HYB_ARGS: &str = "lift/harness/hyb/hyb-args.tsv";
const FIXED_HYBRID_CMD: &str = "lift/harness/hyb/hybrid.cmd";
const FIXED_PACK_ARGS: &str = "lift/harness/web/pack.args";
const FIXED_LISTS: &str = "lift/harness/hyb/lists";
const INSTALLER_LIST: &str = "inputs/fl-installer-files.tsv";
const COLUMNS: [&str; 8] = ["key", "guest", "role", "sha256", "size", "hook", "args", "omit"];
const DATA_COLUMNS: [&str; 4] = ["guest", "installer", "sha256", "size"];
const LAZY_FILE: &str = "@lazy-file:";

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
    /// RVAs that are left out of the direct list (the C part lifts them), added by hand to the ones found by itself: the functions that call
    /// the guest's setjmp (`setjmp-entries` of fe-decode, Issue #1370). Such a caller registers the landing in C.
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

    /// the file of the seed list (relative to HOST_DIR): the folders of the DLL's path above its own folder, and the name (the key, unique).
    /// A `Shared\file` DLL has no folder above its own: its seed file is directly in the install folder.
    pub fn seed_file(&self) -> String {
        let parts: Vec<&str> = self.guest.split('\\').collect();
        let mut path = vec![SEED_ROOT.to_string()];
        path.extend(parts[..parts.len() - 2].iter().map(|p| p.to_string()));
        path.push(self.name());
        path.join("/")
    }

    /// the lower-case path suffix that LoadLibrary of the full path is matched with (fle_realmod_find_path)
    pub fn suffix(&self) -> String {
        self.guest.replace('\\', "/").to_lowercase()
    }

    /// A program (the installer's file is an .exe: the Plugin Manager, Issue #1499), not a DLL. It is lifted as a module of its own, like a DLL. Its VFS
    /// row is the fixed seed table's (a lazy file of the resources), so the seed rows of the plugins leave it out, and its folder has no 本体 row
    /// (no database entry). Its key still ends with .dll: the engine finds a realmod by the rebased file's name without the base.
    pub fn is_exe(&self) -> bool {
        self.basename().to_lowercase().ends_with(".exe")
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

/// A row of `lift/plugins/<name>/data.tsv` (Issue #1478): a file of the plugin's Data folder, which the page serves after the engine has started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Data {
    pub plugin: String,
    /// the path in the installer, without the install folder: `Plugins\Fruity\Generators\Sytrus\Data\Default.dat`
    pub guest: String,
    /// the same spelling as the row of inputs/fl-installer-files.tsv (the installer path; the same as guest, case-insensitive)
    pub installer: String,
    pub sha256: String,
    pub size: u64,
}

impl Data {
    pub fn guest_path(&self) -> String {
        format!("{GUEST_ROOT}{}", self.guest)
    }

    /// the host file, relative to the host tree: the guest path with `/` for `\` and `_` for a space (the names of #1414)
    pub fn host_file(&self) -> String {
        format!("plugins/{}", self.guest.replace('\\', "/").replace(' ', "_"))
    }

    /// the seed row of the file: the guest path, the host field with `@lazy-file:`, the SHA-256
    pub fn seed_row(&self) -> String {
        format!("{}\t{LAZY_FILE}{}\t{}", self.guest_path(), self.host_file(), self.sha256)
    }
}

/// `lift/plugins/<name>/data.tsv`: the header, then the rows of one plugin's Data (`#` lines are notes).
pub fn parse_data(plugin: &str, text: &str) -> Result<Vec<Data>, String> {
    let mut out = Vec::new();
    let mut header = false;
    for (n, line) in text.lines().enumerate() {
        let at = |m: &str| format!("{plugin}/data.tsv line {}: {m}", n + 1);
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if !header {
            if f.as_slice() != DATA_COLUMNS {
                return Err(at("the header must be guest, installer, sha256, size (TAB separated)"));
            }
            header = true;
            continue;
        }
        if f.len() != 4 {
            return Err(at(&format!("{} fields, want 4", f.len())));
        }
        if !f[3].bytes().all(|b| b.is_ascii_digit()) {
            return Err(at("size must be a positive integer"));
        }
        let size = f[3].parse::<u64>().ok().filter(|v| *v > 0).ok_or_else(|| at("size must be a positive integer"))?;
        out.push(Data { plugin: plugin.to_string(), guest: f[0].to_string(), installer: f[1].to_string(), sha256: f[2].to_string(), size });
    }
    if !header {
        return Err(format!("{plugin}/data.tsv: no header"));
    }
    Ok(out)
}

/// inputs/fl-installer-files.tsv: the installer path → (size, SHA-256) of each row.
pub fn installer_list(path: &str) -> Result<BTreeMap<String, (u64, String)>, String> {
    let mut out = BTreeMap::new();
    for line in read_text(path)?.lines() {
        if line.trim().is_empty() || line.starts_with('#') || line.starts_with("guest\t") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 3 {
            return Err(format!("{path}: a row of {} fields", f.len()));
        }
        out.insert(f[0].to_string(), (f[1].parse::<u64>().unwrap_or(0), f[2].to_string()));
    }
    Ok(out)
}

/// The checks of the Data rows (Issue #1478; plugins.rs and extract-fl.mjs read the same rows): the guest path is the folder's own Data
/// (`Plugins\Fruity\<kind>\<folder>\Data\...`), it is not a DLL and not a DLL row's, and its installer row is in the list with the same
/// SHA-256 and size. The guest paths and the host files are unique.
pub fn validate_data(data: &[Data], dlls: &[Dll], list: &BTreeMap<String, (u64, String)>) -> Result<(), String> {
    let mut guests: BTreeSet<String> = dlls.iter().map(|d| d.guest.to_lowercase()).collect();
    let mut hosts = BTreeSet::new();
    for d in data {
        let at = |m: &str| format!("plugin {} ({}): {m}", d.plugin, d.guest);
        let parts: Vec<&str> = d.guest.split('\\').collect();
        let own = parts.len() >= 6 && parts[0] == "Plugins" && parts[1] == "Fruity" && matches!(parts[2], "Effects" | "Generators") && parts[4] == "Data" && !d.guest.contains('/');
        if !own {
            return Err(at("the guest path is Plugins\\Fruity\\<kind>\\<folder>\\Data\\... (the folder's own Data)"));
        }
        if d.guest.to_lowercase().ends_with(".dll") {
            return Err(at("a DLL is a row of plugin.tsv, not of data.tsv"));
        }
        let Some((size, sha)) = list.get(&d.installer) else {
            return Err(at(&format!("installer {}: not a row of {INSTALLER_LIST}", d.installer)));
        };
        if d.installer.to_lowercase() != d.guest.to_lowercase() {
            return Err(at("the installer path is the guest path (the same spelling, case-insensitive)"));
        }
        if d.sha256.len() != 64 || !d.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(at("sha256 is 64 lower case hex digits"));
        }
        if *sha != d.sha256 {
            return Err(at("sha256 is not the one of the installer list"));
        }
        if *size != d.size {
            return Err(at(&format!("size {}: the installer list says {size}", d.size)));
        }
        if !guests.insert(d.guest.to_lowercase()) {
            return Err(at("the guest path is not unique (a DLL or another Data row has it)"));
        }
        if !hosts.insert(d.host_file()) {
            return Err(at("the host file is not unique (a space is _)"));
        }
    }
    Ok(())
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
        let parts = d.guest.split('\\').count();
        if (parts < 3 && !(parts == 2 && d.guest.starts_with("Shared\\"))) || d.guest.contains('/') {
            return Err(at("the guest path is Plugins\\...\\file (backslashes, at least three parts) or Shared\\file (Issue #1429)"));
        }
        if !guests.insert(d.guest.to_lowercase()) {
            return Err(at("the guest path is not unique"));
        }
        let file = d.basename().to_lowercase();
        if !file.ends_with(".dll") && !file.ends_with(".exe") {
            return Err(at("the file is not a DLL or an .exe"));
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

/// The folders of a plugins folder, sorted: each one is a plugin, and its name is lower case letters, digits and '-' only.
fn plugin_folders(dir: &str) -> Result<Vec<String>, String> {
    let mut folders = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("{dir}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            folders.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    folders.sort();
    if let Some(folder) = folders.iter().find(|f| !valid_folder(f)) {
        return Err(format!("{dir}/{folder}: the folder name is lower case letters, digits and '-' only"));
    }
    Ok(folders)
}

/// The plugin rows of a plugins folder, sorted by key, without the checks of `validate` (read_plugins_from adds them).
pub fn read_rows_from(dir: &str) -> Result<Vec<Dll>, String> {
    let mut dlls = Vec::new();
    for folder in plugin_folders(dir)? {
        let path = format!("{dir}/{folder}/plugin.tsv");
        let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        dlls.extend(parse_plugin(&folder, &text)?);
    }
    dlls.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(dlls)
}

/// The Data rows of a plugins folder, sorted by guest path: the data.tsv of each folder that has one (a folder without one has no Data).
/// Without the checks of `validate_data` (read_data adds them).
pub fn read_data_from(dir: &str) -> Result<Vec<Data>, String> {
    let mut out = Vec::new();
    for folder in plugin_folders(dir)? {
        let path = format!("{dir}/{folder}/data.tsv");
        if Path::new(&path).is_file() {
            let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            out.extend(parse_data(&folder, &text)?);
        }
    }
    out.sort_by(|a, b| a.guest.cmp(&b.guest));
    Ok(out)
}

/// `lift/plugins/<name>/seeds.tsv` (Issue #1429): a function of a DLL that the DLL reaches only through a pointer in its data and that has no
/// `.pdata` entry, so the lift does not find it. A row is the DLL's key and the RVA of the function; the lift of the DLL takes it as a seed (see generate).
pub fn parse_seeds(plugin: &str, text: &str) -> Result<Vec<(String, u64)>, String> {
    let mut out = Vec::new();
    let mut header = false;
    for (n, line) in text.lines().enumerate() {
        let at = |m: &str| format!("{plugin}/seeds.tsv line {}: {m}", n + 1);
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if !header {
            if f != ["key", "rva"] {
                return Err(at("the header must be key, rva (TAB separated)"));
            }
            header = true;
            continue;
        }
        if f.len() != 2 {
            return Err(at(&format!("{} fields, want 2", f.len())));
        }
        out.push((f[0].to_string(), hex(f[1]).map_err(|e| at(&e))?));
    }
    if !header {
        return Err(format!("{plugin}/seeds.tsv: no header"));
    }
    Ok(out)
}

/// The seed rows of a plugins folder: the seeds.tsv of each folder that has one (a folder without one has no seeds).
pub fn read_seeds_from(dir: &str) -> Result<Vec<(String, u64)>, String> {
    let mut out = Vec::new();
    for folder in plugin_folders(dir)? {
        let path = format!("{dir}/{folder}/seeds.tsv");
        if Path::new(&path).is_file() {
            let text = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            out.extend(parse_seeds(&folder, &text)?);
        }
    }
    Ok(out)
}

pub fn read_plugins() -> Result<Vec<Dll>, String> {
    read_plugins_from(PLUGINS_DIR)
}

/// The Data rows of lift/plugins, after the checks against the installer list (validate_data).
pub fn read_data(dlls: &[Dll]) -> Result<Vec<Data>, String> {
    let data = read_data_from(PLUGINS_DIR)?;
    validate_data(&data, dlls, &installer_list(INSTALLER_LIST)?)?;
    Ok(data)
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

/// The names of the Plugin database's entries: one per plugin, the folder of its main DLL in the installer (`Plugins\Fruity\Effects\Fruity Balance\...`
/// gives `Fruity Balance`). A plugin has one main DLL, and the names are unique and usable as file names. A folder that has only helpers (補助,
/// e.g. `Shared\Reverb_x64.dll`, a library that a plugin loads) is no plugin: it has no entry (Issue #1429).
pub fn plugin_db_names(dlls: &[Dll]) -> Result<Vec<String>, String> {
    let mut plugins: BTreeMap<&str, Vec<&Dll>> = BTreeMap::new();
    for d in dlls {
        plugins.entry(d.plugin.as_str()).or_default().push(d);
    }
    let mut names = BTreeSet::new();
    for (plugin, rows) in &plugins {
        let mains: Vec<&&Dll> = rows.iter().filter(|d| d.main).collect();
        if mains.is_empty() {
            continue;
        }
        if mains.len() != 1 {
            return Err(format!("{plugin}/plugin.tsv: {} rows of 本体, want one (the plugin's Plugin database entry is its main DLL)", mains.len()));
        }
        let parts: Vec<&str> = mains[0].guest.split('\\').collect();
        if parts.len() < 3 {
            return Err(format!("{plugin}/plugin.tsv: the main DLL {} has no folder of its own (the Plugin database entry is that folder)", mains[0].guest));
        }
        let name = parts[parts.len() - 2];
        if name.is_empty() || name.chars().any(|c| "\\/:*?\"<>|".contains(c)) || !names.insert(name.to_string()) {
            return Err(format!("{plugin}/plugin.tsv: the Plugin database name {name} is empty, not a file name, or another plugin's"));
        }
    }
    Ok(names.into_iter().collect())
}

/// The seed rows of the Plugin database entries (`PLUGIN_DB_GUEST`): each `<name>.nfo` is written to the host tree first, and its row names its SHA-256.
fn plugin_db_rows(names: &[String]) -> Result<Vec<String>, String> {
    let dir = format!("{HOST_DIR}/{PLUGIN_DB_HOST}");
    fs::create_dir_all(&dir).map_err(|e| format!("{dir}: {e}"))?;
    let mut rows = Vec::new();
    for name in names {
        let rel = format!("{PLUGIN_DB_HOST}/{name}.nfo");
        let path = format!("{HOST_DIR}/{rel}");
        write(&path, &format!("Tip={name}\r\n"))?;
        rows.push(format!("{PLUGIN_DB_GUEST}{name}.nfo\t{rel}\t{}", crate::sha256_hex(&path)?));
    }
    Ok(rows)
}

/// pack.args: the value of --args gets the DLLs at their bases; the DLL lazy files, their seed files and their separate modules are appended,
/// then the Data files of the plugins (one --lazy-file each: the host file, and its path under seed/).
pub fn pack_args(fixed: &str, dlls: &[(&Dll, u64)], data: &[Data]) -> Result<String, String> {
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
    for (d, _) in dlls.iter().filter(|(d, _)| !d.is_exe()) {
        let seed = d.seed_file();
        lines.push("--lazy-file".into());
        lines.push(format!("{HOST_DIR}/{seed}=seed/{seed}"));
    }
    for (d, base) in dlls {
        let wasm = format!("{}.{:x}.wasm", d.stem(), base);
        lines.push("--lazy-file".into());
        lines.push(format!("{XMOD_DIR}/{wasm}={wasm}"));
    }
    for d in data {
        lines.push("--lazy-file".into());
        lines.push(format!("{HOST_DIR}/{0}=seed/{0}", d.host_file()));
    }
    Ok(lines.join("\n") + "\n")
}

/// The seed table of the build: the fixed rows, then the seed row of each DLL (not of an .exe: its row is in the fixed table), then the seed row of
/// each Data file (Issue #1478).
pub fn seed_table(fixed: &str, dlls: &[Dll], data: &[Data]) -> String {
    let mut seed = fixed.to_string();
    for d in dlls.iter().filter(|d| !d.is_exe()) {
        seed.push_str(&format!("{}\n", seed_row(d)));
    }
    for d in data {
        seed.push_str(&format!("{}\n", d.seed_row()));
    }
    seed
}

fn write(path: &str, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("{path}: {e}"))
}

/// The tables of the build, from the plugin rows and the fixed rows. `with_base` is false in the extraction (no host yet): the
/// extraction table and the seed table do not depend on the bases, and the lists are left alone. With the bases, lists/ gets the
/// fixed lists again (the lists stage then writes the plugins' lists, so no list of a removed plugin is left behind).
pub fn generate(with_base: bool) -> Result<(), String> {
    let mut dlls = read_plugins()?;
    let data = read_data(&dlls)?;
    fs::create_dir_all(OUT_DIR).map_err(|e| format!("{OUT_DIR}: {e}"))?;
    let mut extract = read_text(FIXED_EXTRACT)?;
    let mut seed_args = read_text(FIXED_SEED_ARGS)?;
    for d in &dlls {
        extract.push_str(&format!("{}\n", extract_row(d)));
        if !d.is_exe() {
            seed_args.push_str(&format!("--lazy\n{}\n", d.seed_file()));
        }
    }
    write(&format!("{OUT_DIR}/extract-dlls.tsv"), &extract)?;
    let seed = seed_table(&read_text(FIXED_SEED)?, &dlls, &data);
    write(&format!("{OUT_DIR}/vfs-seed-extract.tsv"), &seed)?;
    let mut idle1 = seed;
    for row in plugin_db_rows(&plugin_db_names(&dlls)?)? {
        idle1.push_str(&format!("{row}\n"));
    }
    write(&format!("{OUT_DIR}/vfs-seed-idle1.tsv"), &idle1)?;
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

    let seeds = read_seeds_from(PLUGINS_DIR)?;
    if let Some((key, _)) = seeds.iter().find(|(k, _)| !dlls.iter().any(|d| &d.key == k)) {
        return Err(format!("seeds.tsv: the key {key} is not the key of a plugin DLL"));
    }
    for (d, base) in dlls.iter_mut().zip(bases.iter().copied()) {
        let rows: String = seeds.iter().filter(|(k, _)| *k == d.key).map(|(_, rva)| format!("0x{:x}\n", base + rva)).collect();
        if rows.is_empty() {
            continue;
        }
        fs::create_dir_all(format!("{OUT_DIR}/seeds")).map_err(|e| format!("{OUT_DIR}/seeds: {e}"))?;
        let path = format!("{OUT_DIR}/seeds/{}.txt", d.key);
        write(&path, &rows)?;
        d.args.push("--seeds".into());
        d.args.push(path);
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
    let mut pack = pack_args(&read_text(FIXED_PACK_ARGS)?, &pairs, &data)?;
    if Path::new(super::OFFLINE_PAGE_SRC).is_file() {
        pack.push_str(&format!("--lazy-file\n{}={}\n", super::OFFLINE_PAGE_SRC, super::OFFLINE_PAGE_PATH));
    }
    write(&format!("{OUT_DIR}/pack.args"), &pack)?;
    println!("plugins: {} DLLs, {} data files; bases {}", dlls.len(), data.len(), bases.iter().map(|b| format!("0x{b:x}")).collect::<Vec<_>>().join(" "));
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
    let setjmp = setjmp_entries(bin, &rebased)?;
    let wasm = format!("{work}/{key}.wasm");
    let report = format!("{work}/{key}.tsv");
    let mut argv = vec![format!("{bin}/fe-wasmgen"), rebased, wasm];
    argv.extend(args.split_whitespace().map(String::from));
    argv.extend(fs::read_to_string(N0_ARGS).map_err(|e| format!("{N0_ARGS}: {e}"))?.split_whitespace().map(String::from));
    argv.extend(["--report".to_string(), report.clone()]);
    run_cmd(&argv, "fe-wasmgen")?;
    let ok = report_ok(&fs::read_to_string(&report).map_err(|e| format!("{report}: {e}"))?)?;
    let mut omit = vas_of(omit)?;
    omit.extend(setjmp);
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

/// The units the lift wrote (lift/out-app/gen/src/*.c). Every unit of the compile lists must be written. A unit that the lists do not name
/// must be a plugin's unit (`m<K>-<NN>.c`, the module of a plugin, design (13)): it is compiled too, after the lists. Returns those units.
pub fn check_units(listed: &[String]) -> Result<Vec<String>, String> {
    let mut found = BTreeSet::new();
    for entry in fs::read_dir("lift/out-app/gen/src").map_err(|e| format!("lift/out-app/gen/src: {e}"))? {
        let name = entry.map_err(|e| e.to_string())?.file_name().to_string_lossy().into_owned();
        if name.ends_with(".c") && name != "common.c" {
            found.insert(name);
        }
    }
    check_gen_units(listed, &found)
}

/// check_units on the names of the units gen wrote (`found`: the .c files of lift/out-app/gen/src without common.c).
fn check_gen_units(listed: &[String], found: &BTreeSet<String>) -> Result<Vec<String>, String> {
    let want: BTreeSet<String> = listed.iter().map(|p| Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).collect();
    let absent: Vec<&String> = want.difference(found).collect();
    if !absent.is_empty() {
        return Err(format!("the units of the compile lists are not written by gen: {absent:?}"));
    }
    let extra: Vec<String> = found.difference(&want).cloned().collect();
    if let Some(name) = extra.iter().find(|n| !is_plugin_unit(n)) {
        return Err(format!("gen wrote {name}, which the compile lists do not name and which is not a plugin unit (m<K>-<NN>.c)"));
    }
    Ok(extra)
}

/// `m<K>-<NN>.c`: the unit NN of the module K (the modules of the plugins come after the fixed ones).
fn is_plugin_unit(name: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    name.strip_prefix('m').and_then(|r| r.strip_suffix(".c")).and_then(|r| r.split_once('-')).is_some_and(|(k, n)| digits(k) && digits(n))
}

/// The VAs of the functions that call the guest's setjmp (fe-decode setjmp-entries on the rebased DLL, the VAs of the report). Their landings are
/// registered by the C code, so they are left out of the direct list with the `omit` column (Issue #1370: the automatic part of it).
fn setjmp_entries(bin: &str, rebased: &str) -> Result<BTreeSet<u64>, String> {
    let text = run_output(&[format!("{bin}/fe-decode"), "setjmp-entries".into(), rebased.into()], "fe-decode setjmp-entries")?;
    text.lines().filter(|l| !l.trim().is_empty()).map(hex).collect()
}

fn run_output(argv: &[String], label: &str) -> Result<String, String> {
    let out = Command::new(&argv[0]).args(&argv[1..]).output().map_err(|e| format!("{label}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{label}: {}", out.status));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("{label}: {e}"))
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
    fn the_plugin_database_names_are_the_folders_of_the_main_dlls() {
        let mut dlls = vec![balance()];
        dlls.extend(parse_plugin("sytrus", include_str!("../../../plugins/sytrus/plugin.tsv")).unwrap());
        assert_eq!(plugin_db_names(&dlls).unwrap(), ["Fruity Balance", "Sytrus"]);
        assert!(plugin_db_names(&[balance(), balance()]).is_err());
    }

    #[test]
    fn the_filter_row_is_the_installer_file_and_its_entry_is_found_by_setjmp_entries() {
        let d = parse_plugin("fruity-filter", include_str!("../../../plugins/fruity-filter/plugin.tsv")).unwrap().remove(0);
        assert_eq!(d.name(), "effects.fruity-filter.fruity_filter_x64.dll");
        assert_eq!(d.installer_name(), "fruity filter_x64.dll");
        assert_eq!(d.sha256, "f4801444224236132b61983b7b81d0d3f89395085a1dc5430653a5edb7273d6c");
        assert_eq!(d.size, 3780608);
        assert_eq!(d.hook, Hook::Existing);
        assert!(d.omit.is_empty(), "the entry 0x2cdf20 comes from fe-decode setjmp-entries (run_list), not from the omit column");
    }

    #[test]
    fn the_reverb_seeds_are_distinct_rvas_of_its_key_and_the_format_is_checked() {
        let rows = parse_seeds("reverb", include_str!("../../../plugins/reverb/seeds.tsv")).unwrap();
        assert_eq!(rows.len(), 28);
        assert!(rows.iter().all(|(k, _)| k == "reverb_x64.dll"));
        assert!(rows.contains(&("reverb_x64.dll".to_string(), 0x17ef0)), "the first entry of the function table at RVA 0x628d0");
        let mut distinct: Vec<u64> = rows.iter().map(|(_, r)| *r).collect();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 28);
        assert!(parse_seeds("x", "key\tva\nreverb_x64.dll\t0x10\n").is_err());
        assert!(parse_seeds("x", "key\trva\nreverb_x64.dll\tzz\n").is_err());
        assert!(parse_seeds("x", "key\trva\nreverb_x64.dll\t0x10\textra\n").is_err());
    }

    #[test]
    fn a_shared_helper_is_no_plugin_and_has_no_plugin_database_entry() {
        let mut dlls = vec![balance()];
        dlls.extend(parse_plugin("sytrus", include_str!("../../../plugins/sytrus/plugin.tsv")).unwrap());
        dlls.extend(parse_plugin("reverb", include_str!("../../../plugins/reverb/plugin.tsv")).unwrap());
        assert_eq!(dlls[2].guest, "Shared\\Reverb_x64.dll");
        assert!(!dlls[2].main);
        assert_eq!(plugin_db_names(&dlls).unwrap(), ["Fruity Balance", "Sytrus"]);
        assert!(validate(&dlls).is_ok());
        assert_eq!(dlls[2].seed_file(), "lift/out-idle1/inputs/install/reverb_x64.dll");
        assert_eq!(dlls[2].host_file(), "lift/out-plugins/reverb_x64.dll");
        assert_eq!(dlls[2].suffix(), "shared/reverb_x64.dll");
        assert_eq!(seed_row(&dlls[2]), "C:\\Program Files\\Image-Line\\FL Studio 21\\Shared\\Reverb_x64.dll\tlift/out-idle1/inputs/install/reverb_x64.dll\t892423bcce3d516eb42d303566e8a15a23a9ae9014fbc589c79afa542019ba40");
        let mut main_in_shared = parse_plugin("reverb", include_str!("../../../plugins/reverb/plugin.tsv")).unwrap();
        main_in_shared[0].main = true;
        assert!(plugin_db_names(&main_in_shared).is_err());
    }

    #[test]
    fn an_exe_row_is_a_module_but_no_seed_row_and_no_database_entry() {
        let text = format!("key\tguest\trole\tsha256\tsize\thook\targs\tomit\nplugin-manager.pluginmanager.dll\tSystem\\Tools\\Plugin Manager\\PluginManager.exe\t補助\t{}\t7158856\tnone\t\t-\n", "a".repeat(64));
        let dlls = parse_plugin("plugin-manager", &text).unwrap();
        validate(&dlls).unwrap();
        let d = &dlls[0];
        assert!(d.is_exe());
        assert_eq!(d.rebased(0x6a00000), "plugin-manager.pluginmanager.6a00000.dll");
        assert_eq!(module_row(d, 0x6a00000, None), "plugin-manager.pluginmanager.dll\tlift/out-plugins/plugin-manager.pluginmanager.dll\t0x6a00000\t--pdata --extra-imports lift/harness/seeds/ship6-imports.txt\tsystem/tools/plugin manager/pluginmanager.exe");
        assert!(plugin_db_names(&dlls).unwrap().is_empty());
        assert_eq!(seed_table("", &dlls, &[]), "");
        let pack = pack_args("--args\n/data/FLEngine_x64.dll\n", &[(d, 0x6a00000)], &[]).unwrap();
        assert!(pack.contains("--lazy-dll\nplugin-manager.pluginmanager.6a00000.dll\n"));
        assert!(!pack.contains("seed/"));
        let wrong = text.replace("PluginManager.exe\t", "PluginManager.txt\t");
        assert!(validate(&parse_plugin("plugin-manager", &wrong).unwrap()).is_err());
        let not_dll_key = text.replace("pluginmanager.dll\t", "pluginmanager.exe\t");
        assert!(validate(&parse_plugin("plugin-manager", &not_dll_key).unwrap()).is_err());
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
    fn the_plugins_units_are_the_mk_nn_names_beyond_the_fixed_lists() {
        assert!(is_plugin_unit("m11-06.c"));
        assert!(is_plugin_unit("m10-00.c"));
        assert!(!is_plugin_unit("fle-acm.c"));
        assert!(!is_plugin_unit("m11-06.o"));
        assert!(!is_plugin_unit("mx-00.c"));
    }

    /// Issue #1525: a plugin of two DLLs (a main DLL and a helper) gets the module numbers K and K+1 after the fixed modules (the rows of
    /// lift/harness/b2d2/modules.tsv), in the order of the keys. gen writes units 00..03 of the main module and unit 00 of the helper (as the
    /// Edison run of #1435 did). The listed units are those of the fixed modules, so the check passes, and the plugin's units are the extra ones.
    #[test]
    fn the_units_of_a_two_dll_plugin_are_extra_units_and_the_listed_ones_still_stop() {
        let sha = "a".repeat(64);
        let text = format!(
            "key\tguest\trole\tsha256\tsize\thook\targs\tomit\neffects.edison.edison_x64.dll\tPlugins\\Fruity\\Effects\\Edison\\Edison_x64.dll\t本体\t{sha}\t10\tnone\t\t-\neffects.edison.testdll_x64.dll\tPlugins\\Fruity\\Effects\\Edison\\TestDll_x64.dll\t補助\t{sha}\t20\tnone\t\t-\n"
        );
        let mut keys: Vec<String> = parse_plugin("edison", &text).unwrap().into_iter().map(|d| d.key).collect();
        keys.sort();
        assert_eq!(keys, ["effects.edison.edison_x64.dll", "effects.edison.testdll_x64.dll"]);
        let fixed = include_str!("../../../harness/b2d2/modules.tsv").lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).count();
        let (main, helper) = (fixed + 1, fixed + 2);
        let listed: Vec<String> = include_str!("../../../harness/two/plain-units.txt").split_whitespace().chain(include_str!("../../../harness/two/ai-units.txt").split_whitespace()).map(String::from).collect();
        let module_of = |k: usize| format!("m{k}-");
        let mut written: BTreeSet<String> = listed
            .iter()
            .map(|p| p.rsplit('/').next().unwrap_or_default().to_string())
            .filter(|u| !u.starts_with(&module_of(main)) && !u.starts_with(&module_of(helper)))
            .collect();
        written.extend((0..4).map(|n| format!("m{main}-{n:02}.c")));
        written.insert(format!("m{helper}-00.c"));
        let extra = check_gen_units(&listed, &written).unwrap();
        let mut expected: Vec<String> = (0..4).map(|n| format!("m{main}-{n:02}.c")).chain([format!("m{helper}-00.c")]).collect();
        expected.sort();
        assert_eq!(extra, expected);
        written.remove("m1-00.c");
        assert!(check_gen_units(&listed, &written).is_err(), "a listed unit that gen does not write must still stop the build");
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

    /// A scratch folder of a test: it is removed before the test and when the test ends (also when it fails).
    struct Scratch(&'static str);

    impl Scratch {
        fn new(path: &'static str) -> Scratch {
            let _ = fs::remove_dir_all(path);
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.0);
        }
    }

    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    /// The second wave's folders as pm-work/tools/gen-wave-plugins.mjs writes them: the folder without its kind prefix (with the prefix when
    /// another folder has the same name), and the keys `<kind>.<folder>.<file>`. The table's folders are `<kind>-<folder>` with their keys
    /// `<kind>.<kind>-<folder>.<file>`, so the keys differ, and the guest paths are the same.
    fn write_wave_folders(table: &str, state: &str) {
        let folders: Vec<String> = fs::read_dir(table).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        let name_of = |folder: &str| folder.split_once('-').unwrap().1.to_string();
        for folder in &folders {
            let kind = folder.split_once('-').unwrap().0;
            let shared = folders.iter().filter(|f| name_of(f) == name_of(folder)).count() > 1;
            let dir = if shared { folder.clone() } else { name_of(folder) };
            let text = fs::read_to_string(format!("{table}/{folder}/plugin.tsv")).unwrap();
            fs::create_dir(format!("{state}/{dir}")).unwrap();
            fs::write(format!("{state}/{dir}/plugin.tsv"), gen_form(kind, &dir, &text)).unwrap();
        }
    }

    fn gen_form(kind: &str, dir: &str, text: &str) -> String {
        let mut out = String::new();
        for line in text.lines() {
            match line.split_once('\t') {
                Some((key, rest)) if key != "key" => {
                    let guest = rest.split('\t').next().unwrap_or_default();
                    let file: String = guest.rsplit('\\').next().unwrap_or_default().to_lowercase().chars().map(|c| if matches!(c, 'a'..='z' | '0'..='9' | '.' | '_' | '-') { c } else { '_' }).collect();
                    out.push_str(&format!("{kind}.{dir}.{file}\t{rest}\n"));
                }
                _ => out.push_str(&format!("{line}\n")),
            }
        }
        out
    }

    /// The second wave (Issue #1379, tools/plugin-layout/wave2-plugins.mjs): 159 DLLs of the plan, with the first wave's Balance, Filter and Sytrus (lift/plugins). Their file names are
    /// not unique, and their keys are. With SizeOfImage (inputs/plugin-image-sizes.tsv, by SHA-256) they fit the image area: each base is 64 KiB
    /// aligned, after the one before it by that one's size, from Balance's base (0xC1D0000, after oggio's end 0xC1CF000) to below GLOBAL_BASE.
    /// The wave's folders may be in lift/plugins already (Issue #1479): the test runs on a copy that has them, and on lift/plugins as it is.
    #[test]
    fn the_second_wave_is_160_dlls_and_fits_the_image_area() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../.tmp-wave2-plugins");
        let _scratch = Scratch::new(root);
        let tool = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tools/plugin-layout/wave2-plugins.mjs");
        let table = format!("{root}/table");
        assert!(Command::new("node").args([tool, table.as_str()]).status().expect("node").success());
        let wave = read_rows_from(&table).unwrap();
        assert_eq!(wave.len(), 159);
        assert_eq!(wave.iter().filter(|d| d.main).count(), 131);
        assert_eq!(wave.iter().map(|d| d.size).sum::<u64>(), 602_251_576);
        let state = format!("{root}/state");
        copy_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins")), Path::new(&state));
        write_wave_folders(&table, &state);
        assert_eq!(read_rows_from(&state).unwrap().len(), 164, "the copy of lift/plugins (Balance, Filter, Sytrus, Reverb and the Plugin Manager's .exe) and the wave's 159 rows");
        let stale = format!("{root}/stale");
        fs::create_dir_all(format!("{stale}/old")).unwrap();
        fs::write(format!("{stale}/old/plugin.tsv"), format!("key\tguest\trole\tsha256\tsize\thook\targs\tomit\neffects.old.old_x64.dll\tPlugins\\Fruity\\Effects\\Old\\Old_x64.dll\t本体\t{}\t10\tnone\t\t-\n", "a".repeat(64))).unwrap();
        assert!(second_wave_dlls(&stale, &wave).unwrap_err().contains("out of date"), "a row that is neither in the table nor in the first wave's folders must stop the test");
        let sizes_text = include_str!("../../../../inputs/plugin-image-sizes.tsv");
        let image_of = |d: &Dll| -> u64 {
            let line = sizes_text.lines().find(|l| l.split('\t').next() == Some(d.sha256.as_str())).unwrap_or_else(|| panic!("{}: no SizeOfImage row", d.key));
            line.split('\t').nth(2).unwrap().parse().unwrap()
        };
        for plugins in [state.as_str(), concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins")] {
            let dlls = second_wave_dlls(plugins, &wave).unwrap();
            assert_eq!(dlls.len(), 164);
            let sizes: Vec<u64> = dlls.iter().map(image_of).collect();
            let bases = assign_bases(0xC1CF000, &sizes).expect("the images fit below GLOBAL_BASE");
            assert_eq!(bases[0], 0xC1D0000);
            let wave_total: u64 = dlls.iter().filter(|d| d.plugin != "fruity-balance" && d.plugin != "fruity-filter" && !d.is_exe()).map(|d| align_up(image_of(d), ALIGN)).sum();
            assert_eq!(wave_total, 748_158_976 + 524_288, "the wave's 748,158,976 B and Reverb's 516,096 B of SizeOfImage (aligned: 524,288)");
            for pair in bases.windows(2).zip(sizes.iter()) {
                assert!(pair.0[1] >= pair.0[0] + align_up(*pair.1, ALIGN), "the images at {:#x} and {:#x} overlap", pair.0[0], pair.0[1]);
            }
            assert!(bases.iter().all(|b| b % ALIGN == 0));
            assert!(*bases.last().unwrap() + align_up(*sizes.last().unwrap(), ALIGN) <= GLOBAL_BASE);
        }
    }

    /// The DLLs of the second wave, from a plugins folder (lift/plugins, or a copy with the wave's folders) and the table's rows. A DLL is counted
    /// once, by its guest path: the folder's row is the one kept (its key is the realmod's name). A row of the folder that is neither in the
    /// table nor in the first wave's folders means that the table is out of date.
    fn second_wave_dlls(plugins: &str, table: &[Dll]) -> Result<Vec<Dll>, String> {
        let rows = read_rows_from(plugins)?;
        let table_guests: BTreeSet<String> = table.iter().map(|d| d.guest.to_lowercase()).collect();
        if let Some(d) = rows.iter().find(|d| !table_guests.contains(&d.guest.to_lowercase()) && !first_wave_folder(&d.guest)) {
            return Err(format!("{plugins}: {} ({}) is neither in the table nor in the first wave's folders: the table is out of date", d.key, d.guest));
        }
        let folder_guests: BTreeSet<String> = rows.iter().map(|d| d.guest.to_lowercase()).collect();
        let mut dlls = rows;
        dlls.extend(table.iter().filter(|d| !folder_guests.contains(&d.guest.to_lowercase())).cloned());
        dlls.sort_by(|a, b| a.key.cmp(&b.key));
        validate(&dlls)?;
        Ok(dlls)
    }

    /// The first wave's folders (tools/plugin-layout/wave2-plugins.mjs, FIRST_WAVE): their DLLs are in lift/plugins, and not in the table.
    /// Reverb_x64.dll (Issue #1429, a helper of Sytrus in Shared) is in lift/plugins too, and not in the table. The Plugin Manager's .exe (Issue #1499)
    /// is in lift/plugins too, and not in the table.
    fn first_wave_folder(guest: &str) -> bool {
        const FIRST_WAVE: [&str; 6] = [r"Plugins\Fruity\Effects\Fruity Balance", r"Plugins\Fruity\Effects\Fruity Filter", r"Plugins\Fruity\Effects\Fruity Delay 2", r"Plugins\Fruity\Generators\Sytrus", r"Shared\Reverb_x64.dll", r"System\Tools\Plugin Manager\PluginManager.exe"];
        let folder = guest.split('\\').take(4).collect::<Vec<&str>>().join("\\");
        FIRST_WAVE.iter().any(|f| f.eq_ignore_ascii_case(&folder))
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

    const BLUR_SHA: &str = "5840b4347a0fdbc0dfe37d2fc372a2d181ea00b1fed6c9073268b729ca225030";

    fn installer() -> BTreeMap<String, (u64, String)> {
        installer_list(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../inputs/fl-installer-files.tsv")).unwrap()
    }

    fn plugins_dir() -> &'static str {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins")
    }

    /// The Data rows of the installer list (Issue #1478): one Edison file. Its seed row, its lazy file in pack.args, and the checks that refuse a wrong row.
    #[test]
    fn a_data_row_is_a_seed_row_and_a_lazy_file_of_its_host_file() {
        let head = "guest\tinstaller\tsha256\tsize\n";
        let guest = "Plugins\\Fruity\\Effects\\Edison\\Data\\Maps\\Blur - alt.fnv";
        let good = format!("{head}{guest}\t{guest}\t{BLUR_SHA}\t152\n");
        let data = parse_data("edison", &good).unwrap();
        validate_data(&data, &[], &installer()).unwrap();
        assert_eq!(data[0].seed_row(), format!("{GUEST_ROOT}{guest}\t@lazy-file:plugins/Plugins/Fruity/Effects/Edison/Data/Maps/Blur_-_alt.fnv\t{BLUR_SHA}"));
        let pack = pack_args("--args\nx\n", &[], &data).unwrap();
        assert!(pack.ends_with("--lazy-file\nlift/out-app/host/plugins/Plugins/Fruity/Effects/Edison/Data/Maps/Blur_-_alt.fnv=seed/plugins/Plugins/Fruity/Effects/Edison/Data/Maps/Blur_-_alt.fnv\n"));
        let refused = |text: String| validate_data(&parse_data("edison", &text).unwrap(), &[], &installer()).is_err();
        assert!(refused(format!("{head}{guest}\t{guest}\t{}\t152\n", "a".repeat(64))), "a sha256 that the list does not have");
        assert!(refused(format!("{head}{guest}\t{guest}\t{BLUR_SHA}\t153\n")), "a size that the list does not have");
        assert!(refused(format!("{head}{guest}\tPlugins\\Fruity\\Effects\\Edison\\Data\\Maps\\Other.fnv\t{BLUR_SHA}\t152\n")), "an installer row that is not the guest path");
        assert!(refused(format!("{head}Plugins\\Fruity\\Effects\\Edison\\Data\\Scripts\\Examples\\testdll_x64.dll\tPlugins\\Fruity\\Effects\\Edison\\Data\\Scripts\\Examples\\testdll_x64.dll\t{BLUR_SHA}\t152\n")), "a DLL");
        assert!(parse_data("edison", "guest\tinstaller\tsha256\n").is_err(), "a header of 3 columns");
        assert!(parse_data("edison", &format!("{head}{guest}\t{guest}\t{BLUR_SHA}\t0\n")).is_err(), "a size of 0");
        assert!(validate_data(&[data[0].clone(), data[0].clone()], &[], &installer()).is_err(), "a guest path listed twice");
    }

    /// The Sytrus Data (109 files, lift/plugins/sytrus/data.tsv): the rows the shared seed table had (#1414), with the same host files and SHA-256s.
    #[test]
    fn the_sytrus_data_rows_are_the_ones_the_seed_table_had() {
        let dlls = read_plugins_from(plugins_dir()).unwrap();
        let data = read_data_from(plugins_dir()).unwrap();
        validate_data(&data, &dlls, &installer()).unwrap();
        assert_eq!(data.len(), 109);
        assert!(data.iter().all(|d| d.plugin == "sytrus"));
        assert_eq!(data.iter().map(|d| d.size).sum::<u64>(), 1_229_971);
        let default = data.iter().find(|d| d.guest.ends_with("\\Data\\Default.dat")).unwrap();
        assert_eq!(default.seed_row(), "C:\\Program Files\\Image-Line\\FL Studio 21\\Plugins\\Fruity\\Generators\\Sytrus\\Data\\Default.dat\t@lazy-file:plugins/Plugins/Fruity/Generators/Sytrus/Data/Default.dat\tadc5842c0d5d63c9dfa93e9ae3f9fb487f656eaafa3e79202ec61c519372430f");
    }

    /// The second wave's Data (Issue #1478): the files of the 130 folders of tools/plugin-layout/wave2-plugins.mjs (plugin-classes.tsv's folders without a VFX DLL,
    /// and not the first wave's), each one a non-DLL file of the folder's own Data. 551 files, 9,438,441 B (the drafts of the second wave say 551).
    fn wave2_data_tsv() -> String {
        let classes = include_str!("../../../../inputs/plugin-classes.tsv");
        let list = include_str!("../../../../inputs/fl-installer-files.tsv");
        let first_wave = ["Plugins\\Fruity\\Effects\\Fruity Balance", "Plugins\\Fruity\\Effects\\Fruity Filter", "Plugins\\Fruity\\Effects\\Fruity Delay 2", "Plugins\\Fruity\\Generators\\Sytrus"];
        let folder = |path: &str| path.split('\\').take(4).collect::<Vec<&str>>().join("\\");
        let vfx: BTreeSet<String> = classes.lines().skip(1).filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.get(8) == Some(&"yes")).then(|| folder(f[0]))
        }).collect();
        let mut out = String::from("guest\tinstaller\tsha256\tsize\n");
        for line in list.lines().filter(|l| !l.starts_with('#') && !l.starts_with("guest\t")) {
            let f: Vec<&str> = line.split('\t').collect();
            let p: Vec<&str> = f[0].split('\\').collect();
            let own = p.len() >= 6 && p[0] == "Plugins" && p[1] == "Fruity" && p[4] == "Data" && !f[0].to_lowercase().ends_with(".dll");
            if own && !vfx.contains(&folder(f[0])) && !first_wave.contains(&folder(f[0]).as_str()) {
                out.push_str(&format!("{}\t{}\t{}\t{}\n", f[0], f[0], f[2], f[1]));
            }
        }
        out
    }

    fn wave2_dlls(dir: &str) -> Vec<Dll> {
        let tool = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tools/plugin-layout/wave2-plugins.mjs");
        let _ = fs::remove_dir_all(dir);
        assert!(Command::new("node").args([tool, dir]).status().expect("node").success());
        let wave = read_rows_from(dir);
        let _ = fs::remove_dir_all(dir);
        wave.unwrap()
    }

    /// The counts of the second wave with the Data (Issue #1478): the lazy files of the manifest, the lazy rows of the seed table (one stream each) and the seed rows
    /// fit the limits that the sources name (manifest.ts maxItems, fle_vfs.c LAZY_MAX_STREAMS, vfs_lazy.h VL_REG_MAX, vfs_seed.h VS_MAX_ENTRIES).
    #[test]
    fn the_second_wave_with_its_data_fits_the_limits() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../.tmp-wave2-data");
        let _ = fs::remove_dir_all(dir);
        fs::create_dir_all(format!("{dir}/wave2")).unwrap();
        fs::write(format!("{dir}/wave2/data.tsv"), wave2_data_tsv()).unwrap();
        let wave = read_data_from(dir);
        let _ = fs::remove_dir_all(dir);
        let wave = wave.unwrap();
        assert_eq!(wave.len(), 551);
        assert_eq!(wave.iter().map(|d| d.size).sum::<u64>(), 9_438_441);

        let mut dlls = read_plugins_from(plugins_dir()).unwrap();
        dlls.extend(wave2_dlls(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../.tmp-wave2-count")));
        dlls.sort_by(|a, b| a.key.cmp(&b.key));
        validate(&dlls).unwrap();
        let mut data = read_data_from(plugins_dir()).unwrap();
        data.extend(wave);
        data.sort_by(|a, b| a.guest.cmp(&b.guest));
        validate_data(&data, &dlls, &installer()).unwrap();
        assert_eq!(data.len(), 660);

        let fixed_seed = read_text(concat!(env!("CARGO_MANIFEST_DIR"), "/../../harness/seeds/vfs-seed-idle1.tsv")).unwrap();
        let seed = seed_table(&fixed_seed, &dlls, &data);
        let plugins: BTreeSet<&str> = dlls.iter().map(|d| d.plugin.as_str()).collect();
        let seed_rows = seed.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).count() + plugins.len();
        let lazy_rows = seed.lines().filter(|l| l.split('\t').nth(1).is_some_and(|h| h.starts_with(LAZY_FILE))).count();
        let pairs: Vec<(&Dll, u64)> = dlls.iter().map(|d| (d, 0u64)).collect();
        let fixed_pack = read_text(concat!(env!("CARGO_MANIFEST_DIR"), "/../../harness/web/pack.args")).unwrap();
        let pack = pack_args(&fixed_pack, &pairs, &data).unwrap();
        let lazy_items = pack.lines().filter(|l| *l == "--lazy-dll" || *l == "--lazy-file").count() + 1;

        let streams = limit_of(concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtime/fle_vfs.c"), "#define LAZY_MAX_STREAMS ").unwrap();
        let reg = limit_of(concat!(env!("CARGO_MANIFEST_DIR"), "/../../harness/vfs_lazy.h"), "#define VL_REG_MAX ").unwrap();
        let entries = limit_of(concat!(env!("CARGO_MANIFEST_DIR"), "/../../harness/vfs_seed.h"), "#define VS_MAX_ENTRIES ").unwrap();
        let items = limit_of(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../app/src/bundle/manifest.ts"), "lazy: { type: 'array', maxItems: ").unwrap();
        eprintln!("plugins: with the second wave's Data: lazy items {lazy_items} of {items}; seed rows {seed_rows} of {entries}; lazy rows {lazy_rows} of {} streams", streams.min(reg));
        assert!(lazy_rows <= streams.min(reg), "the lazy rows of the seed table are one stream each");
        assert!(seed_rows <= entries, "the seed table's rows");
        assert!(lazy_items <= items, "the lazy files of the manifest");
    }
}
