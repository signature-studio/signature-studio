//! The response row that decision 0372 (R5-3h: ole32!CoInitializeEx, reached by GetProcAddress) appends after the last row of HEAD a03c903,
//! user32!SetWindowPlacement of decision 0267 (tests/rows_0267) (decision 0015: append only, no pseudo-VA moves). Shared by the tests that pin
//! the rows before it (wall_fr4, wall_fr5, wall_fr13, wall_fr15, wall_fr16, wall_fr19, wall_fr20); a new row of decision 0372 is added here and
//! at the end of src/imports_table.rs.
#![allow(dead_code)]

pub const ROWS_0372: [(&str, &str); 1] = [("ole32.dll", "CoInitializeEx")];
