//! The response rows that decision 0144 (G8: open a project) appends after the 4 rows of decision 0140 (tests/rows_0140), which come after the 3 of decision 0139, the 10 of decision 0136 and the 7 of decision 0132
//! (decision 0015: append only, no pseudo-VA moves). Shared by the tests that pin the rows before them (wall_fr4, wall_fr5, wall_fr13, wall_fr15, wall_fr16) and by tests/wall_fr19.rs; a new row of decision 0144 is
//! added here and at the end of src/imports_table.rs.
#![allow(dead_code)]

pub const ROWS_0144: [(&str, &str); 2] = [
    ("shell32.dll", "SHAddToRecentDocs"),
    ("shcore.dll", "GetDpiForMonitor"),
];
