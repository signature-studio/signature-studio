//! Issue #1537: the tables that hold the real modules (the fixed ones and the plugin DLLs of lift/plugins) hold all of them at once.
//! Each table is a `#define` of the runtime's sources, read here as text (the same way fe-build's limit checks read them).

use std::fs;
use std::path::Path;

const RUNTIME_MODULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtime/fle_realmod.c");
const RUNTIME_IMAGES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtime/fle_vmap.c");
const RUNTIME_UNWINDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../runtime/fle_seh.c");
const FIXED_MODULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../harness/b2d2/modules.tsv");
const PLUGINS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins");

/// The value of `#define <name> <number>` in a source file (a `u` suffix is allowed).
fn define(path: &str, name: &str) -> u64 {
    let src = fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let needle = format!("#define {name} ");
    let line = src.lines().map(str::trim_start).find(|l| l.starts_with(&needle)).unwrap_or_else(|| panic!("{path}: no #define {name}"));
    let digits: String = line[needle.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or_else(|_| panic!("{path}: {name} is not a number: {line}"))
}

fn rows(text: &str) -> usize {
    text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#') && !l.starts_with("key\t")).count()
}

/// The real modules of the build: the fixed rows, and one row per plugin DLL or exe of each lift/plugins/<folder>/plugin.tsv.
fn real_modules() -> usize {
    let fixed = rows(&fs::read_to_string(FIXED_MODULES).unwrap_or_else(|e| panic!("{FIXED_MODULES}: {e}")));
    let mut plugins = 0;
    for entry in fs::read_dir(Path::new(PLUGINS)).unwrap_or_else(|e| panic!("{PLUGINS}: {e}")) {
        let file = entry.unwrap().path().join("plugin.tsv");
        if file.is_file() {
            plugins += rows(&fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display())));
        }
    }
    fixed + plugins
}

/// The real modules fit fle_realmod.c's table (the build's own check says the same: fe-build plugins.rs, LIMITS).
#[test]
fn the_real_modules_fit_the_module_table() {
    let limit = define(RUNTIME_MODULES, "RM_MAX");
    assert!(real_modules() as u64 <= limit, "{} real modules, the table holds {limit}", real_modules());
}

/// Every real module can be mapped at once, and the FLEngine's image is one more: the image table of VirtualQuery (fle_vmap.c) takes
/// them all. With 16, the image that follows the fifteenth real module stops the run (#1537: a session with more images than that).
#[test]
fn the_image_table_holds_the_engine_and_every_real_module() {
    let modules = define(RUNTIME_MODULES, "RM_MAX");
    let images = define(RUNTIME_IMAGES, "VQ_MAX_IMAGES");
    assert!(images >= modules + 1, "VQ_MAX_IMAGES is {images}: the FLEngine's image and {modules} real modules do not fit");
}

/// The unwinder (fle_seh.c) takes every image of the real modules too. A table that is full stops by name, and does not drop the image
/// (a dropped image is a frame the unwinder cannot find: #1537, the table of 12 dropped the thirteenth image of a session).
#[test]
fn the_unwinder_holds_every_real_module_and_stops_by_name_when_full() {
    let modules = define(RUNTIME_MODULES, "RM_MAX");
    let images = define(RUNTIME_UNWINDER, "IMG_MAX");
    assert!(images >= modules + 1, "IMG_MAX is {images}: the exe and {modules} real modules do not fit");
    let src = fs::read_to_string(RUNTIME_UNWINDER).unwrap();
    assert!(src.contains("the image table of the unwinder is full (IMG_MAX)"), "a full image table must stop by name");
    assert!(!src.contains("nimgs < IMG_MAX"), "a full image table must not drop an image without a word");
}
