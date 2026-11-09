//! The response rows that decision 0132 appends after the 581 rows of decision 0129 (decision 0015: append only, no pseudo-VA moves). Shared by tests/wall_fr11.rs, tests/wall_fr12.rs and
//! tests/wall_fr13.rs, which pin the rows before them; a new wall of decision 0132 that needs an import adds its row here and at the end of src/imports_table.rs.
#![allow(dead_code)]

pub const ROWS_0132: [(&str, &str); 7] = [
    ("gdi32.dll", "CreatePatternBrush"), // wall 19
    ("user32.dll", "ShowScrollBar"),     // wall 20
    ("kernel32.dll", "GetTempFileNameW"), // wall 22
    ("wininet.dll", "InternetOpenW"),     // wall 23
    ("wininet.dll", "InternetSetOptionW"),
    ("wininet.dll", "InternetOpenUrlW"),
    ("wininet.dll", "InternetCloseHandle"),
];
