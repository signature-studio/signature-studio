//! The response rows that decision 0147 (virtual clock and concurrency) appends after the last row of HEAD 7b82424, user32!GetLayeredWindowAttributes of decision 0140 (tests/rows_0140) -- and, once decision 0144's
//! two rows (tests/rows_0144) are in the tree, after those (decision 0015: append only, no pseudo-VA moves). Shared by the tests that pin the rows before them (wall_fr4, wall_fr5, wall_fr13, wall_fr15, wall_fr16) and
//! by tests/wall_fr20.rs; a new row of decision 0147 is added here and at the end of src/imports_table.rs.
#![allow(dead_code)]

pub const ROWS_0147: [(&str, &str); 1] = [("user32.dll", "PostThreadMessageW")];
