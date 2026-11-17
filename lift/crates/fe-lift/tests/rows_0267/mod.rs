//! The response row that decision 0267 (minimize and restore: GetWindowPlacement / SetWindowPlacement) appends after the last row of HEAD 4c5d36e6, user32!PostThreadMessageW of decision 0147 (tests/rows_0147)
//! (decision 0015: append only, no pseudo-VA moves). Shared by the tests that pin the rows before it (wall_fr4, wall_fr5, wall_fr13, wall_fr15, wall_fr16, wall_fr19, wall_fr20);
//! a new row of decision 0267 is added here and at the end of src/imports_table.rs.
#![allow(dead_code)]

pub const ROWS_0267: [(&str, &str); 1] = [("user32.dll", "SetWindowPlacement")];
