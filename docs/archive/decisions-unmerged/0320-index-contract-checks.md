| # | 項目 | Learn の契約 | 期待 | 試作の結果 | 判定 |
|---|---|---|---|---|---|
| T1 | enumeration order | Learn: FindFirstFile does no sorting, the order is not guaranteed and depends on the file system; NTFS usually returns alphabetical order (FindNextFileW remarks) | NTFS order: upper-cased UTF-16 code units, ordinal | 1347 directories with >= 2 entries; order differs from sort-by-lower in 27, from plain code-point sort in 251 | ok |
| T1b | example where the order matters | (same) | NTFS order differs from lower-case order | in "Data/Patches/Misc/Used by demo projects/Astes - Bien Duro": lower-case order 'PL_RV_04_Kick_Sample_777152.ogg' vs NTFS order 'PLRAD_02_Snare_Sample_904404.ogg' | ok |
| T2 | '.' and '..' | not mentioned on the FindFirstFileW page; NTFS (known behaviour, not Learn): a non-root directory listing starts with "." and ".." | '.' , '..' first, then entries | first 4: ['.', '..', 'Advanced', 'Empty'] | ok |
| T2b | mount root | (same; the mount root has a parent, "Image-Line") | '.' , '..' first | first 3: ['.', '..', 'Artwork'] | ok |
| T3 | directory name without wildcard | Learn: "an argument of C:\Windows returns information about the directory C:\Windows, not about a directory or file in it" | one entry: the directory itself, attribute DIRECTORY | 'Templates', attrs 0x10 | ok |
| T4 | trailing backslash | Learn: "An attempt to open a search with a trailing backslash always fails" (the error code is not specified) | fails | handle=None, error=123 | ok |
| T5a | no match in an existing directory | Learn: ERROR_FILE_NOT_FOUND | 2 | 2 | ok |
| T5b | directory does not exist | Learn FindFirstFile page names ERROR_FILE_NOT_FOUND for "no matching files"; the path error for a missing directory is the usual Windows behaviour (not on the page) | 3 (ERROR_PATH_NOT_FOUND) | 3 | ok |
| T5c | missing file without wildcard | Learn: ERROR_FILE_NOT_FOUND | 2 | 2 | ok |
| T6 | end of enumeration | Learn FindNextFileW: ERROR_NO_MORE_FILES | 18 | 18 | ok |
| T7 | "*.*" matches names without a period | not stated on the Learn pages; known Win32 behaviour of FindFirstFile | "*.*" == "*" | directory 'Plugins/Fruity/Generators/BassDrum': 7 vs 7 entries | ok |
| T8 | "*.wv" in Kicks | Learn: wildcards * and ?; the search is on the name only | 349 files, no "." / ".." | 349 entries | ok |
| T9 | case-insensitive search | Naming Files page (not fetched): names are case-insensitive on NTFS by default (known behaviour) | same 349 entries | 349 entries | ok |
| T10 | separators, "..", trailing dot | Win32 path normalisation (GetFullPathName; known behaviour) | same listing as Data\Templates\* | 9 entries | ok |
| T11 | "?" wildcard | Learn: "?" in the file name | any name with >= 1 character, plus "." and ".." (they match "?*"?) | 9 entries: ['.', '..', 'Advanced'] (whether Windows matches "." and ".." with "?*" is not verified) | OPEN |
| T12 | GetFileAttributesW Data\Templates | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0x10 | 0x10, error None | ok |
| T12 | GetFileAttributesW FL64.exe | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0x20 | 0x20, error None | ok |
| T12 | GetFileAttributesW Data\Nope | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0xffffffff + error 2 or 3 | 0xffffffff, error 2 | ok |
| T12 | GetFileAttributesW Nope\x | Learn: attributes of the file or directory; INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) on failure with GetLastError | 0xffffffff + error 2 or 3 | 0xffffffff, error 3 | ok |
| T13 | WIN32_FIND_DATA fields of a file | Learn: dwFileAttributes, ftCreationTime (zero if unsupported), ftLastAccessTime, ftLastWriteTime, nFileSizeHigh/Low, dwReserved0 only for reparse points, cFileName, cAlternateFileName | size split 32/32, write time = file time, reserved0 0 | size 0:597064, write FILETIME 134351659080000000, create FILETIME 134351659080000000, reserved0 0, alt '' | ok |
| T14 | creation / last-access time | Learn: creation time of the file; the real install time is NOT known from the copy | a fixed install time (the newest mtime of FL64.exe) is used | creation = access = 134351659080000000 | OPEN |
| T15 | non-ASCII names | ordering by $UpCase (known NTFS behaviour; Learn silent) | upper-cased code units; "ë" == "Ë" | 3 directories with non-ASCII names; in "Data/Demo projects/Demo songs": ... ['DEDZII - NIGHT TERROR.flp', 'Edlan & Ella Noël - Song For You.flp', 'Electroconductor - Inner Days - Original.flp'] | ok |
| T16 | short (8.3) names | Learn: the search includes the long and short file names; cAlternateFileName holds the 8.3 name (empty if none, always empty with FindExInfoBasic) | prototype returns an empty alternate name | 29274 of 29344 names are not 8.3 and would get a short name on a volume with 8.3 creation; extension collisions through the short name: ".html" files match "*.htm" too (1 real .htm files + 25 .html files), ".flms" files match "*.flm" too (11 real .flm files + 239 .flms files), ".data" files match "*.dat" too (55 real .dat files + 2 .data files), ".flmpst" files match "*.flm" too (11 real .flm files + 581 .flmpst files), ".flmrack" files match "*.flm" too (11 real .flm files + 13 .flmrack files), ".flmapping" files match "*.flm" too (11 real .flm files + 11 .flmapping files) | OPEN |
| T17 | path length | Learn: MAX_PATH (260) unless \\?\ or the long-path opt-in | all paths below the limit | longest relative path 158 chars (+41 root) | ok |

21 ok, 0 FAIL, 3 OPEN
