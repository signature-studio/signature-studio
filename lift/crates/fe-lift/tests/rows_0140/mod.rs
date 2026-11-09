//! The response rows that decision 0140 (layered windows) appends after the 10 rows of decision 0136 (tests/wall_fr14.rs), which come after the 7 of decision 0132 (decision 0015: append only, no pseudo-VA moves).
//! Shared by the tests that pin the rows before them (wall_fr4, wall_fr5, wall_fr13) and by tests/wall_fr16.rs; a new row of decision 0140 is added here and at the end of src/imports_table.rs.
#![allow(dead_code)]

pub const ROWS_0140: [(&str, &str); 4] = [
    ("user32.dll", "UpdateLayeredWindow"),
    ("user32.dll", "UpdateLayeredWindowIndirect"),
    ("user32.dll", "SetLayeredWindowAttributes"),
    ("user32.dll", "GetLayeredWindowAttributes"),
];
