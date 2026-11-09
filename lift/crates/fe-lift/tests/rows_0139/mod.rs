//! The response rows that decision 0139 (docs/decisions/0139-audio-s0-s2.md) appends after the 7 rows of decision 0132 (rows_0132) and the 581 of decision 0129 (decision 0015: append only, no pseudo-VA moves). Shared by the tests that
//! pin the rows before them (wall_fr4/5/8/9/10/11/12) and by tests/wall_fr15.rs; a row that the audio lane adds goes here and at the end of src/imports_table.rs.
#![allow(dead_code)]

pub const ROWS_0139: [(&str, &str); 3] = [
    ("dsound.dll", "DirectSoundCreate"),
    ("dsound.dll", "DirectSoundCreate8"),
    ("dsound.dll", "DirectSoundEnumerateW"),
];
