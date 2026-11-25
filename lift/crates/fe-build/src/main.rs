//! fe-build (decision 0412): the engine steps of deploy.yml as one program. Each process gets the arguments the
//! deploy step gave it, and the working directory is the repository root, so the source paths the compiler sees
//! (and the strings it puts in the wasm) are the same as before. The lists and argument files stay where they are.
//!
//! usage: fe-build engine | extract | external | tables | bundle | upload | endpoint
//! The plans (the stages and steps in run order) are functions that the tests compare with tests/plan/*.txt.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

mod plugins;

const OUT_APP_DIR: &str = "lift/out-app";
const HARNESS_DIR: &str = "lift/harness";
/// Issue #1362: the FL Cloud offline page of the installer (System\Internet, taken with the resources of an Issue that has the `resources` label). A lazy file of the bundle when it is there: the
/// Browser panel's face shows it after the Navigate of the cloud tab (app/src/ui/webview2-surface.ts). Nothing is packed when it is not there.
const OFFLINE_PAGE_SRC: &str = "lift/out-app/host-extra/resources/System/Internet/error-offline.html";
const OFFLINE_PAGE_PATH: &str = "resources/System/Internet/error-offline.html";
const R2_BUCKET: &str = "s3://fl-web-assets/";
const ARTIFACT_BUCKET: &str = "s3://fl-web-artifacts/";
/// The external plugins' wasm (the lazy files of pack.args whose source is here) is built from the installers (external_present).
const PLUGINS_DIR: &str = "lift/out-app/plugins/";
const INSTALLER: &str = "s3://fl-web-artifacts/installers/flstudio_win64_26.1.7.5653.exe";
const BLEND2D_REV: &str = "a7f947677e85441abe1769586d3bb935b4086a07";
/// The SHA-256 of the GitHub archive of BLEND2D_REV. GitHub can regenerate an archive, so a mismatch is a failed attempt
/// and the value is not changed to match (Issue #1374).
const BLEND2D_TARBALL_SHA256: &str = "391a57ad2508ab2291ad04b39b0a82d78473662b6d3029dbaf18de06165ea1f0";
/// The attempts of the archive fetch (the first one included) and the waits after the failed ones (none after the last).
const BLEND2D_ATTEMPTS: usize = 4;
const BLEND2D_WAITS: [Duration; 3] = [Duration::from_secs(10), Duration::from_secs(30), Duration::from_secs(60)];
const BLEND2D_SRC: &str = "lift/out-app/b2d-src/src";
const BLEND2D_LIB: &str = "lift/out-app/b2d/build/libblend2d.a";
/// The flags of the em++ line that the upstream CMake build ran for each source, in its order (the line without
/// the dependency files, the output name and the source).
const BLEND2D_CXX_FLAGS: [&str; 24] = [
    "-O2", "-fno-exceptions", "-fno-rtti", "-matomics", "-mbulk-memory", "-O3", "-DNDEBUG", "-std=gnu++11", "-fvisibility=hidden", "-Wall",
    "-Wextra", "-fno-exceptions", "-fno-rtti", "-fno-math-errno", "-fno-trapping-math", "-fno-finite-math-only", "-mllvm",
    "--disable-loop-idiom-all", "-fno-threadsafe-statics", "-DBL_STATIC", "-DBL_BUILD_NO_JIT", "-O2", "-fmerge-all-constants",
    "-ftree-vectorize",
];
const ENGINE_DIRS: [&str; 10] = ["obj/rt", "obj/h", "obj/fle", "obj/mod", "obj/b2d", "web", "hyb", "seed", "dll", "bundle"];

enum Step {
    Mkdir(Vec<String>),
    Run(Vec<String>),
    /// the arguments are the command's, then one argument per line of the file (read when the step runs: the file is generated)
    RunLines(Vec<String>, String),
    /// the arguments are the command's, then the lines of pack.args (read when the step runs; without the external plugins, their lazy files are left out)
    RunPack(Vec<String>, bool),
    /// the arguments are the command's, then the words of the file (read when the step runs: the file is generated)
    RunWords(Vec<String>, String),
    RunStdin(Vec<String>, String),
    RemoveFile(String),
    RemoveDir(String),
    Fetch(String, String, String),
    CheckSources(String),
    CopyDlls,
    Copy(String, String),
    Glue,
    /// the plugin tables under lift/out-app/plugins (with the bases when true)
    Generate(bool),
    /// the direct list of one plugin DLL (key, the engine's bin directory)
    PluginList(String, String),
    MergeLists,
    CheckUnits(Vec<String>),
}

struct Stage {
    name: String,
    steps: Vec<Step>,
    parallel: bool,
}

impl Step {
    #[cfg(test)]
    fn show(&self) -> String {
        match self {
            Step::Mkdir(dirs) => format!("mkdir -p {}", dirs.join(" ")),
            Step::Run(argv) => argv.join(" "),
            Step::RunLines(argv, file) => format!("{} @lines {file}", argv.join(" ")),
            Step::RunPack(argv, external) => format!("{} @lines {}/pack.args{}", argv.join(" "), plugins::OUT_DIR, if *external { "" } else { ", without the external plugins' lazy files" }),
            Step::RunWords(argv, file) => format!("{} @words {file}", argv.join(" ")),
            Step::RunStdin(argv, stdin_file) => format!("{} < {stdin_file}", argv.join(" ")),
            Step::RemoveFile(path) => format!("rm -f {path}"),
            Step::RemoveDir(dir) => format!("rm -rf {dir}"),
            Step::Fetch(url, tarball, sha) => format!("fetch {url} to {tarball} (sha256 {sha})"),
            Step::CheckSources(list_file) => format!("check {list_file} against the .cpp files under {BLEND2D_SRC}"),
            Step::CopyDlls => format!("copy {OUT_APP_DIR}/gen/dll/*.dll and {OUT_APP_DIR}/host/binaries/win64/FLEngine_x64.dll to {OUT_APP_DIR}/dll/"),
            Step::Copy(from, to) => format!("copy {from} to {to}"),
            Step::Glue => format!("glue {OUT_APP_DIR}/web/flengine6.js -> {OUT_APP_DIR}/web/engine-glue.js (the two rules of {HARNESS_DIR}/web/glue.sed)"),
            Step::Generate(with_bases) => format!("plugins: tables under {} from lift/plugins/*/plugin.tsv (bases: {with_bases})", plugins::OUT_DIR),
            Step::PluginList(key, _) => format!("list {key}: fe-pe rebase, fe-wasmgen report, hook points and omit VAs left out"),
            Step::MergeLists => format!("merge {}/lists into {}/lowered-all.txt", plugins::OUT_DIR, plugins::OUT_DIR),
            Step::CheckUnits(listed) => format!("check the {} units of {OUT_APP_DIR}/gen/src against the compile lists", listed.len()),
        }
    }
}

fn words(path: &str) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(text.split_whitespace().map(String::from).collect())
}

fn lines(path: &str) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(text.lines().map(String::from).collect())
}

fn sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items
}

fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(s, _)| s)
}

fn rt_sources() -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for entry in fs::read_dir("lift/runtime").map_err(|e| format!("lift/runtime: {e}"))? {
        let name = entry.map_err(|e| e.to_string())?.file_name().to_string_lossy().into_owned();
        if name.starts_with("fle_") && name.ends_with(".c") {
            names.push(format!("lift/runtime/{name}"));
        }
    }
    Ok(sorted(names))
}

fn compile_unit(flags: &str, extra: &[&str], src: &str, obj: &str) -> Step {
    let mut argv: Vec<String> = vec!["emcc".into(), "-c".into(), format!("@{flags}")];
    argv.extend(extra.iter().map(|s| s.to_string()));
    argv.push(src.into());
    argv.push("-o".into());
    argv.push(obj.into());
    Step::Run(argv)
}

fn hex_len(text: &str) -> usize {
    text.bytes().take_while(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)).count()
}

/// `^ *function _f_[0-9a-f]+\(\.\.\.args` (the start of a range of glue.sed).
fn is_fn_start(line: &str) -> bool {
    let Some(after_prefix) = line.trim_start_matches(' ').strip_prefix("function _f_") else { return false };
    let digits = hex_len(after_prefix);
    digits > 0 && after_prefix[digits..].starts_with("(...args")
}

/// `^ *(\/\*\* @export \*\/ )?f_[0-9a-f]+: _f_[0-9a-f]+,$` (a line glue.sed deletes).
fn is_export_line(line: &str) -> bool {
    let body = line.trim_start_matches(' ');
    let body = body.strip_prefix("/** @export */ ").unwrap_or(body);
    let Some(after_f) = body.strip_prefix("f_") else { return false };
    let id_digits = hex_len(after_f);
    if id_digits == 0 {
        return false;
    }
    let Some(after_colon) = after_f[id_digits..].strip_prefix(": _f_") else { return false };
    let value_digits = hex_len(after_colon);
    value_digits > 0 && &after_colon[value_digits..] == ","
}

/// `sed -E -f lift/harness/web/glue.sed`: the range from a function start to the line with `stub = true;` (both
/// lines are deleted), then the export lines. The start line itself is not tested against the end.
fn glue(text: &str) -> String {
    let mut out = String::new();
    let mut in_range = false;
    for line in text.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);
        if in_range {
            if body.contains("stub = true;") {
                in_range = false;
            }
            continue;
        }
        if is_fn_start(body) {
            in_range = true;
            continue;
        }
        if is_export_line(body) {
            continue;
        }
        out.push_str(line);
    }
    out
}

/// The sources of Blend2D in the order of the upstream build (the list is data; CheckSources compares it with the tree).
fn blend2d_list_file() -> String {
    format!("{HARNESS_DIR}/b2d1/blend2d-sources.txt")
}

/// (source, object) for each line of the list. The object keeps the source's relative path and name (`x.cpp.o`).
fn blend2d_units() -> Result<Vec<(String, String)>, String> {
    Ok(lines(&blend2d_list_file())?.into_iter().map(|rel| (format!("{BLEND2D_SRC}/{rel}"), format!("{OUT_APP_DIR}/b2d/obj/{rel}.o"))).collect())
}

/// The GitHub archive (a tarball of the commit's tree, with one top-level directory that `tar` strips).
fn blend2d_archive_url() -> String {
    format!("https://github.com/blend2d/blend2d/archive/{BLEND2D_REV}.tar.gz")
}

/// Kept under OUT_APP_DIR, which is git-ignored (`lift/out*/`).
fn blend2d_tarball() -> String {
    format!("{OUT_APP_DIR}/blend2d-{BLEND2D_REV}.tar.gz")
}

/// One download with curl. `-f` makes an HTTP error (403, 404) a failure, and `--max-time` makes a stalled transfer one too
/// (so that it is retried, not left to the job's time limit). The message is curl's last line on stderr.
fn curl_fetch(url: &str, path: &str) -> Result<(), String> {
    let out = Command::new("curl").args(["-sSLf", "--connect-timeout", "30", "--max-time", "600", "-o", path, url]).output().map_err(|e| format!("curl: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let line = stderr.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or_default().trim();
    Err(format!("curl {}: {line}", out.status))
}

/// Puts the archive at `tarball`, unless a tarball with the SHA-256 `sha` is there already. An attempt is `fetch(url, part)`
/// and then the SHA-256 check of `part`, where `part` is `<tarball>.part` until the check passes. A failed attempt is
/// retried after its wait (BLEND2D_WAITS), up to BLEND2D_ATTEMPTS.
fn blend2d_fetch_with(
    url: &str,
    tarball: &str,
    sha: &str,
    fetch: &mut dyn FnMut(&str, &str) -> Result<(), String>,
    wait: &mut dyn FnMut(Duration),
) -> Result<(), String> {
    if Path::new(tarball).is_file() {
        match sha256_hex(tarball) {
            Ok(got) if got == sha => {
                println!("blend2d: {tarball} has the SHA-256 of {BLEND2D_REV}: not fetched");
                return Ok(());
            }
            Ok(got) => eprintln!("fe-build: {tarball} has SHA-256 {got}, expected {sha}: discarded"),
            Err(message) => eprintln!("fe-build: {tarball}: {message}: discarded"),
        }
        fs::remove_file(tarball).map_err(|e| format!("{tarball}: {e}"))?;
    }
    let part = format!("{tarball}.part");
    let mut attempt = 1;
    loop {
        let Err(message) = fetch(url, &part).and_then(|()| {
            let got = sha256_hex(&part)?;
            if got == sha {
                Ok(())
            } else {
                Err(format!("SHA-256 {got}, expected {sha}"))
            }
        }) else {
            break;
        };
        let _ = fs::remove_file(&part);
        if attempt == BLEND2D_ATTEMPTS {
            eprintln!("fe-build: blend2d attempt {attempt}/{BLEND2D_ATTEMPTS} failed ({url}): {message}; giving up");
            return Err(format!("blend2d: no archive after {BLEND2D_ATTEMPTS} attempts"));
        }
        let pause = BLEND2D_WAITS[attempt - 1];
        eprintln!("fe-build: blend2d attempt {attempt}/{BLEND2D_ATTEMPTS} failed ({url}): {message}; waiting {} s", pause.as_secs());
        wait(pause);
        attempt += 1;
    }
    fs::rename(&part, tarball).map_err(|e| format!("{part} to {tarball}: {e}"))
}

/// The archive of the fixed commit, unpacked fresh each time (the old tree is removed), then the patch and the check.
fn blend2d_source_steps() -> Vec<Step> {
    let src = format!("{OUT_APP_DIR}/b2d-src");
    vec![
        Step::Mkdir(vec![OUT_APP_DIR.into()]),
        Step::Fetch(blend2d_archive_url(), blend2d_tarball(), BLEND2D_TARBALL_SHA256.into()),
        Step::RemoveDir(src.clone()),
        Step::Mkdir(vec![src.clone()]),
        Step::Run(vec!["tar".into(), "-xzf".into(), blend2d_tarball(), "-C".into(), src.clone(), "--strip-components=1".into()]),
        Step::RunStdin(
            vec!["patch".into(), "-p1".into(), "-d".into(), src.clone()],
            format!("{HARNESS_DIR}/b2d1/b2d3-compop.patch"),
        ),
        Step::RunStdin(
            vec!["patch".into(), "-p1".into(), "-d".into(), src.clone()],
            format!("{HARNESS_DIR}/b2d1/b2d4-tick.patch"),
        ),
        Step::CheckSources(blend2d_list_file()),
    ]
}

fn blend2d_dirs(units: &[(String, String)]) -> Vec<String> {
    let mut dirs: Vec<String> = units.iter().filter_map(|(_, obj)| Path::new(obj).parent().map(|dir| dir.display().to_string())).collect();
    dirs.push(Path::new(BLEND2D_LIB).parent().map(|dir| dir.display().to_string()).unwrap_or_default());
    dirs.sort();
    dirs.dedup();
    dirs
}

fn blend2d_compile_steps(units: &[(String, String)]) -> Vec<Step> {
    units
        .iter()
        .map(|(src, obj)| {
            let mut argv: Vec<String> = vec!["em++".into()];
            argv.extend(BLEND2D_CXX_FLAGS.iter().map(|flag| flag.to_string()));
            argv.extend(["-c".to_string(), src.clone(), "-o".to_string(), obj.clone()]);
            Step::Run(argv)
        })
        .collect()
}

fn blend2d_archive_steps(units: &[(String, String)]) -> Vec<Step> {
    let mut emar_args: Vec<String> = vec!["emar".into(), "qc".into(), BLEND2D_LIB.into()];
    emar_args.extend(units.iter().map(|(_, obj)| obj.clone()));
    vec![Step::RemoveFile(BLEND2D_LIB.into()), Step::Run(emar_args), Step::Run(vec!["emranlib".into(), BLEND2D_LIB.into()])]
}

fn check_blend2d_sources(list_file: &str) -> Result<(), String> {
    fn collect(dir: &str, rel: &str, found: &mut Vec<String>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| format!("{dir}: {e}"))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel_path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let path = entry.path();
            if path.is_dir() {
                collect(&path.display().to_string(), &rel_path, found)?;
            } else if name.ends_with(".cpp") {
                found.push(rel_path);
            }
        }
        Ok(())
    }
    let listed = sorted(lines(list_file)?);
    let mut found = Vec::new();
    collect(BLEND2D_SRC, "", &mut found)?;
    let found = sorted(found);
    if listed == found {
        Ok(())
    } else {
        Err(format!("{list_file} lists {} sources, the .cpp files under {BLEND2D_SRC} are {}", listed.len(), found.len()))
    }
}

fn plan(bin: &str) -> Result<Vec<Stage>, String> {
    plan_with(bin, &plugins::keys()?)
}

/// The stages of the engine build for the plugins with these keys (the lists stage has one step for each key).
fn plan_with(bin: &str, keys: &[String]) -> Result<Vec<Stage>, String> {
    let fe_lift = format!("{bin}/fe-lift");
    let fe_wasmgen = format!("{bin}/fe-wasmgen");
    let mut stages = vec![Stage { name: "plugins".into(), steps: vec![Step::Generate(true)], parallel: false }];
    let plugin_steps = keys.iter().map(|key| Step::PluginList(key.clone(), bin.to_string())).collect();
    stages.push(Stage { name: "lists".into(), steps: plugin_steps, parallel: true });
    stages.push(Stage { name: "lowered-all".into(), steps: vec![Step::MergeLists], parallel: false });
    let units = blend2d_units()?;
    let mut blend2d_steps = blend2d_source_steps();
    blend2d_steps.push(Step::Mkdir(blend2d_dirs(&units)));
    stages.push(Stage { name: "blend2d".into(), steps: blend2d_steps, parallel: false });
    stages.push(Stage { name: "blend2d-compile".into(), steps: blend2d_compile_steps(&units), parallel: true });
    stages.push(Stage { name: "blend2d-archive".into(), steps: blend2d_archive_steps(&units), parallel: false });

    let mut gen_steps = vec![Step::Mkdir(ENGINE_DIRS.iter().map(|dir| format!("{OUT_APP_DIR}/{dir}")).collect())];
    let mut gen_args = vec![fe_lift, "gen-all".into()];
    gen_args.extend(lines(&format!("{HARNESS_DIR}/b2d2/gen.args"))?);
    gen_steps.push(Step::Run(gen_args));
    gen_steps.push(Step::CopyDlls);
    stages.push(Stage { name: "gen".into(), steps: gen_steps, parallel: false });
    let mut listed_units = words(&format!("{HARNESS_DIR}/two/plain-units.txt"))?;
    listed_units.extend(words(&format!("{HARNESS_DIR}/two/ai-units.txt"))?);
    stages.push(Stage { name: "units".into(), steps: vec![Step::CheckUnits(listed_units)], parallel: false });

    let (rt_flags, h_flags) = (format!("{HARNESS_DIR}/web/cflags-rt.args"), format!("{HARNESS_DIR}/web/cflags-h.args"));
    let (unit_flags, ai_flags) = (format!("{HARNESS_DIR}/web/cflags-u.args"), format!("{HARNESS_DIR}/web/cflags-ai.args"));
    let mut compile = Vec::new();
    let mut rt_objs = Vec::new();
    for src in rt_sources()? {
        let obj_path = format!("{OUT_APP_DIR}/obj/rt/{}.o", stem(&src));
        compile.push(compile_unit(&rt_flags, &[], &src, &obj_path));
        rt_objs.push(obj_path);
    }
    let mut h_objs = Vec::new();
    let main_two = format!("{HARNESS_DIR}/two/main_two.c");
    let obj_path = format!("{OUT_APP_DIR}/obj/h/main_two.o");
    compile.push(compile_unit(&h_flags, &["-DFLE_FRUITY=1"], &main_two, &obj_path));
    h_objs.push(obj_path);
    for src in words(&format!("{HARNESS_DIR}/two/harness-inputs.txt"))? {
        let obj_path = format!("{OUT_APP_DIR}/obj/h/{}.o", stem(&src));
        compile.push(compile_unit(&h_flags, &[], &src, &obj_path));
        h_objs.push(obj_path);
    }
    let mut gen_objs = Vec::new();
    for (list_file, flags) in [("plain-units.txt", &unit_flags), ("ai-units.txt", &ai_flags)] {
        for src in words(&format!("{HARNESS_DIR}/two/{list_file}"))? {
            let dir = if stem(&src).starts_with("fle-") { "fle" } else { "mod" };
            let obj_path = format!("{OUT_APP_DIR}/obj/{dir}/{}.o", stem(&src));
            compile.push(compile_unit(flags, &[], &src, &obj_path));
            gen_objs.push(obj_path);
        }
    }
    let common = format!("{OUT_APP_DIR}/gen/common.c");
    let obj_path = format!("{OUT_APP_DIR}/obj/mod/common.o");
    compile.push(compile_unit(&unit_flags, &[], &common, &obj_path));
    gen_objs.push(obj_path);
    compile.push(Step::Run(vec!["emcc".into(), format!("@{HARNESS_DIR}/web/cflags-b2d.args"), format!("{HARNESS_DIR}/b2d1/b2d_swap.cpp")]));
    let fle_count = gen_objs.iter().filter(|o| o.contains("/obj/fle/")).count();
    let mod_count = gen_objs.len() - fle_count;
    let name = format!(
        "compile (rt {}, h {}, gen {} = fle {fle_count} + mod {mod_count} incl. common.c, b2d 1; total {})",
        rt_objs.len(),
        h_objs.len(),
        gen_objs.len(),
        compile.len()
    );
    stages.push(Stage { name, steps: compile, parallel: true });

    let hybrid_n0_file = format!("{HARNESS_DIR}/hyb/n0.cmd");
    let mut hybrid_n0_argv = vec![fe_wasmgen.clone(), "hybrid-n0".into()];
    hybrid_n0_argv.extend(lines(&hybrid_n0_file)?);
    stages.push(Stage { name: "hybrid-n0".into(), steps: vec![Step::Run(hybrid_n0_argv)], parallel: false });

    let mut link_argv = vec!["emcc".into(), format!("@{HARNESS_DIR}/web/link.rsp"), format!("@{OUT_APP_DIR}/exports.rsp")];
    for group in [
        &h_objs,
        &gen_objs.iter().filter(|o| o.contains("/obj/mod/")).cloned().collect(),
        &gen_objs.iter().filter(|o| o.contains("/obj/fle/")).cloned().collect(),
        &rt_objs,
    ] {
        link_argv.extend(sorted(group.clone()));
    }
    stages.push(Stage { name: "link".into(), steps: vec![Step::Run(link_argv)], parallel: false });

    let mut wasm_opt_argv = vec!["wasm-opt".to_string()];
    wasm_opt_argv.extend(lines(&format!("{HARNESS_DIR}/web/wasm-opt-c.cmd"))?);
    stages.push(Stage { name: "wasm-opt".into(), steps: vec![Step::Run(wasm_opt_argv)], parallel: false });

    let hybrid_cmd = format!("{}/hybrid.cmd", plugins::OUT_DIR);
    let func_names = format!("{OUT_APP_DIR}/hyb/wasm-func-names.tsv");
    stages.push(Stage { name: "hybrid".into(), steps: vec![Step::RunLines(vec![fe_wasmgen], hybrid_cmd), Step::Copy(func_names, format!("{OUT_APP_DIR}/gen/wasm-func-names.tsv")), Step::Glue], parallel: false });
    Ok(stages)
}

fn exec(cmd: &mut Command, label: &str) -> Result<(), String> {
    let status = cmd.status().map_err(|e| format!("{label}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label}: {status}"))
    }
}

fn copy_dlls() -> Result<(), String> {
    let mut srcs = Vec::new();
    for entry in fs::read_dir(format!("{OUT_APP_DIR}/gen/dll")).map_err(|e| format!("{OUT_APP_DIR}/gen/dll: {e}"))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "dll") {
            srcs.push(path);
        }
    }
    srcs.sort();
    srcs.push(PathBuf::from(format!("{OUT_APP_DIR}/host/binaries/win64/FLEngine_x64.dll")));
    for src in srcs {
        let name = src.file_name().ok_or("no file name")?;
        fs::copy(&src, Path::new(&format!("{OUT_APP_DIR}/dll")).join(name)).map_err(|e| format!("{}: {e}", src.display()))?;
    }
    Ok(())
}

fn run(step: &Step) -> Result<(), String> {
    match step {
        Step::Mkdir(dirs) => {
            for dir in dirs {
                fs::create_dir_all(dir).map_err(|e| format!("mkdir {dir}: {e}"))?;
            }
            Ok(())
        }
        Step::Run(argv) => exec(Command::new(&argv[0]).args(&argv[1..]), &argv[0]),
        Step::RunLines(argv, file) => {
            let mut all = argv.clone();
            all.extend(lines(file)?);
            exec(Command::new(&all[0]).args(&all[1..]), &all[0])
        }
        Step::RunWords(argv, file) => {
            let mut all = argv.clone();
            all.extend(words(file)?);
            exec(Command::new(&all[0]).args(&all[1..]), &all[0])
        }
        Step::RunPack(argv, external) => {
            let mut all = argv.clone();
            all.extend(pack_args(*external)?);
            exec(Command::new(&all[0]).args(&all[1..]), &all[0])
        }
        Step::Generate(with_bases) => plugins::generate(*with_bases),
        Step::PluginList(key, bin) => plugins::run_list(key, bin),
        Step::MergeLists => plugins::merge_lists(),
        Step::CheckUnits(listed) => plugins::check_units(listed),
        Step::RunStdin(argv, stdin_file) => {
            let file = fs::File::open(stdin_file).map_err(|e| format!("{stdin_file}: {e}"))?;
            exec(Command::new(&argv[0]).args(&argv[1..]).stdin(Stdio::from(file)), &argv[0])
        }
        Step::RemoveFile(path) => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("rm {path}: {error}")),
        },
        Step::RemoveDir(dir) => match fs::remove_dir_all(dir) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("rm -rf {dir}: {error}")),
        },
        Step::Fetch(url, tarball, sha) => blend2d_fetch_with(url, tarball, sha, &mut curl_fetch, &mut std::thread::sleep),
        Step::CheckSources(list_file) => check_blend2d_sources(list_file),
        Step::CopyDlls => copy_dlls(),
        Step::Copy(from, to) => {
            if let Some(dir) = Path::new(to).parent() {
                fs::create_dir_all(dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
            }
            fs::copy(from, to).map(|_| ()).map_err(|e| format!("copy {from} to {to}: {e}"))
        }
        Step::Glue => {
            let from = format!("{OUT_APP_DIR}/web/flengine6.js");
            let text = fs::read_to_string(&from).map_err(|e| format!("{from}: {e}"))?;
            fs::write(format!("{OUT_APP_DIR}/web/engine-glue.js"), glue(&text)).map_err(|e| format!("engine-glue.js: {e}"))
        }
    }
}

fn run_parallel(steps: &[Step]) -> Result<(), String> {
    let next = AtomicUsize::new(0);
    let failed: Mutex<Option<String>> = Mutex::new(None);
    let workers = std::thread::available_parallelism().map_or(2, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some(step) = steps.get(index) else { break };
                if let Err(message) = run(step) {
                    *failed.lock().unwrap_or_else(|p| p.into_inner()) = Some(message);
                    next.store(steps.len(), Ordering::SeqCst);
                    break;
                }
            });
        }
    });
    match failed.into_inner().unwrap_or_else(|p| p.into_inner()) {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

/// The emsdk's own tools (wasm-opt, ...) sit next to emcc's directory: `<emcc dir>/../bin`. They go first on PATH, so
/// that wasm-opt is the emsdk's version and not the system's one.
fn toolchain_bin() -> Result<String, String> {
    let path = env::var("PATH").unwrap_or_default();
    let emcc_dir = env::split_paths(&path).find(|dir| dir.join("emcc").is_file()).ok_or("emcc is not on PATH")?;
    Ok(emcc_dir.join("../bin").display().to_string())
}

#[cfg(test)]
/// The stages and steps of the engine build in run order, as text (one stage line, then its steps), for the plugins with these keys.
fn engine_plan_lines(bin: &str, keys: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for stage in plan_with(bin, keys)? {
        let par = if stage.parallel { ", parallel" } else { "" };
        out.push(format!("stage {}: {} step(s){par}", stage.name, stage.steps.len()));
        out.extend(stage.steps.iter().map(|step| format!("  {}", step.show())));
    }
    Ok(out)
}

fn engine() -> Result<(), String> {
    let bin = env::current_exe().map_err(|e| e.to_string())?;
    let bin = bin.parent().ok_or("no directory for the binary")?.display().to_string();
    let toolchain = toolchain_bin()?;
    let path = env::var("PATH").unwrap_or_default();
    env::set_var("PATH", format!("{toolchain}:{path}"));
    for stage in plan(&bin)? {
        let par = if stage.parallel { ", parallel" } else { "" };
        println!("stage {}: {} step(s){par}", stage.name, stage.steps.len());
        let started = std::time::Instant::now();
        if stage.parallel {
            run_parallel(&stage.steps)?;
        } else {
            for step in &stage.steps {
                run(step)?;
            }
        }
        println!("stage {} took {:.1} s", stage.name, started.elapsed().as_secs_f64());
    }
    Ok(())
}

fn sha256_hex(path: &str) -> Result<String, String> {
    let out = Command::new("sha256sum").arg(path).output().map_err(|e| format!("sha256sum: {e}"))?;
    if !out.status.success() {
        return Err(format!("sha256sum {path}: {}", out.status));
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.get(..64).map(String::from).ok_or_else(|| format!("sha256sum {path}: no digest"))
}

#[cfg(test)]
/// One stage line, then the steps of the stage (the text that `plan` tests compare).
fn stage_lines(name: &str, steps: &[Step]) -> Vec<String> {
    let mut out = vec![format!("stage {name}: {} step(s)", steps.len())];
    out.extend(steps.iter().map(|step| format!("  {}", step.show())));
    out
}

fn seed_step() -> Step {
    Step::RunWords(vec!["node".into(), "app/scripts/pack-seed.mjs".into()], format!("{}/seed.args", plugins::OUT_DIR))
}

/// The arguments of pack.args (one per line; plugins::generate writes it). Without the external plugins' installers (`external` false),
/// the --lazy-file of a plugin's wasm (its source under lift/out-app/plugins/) is left out, and each one is said on stderr.
fn pack_args(external: bool) -> Result<Vec<String>, String> {
    Ok(keep_lazies(&lines(&format!("{}/pack.args", plugins::OUT_DIR))?, external))
}

fn keep_lazies(args: &[String], external: bool) -> Vec<String> {
    let mut kept = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let plugin = args[i] == "--lazy-file" && args.get(i + 1).is_some_and(|v| v.starts_with(PLUGINS_DIR));
        if plugin && !external {
            eprintln!("fe-build: 外部プラグインのインストーラが無いので、省く: {}", args[i + 1]);
            i += 2;
        } else {
            kept.push(args[i].clone());
            i += 1;
        }
    }
    kept
}

/// The pack step for the engine with the sha256 `sha` (its first 12 digits name the version). Its lazy files are the lines of pack.args,
/// read when the step runs (the offline page is one of them when it is there).
fn pack_step_for(sha: &str, external: bool) -> Step {
    Step::RunPack(vec!["node".into(), "app/scripts/pack-assets.mjs".into(), "--version".into(), sha[..12].into()], external)
}

#[cfg(test)]
fn bundle_plan_steps(sha: &str, external: bool) -> Vec<Step> {
    let mut steps = if external { plugin_steps() } else { Vec::new() };
    steps.extend([seed_step(), pack_step_for(sha, external)]);
    steps
}

/// The bundle. The external plugins' wasm is in it only when their installers are there (external_present).
fn bundle() -> Result<(), String> {
    let external = external_present()?;
    if external {
        for step in &plugin_steps() {
            run(step)?;
        }
    }
    run(&seed_step())?;
    let sha = sha256_hex(&format!("{OUT_APP_DIR}/web/engine.wasm"))?;
    run(&pack_step_for(&sha, external))
}

/// The external plugins' installers (ci/artifacts.lock: the installers/ rows that are not FL Studio's), the input of build.sh of the OTT wasm.
fn external_keys() -> Result<Vec<String>, String> {
    let text = fs::read_to_string("ci/artifacts.lock").map_err(|e| format!("ci/artifacts.lock: {e}"))?;
    let rows = text.lines().filter(|l| !l.starts_with('#') && !l.is_empty());
    Ok(rows.filter_map(|l| l.split('\t').next()).filter(|k| k.starts_with("installers/") && !k.starts_with("installers/flstudio_")).map(String::from).collect())
}

/// The installers into lift/out-app/external, then the check of them against ci/artifacts.lock (the check of build.sh reads all of them).
fn external_steps() -> Result<Vec<Step>, String> {
    let dir = format!("{OUT_APP_DIR}/external");
    let mut steps = vec![Step::Mkdir(vec![dir.clone()])];
    for key in external_keys()? {
        let name = key.rsplit('/').next().unwrap_or(&key).to_string();
        steps.push(aws_step(&["s3", "cp", &format!("{ARTIFACT_BUCKET}{key}"), &format!("{dir}/{name}")]));
    }
    steps.push(Step::Run(vec!["node".into(), "tools/external-inputs/check.mjs".into(), dir]));
    Ok(steps)
}

fn external() -> Result<(), String> {
    for step in &external_steps()? {
        run(step)?;
    }
    Ok(())
}

/// The file names of the external plugins' installers (the names that `external` gives them in lift/out-app/external).
fn external_names() -> Result<Vec<String>, String> {
    Ok(external_keys()?.iter().map(|key| key.rsplit('/').next().unwrap_or(key).to_string()).collect())
}

/// Whether the external plugins' installers are in lift/out-app/external (`external` puts them there).
fn external_present() -> Result<bool, String> {
    external_in(&format!("{OUT_APP_DIR}/external"), &external_names()?)
}

/// Whether the installers `names` are in `dir`. None of them: false, and the bundle leaves the plugins out. Some of them, or a
/// file that does not match ci/artifacts.lock: an error (the same check as `external`'s).
fn external_in(dir: &str, names: &[String]) -> Result<bool, String> {
    if !names.iter().any(|name| Path::new(dir).join(name).is_file()) {
        return Ok(false);
    }
    run(&Step::Run(vec!["node".into(), "tools/external-inputs/check.mjs".into(), dir.into()]))?;
    Ok(true)
}

/// The external plugins' wasms (#1337 OTT, #1336 Serum 2, #1338 Vital): build.sh reads the installers that `external` put in lift/out-app/external and writes
/// lift/out-app/plugins/<name>/vst-ext.wasm, the lazy files of the bundle (pack.args). The same builds as the work chain's.
/// The bundle runs them only when the installers are there (external_present).
fn plugin_steps() -> Vec<Step> {
    ["ott", "serum2", "vital"]
        .iter()
        .map(|name| {
            Step::Run(vec![
                "bash".into(),
                format!("{HARNESS_DIR}/web/mt1/plugin-worker/build.sh"),
                name.to_string(),
                format!("{OUT_APP_DIR}/external"),
                format!("{OUT_APP_DIR}/plugins/{name}"),
            ])
        })
        .collect()
}

fn aws_step(args: &[&str]) -> Step {
    let mut argv = vec!["aws".to_string()];
    argv.extend(args.iter().map(|arg| arg.to_string()));
    Step::Run(argv)
}

/// The `aws s3 sync` calls that upload the bundle: the Brotli files, then the raw files.
fn upload_sync_steps() -> Vec<Step> {
    let br_dir = format!("{OUT_APP_DIR}/bundle/br");
    let raw_dir = format!("{OUT_APP_DIR}/bundle/raw");
    vec![
        aws_step(&["s3", "sync", &br_dir, R2_BUCKET, "--content-encoding", "br", "--cache-control", "max-age=31536000, immutable"]),
        aws_step(&["s3", "sync", &raw_dir, R2_BUCKET, "--cache-control", "no-cache"]),
    ]
}

/// The copy that makes the new bundle live. It is always the last step of upload(), after the bundle's files are up,
/// so the manifest never names a file that is not there yet.
fn upload_manifest_step() -> Step {
    let manifest_file = format!("{OUT_APP_DIR}/bundle/raw/fl-Oz/manifest.json");
    let manifest_key = format!("{R2_BUCKET}manifest.json");
    aws_step(&["s3", "cp", &manifest_file, &manifest_key, "--cache-control", "no-cache", "--content-type", "application/json"])
}

fn upload() -> Result<(), String> {
    for step in &upload_sync_steps() {
        run(step)?;
    }
    run(&upload_manifest_step())
}

fn endpoint() -> Result<(), String> {
    let text = fs::read_to_string("app/wrangler.jsonc").map_err(|e| format!("app/wrangler.jsonc: {e}"))?;
    let line = text.lines().find(|l| l.contains("\"account_id\"")).ok_or("app/wrangler.jsonc has no account_id")?;
    let account = line.split('"').nth(3).ok_or("account_id has no value")?;
    let url = format!("https://{account}.r2.cloudflarestorage.com");
    let path = env::var("GITHUB_ENV").map_err(|_| "GITHUB_ENV is not set (this runs in GitHub Actions)".to_string())?;
    let mut env_file = fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("{path}: {e}"))?;
    writeln!(env_file, "AWS_ENDPOINT_URL={url}").and_then(|_| writeln!(env_file, "RUNS_ON_S3_BUCKET_ENDPOINT={url}")).map_err(|e| format!("{path}: {e}"))
}

fn extract_steps() -> Vec<Step> {
    let host = format!("{OUT_APP_DIR}/host");
    let installer = format!("{OUT_APP_DIR}/in.exe");
    vec![
        Step::Mkdir(vec![host.clone()]),
        aws_step(&["s3", "cp", INSTALLER, &installer]),
        Step::Run(vec![
            "node".into(),
            "app/scripts/extract-fl.mjs".into(),
            installer,
            host,
            "--dlls".into(),
            format!("{}/extract-dlls.tsv", plugins::OUT_DIR),
            "--seed".into(),
            format!("{}/vfs-seed-idle1.tsv", plugins::OUT_DIR),
        ]),
    ]
}

fn extract() -> Result<(), String> {
    plugins::generate(false)?;
    for step in &extract_steps() {
        run(step)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("engine") => {
            if !Path::new("lift/Cargo.toml").is_file() {
                Err("run from the repository root (lift/Cargo.toml is not here)".into())
            } else {
                engine()
            }
        }
        Some("extract") => extract(),
        Some("external") => external(),
        Some("tables") => plugins::generate(false),
        Some("bundle") => bundle(),
        Some("upload") => upload(),
        Some("endpoint") => endpoint(),
        _ => {
            eprintln!("usage: fe-build engine | extract | external | tables | bundle | upload | endpoint");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fe-build: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{blend2d_fetch_with, bundle_plan_steps, engine_plan_lines, external_in, external_names, external_steps, extract_steps, glue, keep_lazies, plan, plugins, stage_lines, upload_manifest_step, upload_sync_steps};
    use std::cell::Cell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    const SHA: &str = "b440e13a623aa949ccce21db53566658425743bb2b21263bcf773d278396e043";
    /// The SHA-256 of an empty file. The fetch tests use an empty file (matches) or the bytes BAD (does not match).
    const EMPTY_SHA: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const EMPTY: &[u8] = b"";
    const BAD: &[u8] = b"stale";
    const URL: &str = "https://example.invalid/blend2d.tar.gz";

    /// A directory `.tmp-fe-build-<name>` in the repository root, removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").join(format!(".tmp-fe-build-{name}"));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("scratch directory");
            Scratch(dir)
        }

        fn path(&self, name: &str) -> String {
            self.0.join(name).display().to_string()
        }

        fn dir(&self) -> String {
            self.0.display().to_string()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A fetch that answers the attempts in order: Ok(bytes) writes them to the file, Err(message) fails.
    fn scripted<'a>(outcomes: &'a [Result<&'static [u8], &'static str>], calls: &'a Cell<usize>) -> impl FnMut(&str, &str) -> Result<(), String> + 'a {
        move |_url: &str, part: &str| -> Result<(), String> {
            let n = calls.get();
            calls.set(n + 1);
            match outcomes.get(n).ok_or("no outcome for this attempt")? {
                Ok(bytes) => fs::write(part, bytes).map_err(|e| e.to_string()),
                Err(message) => Err(message.to_string()),
            }
        }
    }

    #[test]
    fn blend2d_fetch_retries_then_succeeds() {
        let dir = Scratch::new("retry");
        let tarball = dir.path("blend2d.tar.gz");
        let outcomes = [Err("curl exit status: 22: curl: (22) The requested URL returned error: 403"), Err("curl exit status: 28: curl: (28) timed out"), Ok(EMPTY)];
        let calls = Cell::new(0);
        let mut waits = Vec::new();
        let result = blend2d_fetch_with(URL, &tarball, EMPTY_SHA, &mut scripted(&outcomes, &calls), &mut |wait: Duration| waits.push(wait));
        assert_eq!(result, Ok(()));
        assert_eq!(calls.get(), 3);
        assert_eq!(waits, [Duration::from_secs(10), Duration::from_secs(30)]);
        assert_eq!(fs::read(&tarball).expect("the tarball"), EMPTY);
        assert!(!Path::new(&dir.path("blend2d.tar.gz.part")).exists());
    }

    #[test]
    fn blend2d_fetch_gives_up_after_four_attempts() {
        let dir = Scratch::new("give-up");
        let tarball = dir.path("blend2d.tar.gz");
        let outcomes = [Err("curl exit status: 6: curl: (6) Could not resolve host"); 4];
        let calls = Cell::new(0);
        let mut waits = Vec::new();
        let result = blend2d_fetch_with(URL, &tarball, EMPTY_SHA, &mut scripted(&outcomes, &calls), &mut |wait: Duration| waits.push(wait));
        assert!(result.is_err());
        assert_eq!(calls.get(), 4);
        assert_eq!(waits, [Duration::from_secs(10), Duration::from_secs(30), Duration::from_secs(60)]);
        assert!(!Path::new(&tarball).exists());
        assert!(!Path::new(&dir.path("blend2d.tar.gz.part")).exists());
    }

    #[test]
    fn blend2d_fetch_retries_on_a_sha_mismatch() {
        let dir = Scratch::new("sha-retry");
        let tarball = dir.path("blend2d.tar.gz");
        let outcomes = [Ok(BAD), Ok(EMPTY)];
        let calls = Cell::new(0);
        let mut waits = Vec::new();
        let result = blend2d_fetch_with(URL, &tarball, EMPTY_SHA, &mut scripted(&outcomes, &calls), &mut |wait: Duration| waits.push(wait));
        assert_eq!(result, Ok(()));
        assert_eq!(calls.get(), 2);
        assert_eq!(waits, [Duration::from_secs(10)]);
        assert_eq!(fs::read(&tarball).expect("the tarball"), EMPTY);
    }

    #[test]
    fn blend2d_fetch_reuses_a_matching_tarball() {
        let dir = Scratch::new("reuse");
        let tarball = dir.path("blend2d.tar.gz");
        fs::write(&tarball, EMPTY).expect("the tarball");
        let calls = Cell::new(0);
        let mut waits = Vec::new();
        let result = blend2d_fetch_with(URL, &tarball, EMPTY_SHA, &mut scripted(&[], &calls), &mut |wait: Duration| waits.push(wait));
        assert_eq!(result, Ok(()));
        assert_eq!(calls.get(), 0);
        assert!(waits.is_empty());
    }

    #[test]
    fn blend2d_fetch_discards_a_mismatched_tarball() {
        let dir = Scratch::new("discard");
        let tarball = dir.path("blend2d.tar.gz");
        fs::write(&tarball, BAD).expect("the tarball");
        let outcomes = [Ok(EMPTY)];
        let calls = Cell::new(0);
        let mut waits = Vec::new();
        let result = blend2d_fetch_with(URL, &tarball, EMPTY_SHA, &mut scripted(&outcomes, &calls), &mut |wait: Duration| waits.push(wait));
        assert_eq!(result, Ok(()));
        assert!(calls.get() >= 1);
        assert_eq!(fs::read(&tarball).expect("the tarball"), EMPTY);
    }

    #[test]
    fn glue_matches_sed() {
        let input = include_str!("../tests/glue-in.js");
        let expected = include_str!("../tests/glue-sed.js");
        assert_eq!(glue(input), expected);
    }

    /// The engine plan for the two fixed dummy plugins (tests/plugins), so that the recorded text does not change when a plugin is added.
    /// The real plugins only have their stages checked (the names and the order below).
    #[test]
    fn the_real_plan_has_its_stages_in_order() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        std::env::set_current_dir(&root).expect("the repository root");
        let names: Vec<String> = plan("<bin>").expect("plan").iter().map(|s| s.name.split(" (").next().unwrap_or_default().to_string()).collect();
        assert_eq!(
            names,
            ["plugins", "lists", "lowered-all", "blend2d", "blend2d-compile", "blend2d-archive", "gen", "units", "compile", "hybrid-n0", "link", "wasm-opt", "hybrid"]
        );
    }

    /// The working directory is the repository root (the plans, ci/artifacts.lock and tools/ are read from there).
    fn at_root() {
        std::env::set_current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")).expect("the repository root");
    }

    /// The plans of the engine, bundle, extract and external commands equal the texts recorded before the `--plan` flag was removed
    /// (tests/plan/*.txt). The engine's binary directory is written as `<bin>`. The upload plan is tested by upload_plan_matches_recorded_text.
    /// The bundle has two plans: with the external plugins' installers (bundle.txt) and without them (bundle-no-external.txt).
    #[test]
    fn plans_match_recorded_texts() {
        at_root();
        let dummies: Vec<String> = plugins::read_plugins_from("lift/crates/fe-build/tests/plugins").expect("the dummy plugins").into_iter().map(|d| d.key).collect();
        let engine = engine_plan_lines("<bin>", &dummies).expect("engine plan");
        assert_eq!(engine.join("\n") + "\n", include_str!("../tests/plan/engine.txt"));
        let bundle = bundle_plan_steps(SHA, true);
        assert_eq!(stage_lines("bundle", &bundle).join("\n") + "\n", include_str!("../tests/plan/bundle.txt"));
        let bundle = bundle_plan_steps(SHA, false);
        assert_eq!(stage_lines("bundle", &bundle).join("\n") + "\n", include_str!("../tests/plan/bundle-no-external.txt"));
        assert_eq!(stage_lines("extract", &extract_steps()).join("\n") + "\n", include_str!("../tests/plan/extract.txt"));
        assert_eq!(stage_lines("external", &external_steps().expect("external plan")).join("\n") + "\n", include_str!("../tests/plan/external.txt"));
    }

    #[test]
    fn external_lazy_files_are_left_out_without_their_installers() {
        let lazy = |v: &str| [String::from("--lazy-file"), v.to_string()];
        let args: Vec<String> = [lazy("lift/out-app/plugins/ott/vst-ext.wasm=plugins/ott.wasm"), lazy("lift/out-app/xmod/a.wasm=a.wasm")].concat();
        assert_eq!(keep_lazies(&args, true), args);
        assert_eq!(keep_lazies(&args, false), lazy("lift/out-app/xmod/a.wasm=a.wasm").to_vec());
    }

    #[test]
    fn external_absent_leaves_the_plugins_out() {
        at_root();
        let dir = Scratch::new("external-absent");
        assert_eq!(external_in(&dir.dir(), &external_names().expect("ci/artifacts.lock")), Ok(false));
    }

    #[test]
    fn external_partly_present_stops() {
        at_root();
        let dir = Scratch::new("external-partly");
        fs::write(dir.path("VitalInstaller.exe"), EMPTY).expect("one installer");
        assert!(external_in(&dir.dir(), &external_names().expect("ci/artifacts.lock")).is_err());
    }

    #[test]
    fn upload_plan_matches_recorded_text() {
        let mut upload = upload_sync_steps();
        upload.push(upload_manifest_step());
        assert_eq!(stage_lines("upload", &upload).join("\n") + "\n", include_str!("../tests/plan/upload.txt"));
    }
}
