//! Issue #1523: the name of a real module in harness/two/main_two.c. The harness matches a real module's file (`<key stem>.<base hex>.dll`) to
//! its entry of fle_realmod_table by the name that rm_table_name makes of the file (the key: the stem and ".dll", lower case). The name is
//! held in `want`. A key may be 255 characters (fe-build plugins.rs, KEY_MAX), so `want` takes 256 bytes with the NUL.
//!
//! rm_table_name is not copied here: its text is read from main_two.c and compiled with clang, with a `want` of the size that main_two.c
//! declares. main() is checked by its text: it passes `want` to rm_table_name and matches the table by strcmp.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The longest key (fe-build plugins.rs, KEY_MAX): the name fits `want` with its NUL.
const KEY_MAX: usize = 255;
const BASE: u64 = 0x6800000;

fn main_two_text() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/two/main_two.c")).unwrap()
}

/// The text of rm_table_name: from its first line to the `}` at the start of the line that closes it.
fn rm_table_name_text(src: &str) -> &str {
    let start = src.find("static int rm_table_name(").expect("main_two.c has rm_table_name");
    let end = src[start..].find("\n}\n").expect("the end of rm_table_name") + start + 3;
    &src[start..end]
}

/// The size of `want` in main() (`char want[N];`).
fn want_size(src: &str) -> usize {
    assert_eq!(src.matches("char want[").count(), 1, "main_two.c declares want once");
    let at = src.find("char want[").unwrap() + "char want[".len();
    let rest = &src[at..];
    rest[..rest.find(']').unwrap()].parse().unwrap()
}

/// A driver: rm_table_name as main_two.c has it, and a `want` of the size of main_two.c. One line per file name: the result, the key's length, the name.
fn driver(src: &str) -> String {
    format!(
        "#include <ctype.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\n{}\n\nint main(int argc, char **argv) {{\n    char want[{}];\n    for (int i = 1; i < argc; i++) {{\n        size_t len = 0;\n        int got = rm_table_name(argv[i], want, sizeof want, &len);\n        printf(\"%d %zu %s\\n\", got, len, got == 0 ? want : \"-\");\n    }}\n    return 0;\n}}\n",
        rm_table_name_text(src),
        want_size(src)
    )
}

/// The driver's output for the file names: (result, key length, name).
fn run_driver(tag: &str, src: &str, files: &[String]) -> Vec<(i32, usize, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../../.tmp-realmod-names-{tag}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let source = dir.join("driver.c");
    fs::write(&source, driver(src)).unwrap();
    let exe = dir.join("driver");
    let built = Command::new("clang").args(["-std=gnu11", "-O0", "-w"]).arg(&source).arg("-o").arg(&exe).output().expect("clang not found");
    let out = built.status.success().then(|| Command::new(&exe).args(files).output().unwrap());
    let _ = fs::remove_dir_all(&dir);
    assert!(built.status.success(), "clang: {}", String::from_utf8_lossy(&built.stderr));
    let out = out.unwrap();
    assert!(out.status.success(), "the driver failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| {
            let mut f = l.splitn(3, ' ');
            (f.next().unwrap().parse().unwrap(), f.next().unwrap().parse().unwrap(), f.next().unwrap().to_string())
        })
        .collect()
}

/// A key of `len` characters, lower case, in the form of the plugins' keys (`effects.<folder>.<file>.dll`).
fn key_of(len: usize) -> String {
    let head = "effects.fruity-formula-controller.fruity_formula_controller_x64";
    format!("{head}{}.dll", "x".repeat(len - head.len() - 4))
}

/// The file of a key at its base: the key without ".dll", the base hex, ".dll" (lift/out-plugins/ is where the extraction puts it).
fn rebased_file(key: &str) -> String {
    format!("lift/out-plugins/{}.{BASE:x}.dll", key.trim_end_matches(".dll"))
}

#[test]
fn the_names_of_100_and_255_characters_are_matched_by_their_file_names() {
    let src = main_two_text();
    assert_eq!(want_size(&src), KEY_MAX + 1, "main_two.c: char want[N], N the key's limit with its NUL");
    let keys = ["effects.fruity-formula-controller.fruity_formula_controller_x64.dll".to_string(), "effects.fruity-multiband-compressor.fruity_multiband_compressor_x64.dll".to_string(), key_of(100), key_of(KEY_MAX)];
    let files: Vec<String> = keys.iter().map(|k| rebased_file(k)).collect();
    let rows = run_driver("names", &src, &files);
    assert_eq!(rows.len(), keys.len());
    for (key, (rc, len, name)) in keys.iter().zip(rows) {
        assert_eq!((rc, len, name.as_str()), (0, key.len(), key.as_str()), "the name of {} characters", key.len());
    }
}

#[test]
fn a_name_of_256_characters_is_refused_by_its_length_and_a_file_without_a_base_is_not_a_length() {
    let src = main_two_text();
    let files = [rebased_file(&key_of(KEY_MAX + 1)), "lift/out-plugins/no-base-suffix.dll".to_string()];
    let rows = run_driver("refused", &src, &files);
    assert_eq!(rows[0].0, -2, "a 256 character name does not fit want");
    assert_eq!(rows[0].1, KEY_MAX + 1, "the length the refusal tells");
    assert_eq!(rows[1].0, -1, "a file without a base suffix");
}

#[test]
fn main_gives_want_to_rm_table_name_and_matches_the_table_by_its_name_and_tells_a_long_name_apart() {
    let src = main_two_text();
    assert!(src.contains("rm_table_name(rpath[k], want, sizeof want, &len)"), "main() passes want, sizeof want and the length");
    assert!(src.contains("strcmp(fle_realmod_table[i].name, want) == 0"), "main() matches the table by the name");
    assert!(src.contains("its name is %zu characters"), "a long name is stopped with its length, not as a file without a base suffix");
}
