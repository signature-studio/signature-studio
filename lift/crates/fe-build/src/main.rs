//! fe-build (decision 0412): the engine steps of deploy.yml as one program. Each process gets the arguments the
//! deploy step gave it, and the working directory is the repository root, so the source paths the compiler sees
//! (and the strings it puts in the wasm) are the same as before. The lists and argument files stay where they are.
//!
//! usage: fe-build engine | extract | bundle | upload | endpoint
//! The plans (the stages and steps in run order) are functions that the tests compare with tests/plan/*.txt.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const OUT_APP_DIR: &str = "lift/out-app";
const HARNESS_DIR: &str = "lift/harness";
/// Issue #1362: the FL Cloud offline page of the installer (System\Internet, taken with the resources of an Issue that has the `resources` label). A lazy file of the bundle when it is there: the
/// Browser panel's face shows it after the Navigate of the cloud tab (app/src/ui/webview2-surface.ts). Nothing is packed when it is not there.
const OFFLINE_PAGE_SRC: &str = "lift/out-app/host-extra/resources/System/Internet/error-offline.html";
const OFFLINE_PAGE_PATH: &str = "resources/System/Internet/error-offline.html";
const R2_BUCKET: &str = "s3://fl-web-assets/";
const INSTALLER: &str = "s3://fl-web-artifacts/installers/flstudio_win64_26.1.7.5653.exe";
const BLEND2D_URL: &str = "https://github.com/blend2d/blend2d.git";
const BLEND2D_REV: &str = "a7f947677e85441abe1769586d3bb935b4086a07";
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
    RunStdin(Vec<String>, String),
    RemoveFile(String),
    CheckSources(String),
    CopyDlls,
    Copy(String, String),
    Glue,
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
            Step::RunStdin(argv, stdin_file) => format!("{} < {stdin_file}", argv.join(" ")),
            Step::RemoveFile(path) => format!("rm -f {path}"),
            Step::CheckSources(list_file) => format!("check {list_file} against the .cpp files under {BLEND2D_SRC}"),
            Step::CopyDlls => format!("copy {OUT_APP_DIR}/gen/dll/*.dll and {OUT_APP_DIR}/host/binaries/win64/FLEngine_x64.dll to {OUT_APP_DIR}/dll/"),
            Step::Copy(from, to) => format!("copy {from} to {to}"),
            Step::Glue => format!("glue {OUT_APP_DIR}/web/flengine6.js -> {OUT_APP_DIR}/web/engine-glue.js (the two rules of {HARNESS_DIR}/web/glue.sed)"),
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

fn blend2d_clone() -> Vec<Step> {
    let src = format!("{OUT_APP_DIR}/b2d-src");
    vec![
        Step::Mkdir(vec![OUT_APP_DIR.into()]),
        Step::Run(vec!["git".into(), "clone".into(), "-q".into(), BLEND2D_URL.into(), src.clone()]),
        Step::Run(vec!["git".into(), "-C".into(), src.clone(), "checkout".into(), "-q".into(), BLEND2D_REV.into()]),
        Step::RunStdin(
            vec!["patch".into(), "-p1".into(), "-d".into(), src.clone()],
            format!("{HARNESS_DIR}/b2d1/b2d3-compop.patch"),
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
    let fe_lift = format!("{bin}/fe-lift");
    let fe_wasmgen = format!("{bin}/fe-wasmgen");
    let units = blend2d_units()?;
    let mut blend2d_steps = blend2d_clone();
    blend2d_steps.push(Step::Mkdir(blend2d_dirs(&units)));
    let mut stages = vec![Stage { name: "blend2d".into(), steps: blend2d_steps, parallel: false }];
    stages.push(Stage { name: "blend2d-compile".into(), steps: blend2d_compile_steps(&units), parallel: true });
    stages.push(Stage { name: "blend2d-archive".into(), steps: blend2d_archive_steps(&units), parallel: false });

    let mut gen_steps = vec![Step::Mkdir(ENGINE_DIRS.iter().map(|dir| format!("{OUT_APP_DIR}/{dir}")).collect())];
    let mut gen_args = vec![fe_lift, "gen-all".into()];
    gen_args.extend(lines(&format!("{HARNESS_DIR}/b2d2/gen.args"))?);
    gen_steps.push(Step::Run(gen_args));
    gen_steps.push(Step::CopyDlls);
    stages.push(Stage { name: "gen".into(), steps: gen_steps, parallel: false });

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

    let mut hybrid_argv = vec![fe_wasmgen];
    hybrid_argv.extend(lines(&format!("{HARNESS_DIR}/hyb/hybrid.cmd"))?);
    let func_names = format!("{OUT_APP_DIR}/hyb/wasm-func-names.tsv");
    stages.push(Stage { name: "hybrid".into(), steps: vec![Step::Run(hybrid_argv), Step::Copy(func_names, format!("{OUT_APP_DIR}/gen/wasm-func-names.tsv")), Step::Glue], parallel: false });
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
        Step::RunStdin(argv, stdin_file) => {
            let file = fs::File::open(stdin_file).map_err(|e| format!("{stdin_file}: {e}"))?;
            exec(Command::new(&argv[0]).args(&argv[1..]).stdin(Stdio::from(file)), &argv[0])
        }
        Step::RemoveFile(path) => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("rm {path}: {error}")),
        },
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
/// The stages and steps of the engine build in run order, as text (one stage line, then its steps).
fn engine_plan_lines(bin: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for stage in plan(bin)? {
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
        if stage.parallel {
            run_parallel(&stage.steps)?;
        } else {
            for step in &stage.steps {
                run(step)?;
            }
        }
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

fn seed_step() -> Result<Step, String> {
    let mut argv: Vec<String> = vec!["node".into(), "app/scripts/pack-seed.mjs".into()];
    argv.extend(words(&format!("{HARNESS_DIR}/web/seed.args"))?);
    Ok(Step::Run(argv))
}

/// The pack step for the engine with the sha256 `sha` (its first 12 digits name the version).
fn pack_step_for(sha: &str) -> Result<Step, String> {
    let mut argv: Vec<String> = vec!["node".into(), "app/scripts/pack-assets.mjs".into(), "--version".into(), sha[..12].into()];
    argv.extend(lines(&format!("{HARNESS_DIR}/web/pack.args"))?);
    if Path::new(OFFLINE_PAGE_SRC).is_file() {
        argv.push("--lazy-file".into());
        argv.push(format!("{OFFLINE_PAGE_SRC}={OFFLINE_PAGE_PATH}"));
    }
    Ok(Step::Run(argv))
}

#[cfg(test)]
fn bundle_plan_steps(sha: &str) -> Result<Vec<Step>, String> {
    Ok(vec![seed_step()?, pack_step_for(sha)?])
}

fn bundle() -> Result<(), String> {
    run(&seed_step()?)?;
    let sha = sha256_hex(&format!("{OUT_APP_DIR}/web/engine.wasm"))?;
    run(&pack_step_for(&sha)?)
}

fn aws_step(args: &[&str]) -> Step {
    let mut argv = vec!["aws".to_string()];
    argv.extend(args.iter().map(|arg| arg.to_string()));
    Step::Run(argv)
}

/// The three `aws s3` calls that do not depend on a gate: the Brotli bundle and the raw bundle.
fn upload_sync_steps() -> Vec<Step> {
    let br_dir = format!("{OUT_APP_DIR}/bundle/br");
    let raw_dir = format!("{OUT_APP_DIR}/bundle/raw");
    vec![
        aws_step(&["s3", "sync", &br_dir, R2_BUCKET, "--content-encoding", "br", "--cache-control", "max-age=31536000, immutable"]),
        aws_step(&["s3", "sync", &raw_dir, R2_BUCKET, "--cache-control", "no-cache"]),
    ]
}

/// The copies of the manifest: the engine's sha256 must be a line of verified.txt, and a non-empty root.txt names
/// the key that is copied over the manifest.
fn upload_gated_steps(sha: &str, verified: &str, root: &str) -> Vec<Step> {
    let manifest_key = format!("{R2_BUCKET}manifest.json");
    let mut steps = Vec::new();
    if verified.lines().any(|line| line == sha) {
        let manifest_file = format!("{OUT_APP_DIR}/bundle/raw/fl-Oz/manifest.json");
        steps.push(aws_step(&["s3", "cp", &manifest_file, &manifest_key, "--cache-control", "no-cache", "--content-type", "application/json"]));
    }
    if !root.is_empty() {
        let src = format!("{R2_BUCKET}{}", root.trim_end_matches('\n'));
        steps.push(aws_step(&["s3", "cp", &src, &manifest_key, "--cache-control", "no-cache", "--content-type", "application/json"]));
    }
    steps
}

fn upload() -> Result<(), String> {
    for step in &upload_sync_steps() {
        run(step)?;
    }
    let sha = sha256_hex(&format!("{OUT_APP_DIR}/web/engine.wasm"))?;
    let verified = fs::read_to_string(format!("{HARNESS_DIR}/web/verified.txt")).map_err(|e| format!("verified.txt: {e}"))?;
    let root = fs::read_to_string(format!("{HARNESS_DIR}/web/root.txt")).unwrap_or_default();
    for step in &upload_gated_steps(&sha, &verified, &root) {
        run(step)?;
    }
    Ok(())
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
            "ci/extract-dlls.tsv".into(),
            "--seed".into(),
            format!("{HARNESS_DIR}/seeds/vfs-seed-idle1.tsv"),
        ]),
    ]
}

fn extract() -> Result<(), String> {
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
        Some("bundle") => bundle(),
        Some("upload") => upload(),
        Some("endpoint") => endpoint(),
        _ => {
            eprintln!("usage: fe-build engine | extract | bundle | upload | endpoint");
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
    use super::{bundle_plan_steps, engine_plan_lines, extract_steps, glue, stage_lines, upload_gated_steps, upload_sync_steps};
    use std::path::Path;

    const SHA: &str = "b440e13a623aa949ccce21db53566658425743bb2b21263bcf773d278396e043";

    #[test]
    fn glue_matches_sed() {
        let input = include_str!("../tests/glue-in.js");
        let expected = include_str!("../tests/glue-sed.js");
        assert_eq!(glue(input), expected);
    }

    /// The plans of the four commands equal the texts recorded before the `--plan` flag was removed (tests/plan/*.txt).
    /// The engine's binary directory is written as `<bin>`. The upload gate takes the sha, verified.txt and root.txt as text.
    #[test]
    fn plans_match_recorded_texts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        std::env::set_current_dir(&root).expect("the repository root");
        let engine = engine_plan_lines("<bin>").expect("engine plan");
        assert_eq!(engine.join("\n") + "\n", include_str!("../tests/plan/engine.txt"));
        let bundle = bundle_plan_steps(SHA).expect("bundle plan");
        assert_eq!(stage_lines("bundle", &bundle).join("\n") + "\n", include_str!("../tests/plan/bundle.txt"));
        let mut upload = upload_sync_steps();
        upload.extend(upload_gated_steps(SHA, SHA, ""));
        assert_eq!(stage_lines("upload", &upload).join("\n") + "\n", include_str!("../tests/plan/upload.txt"));
        assert_eq!(stage_lines("extract", &extract_steps()).join("\n") + "\n", include_str!("../tests/plan/extract.txt"));
    }
}
