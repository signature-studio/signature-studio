//! Bounds-checked reader for PE32+ (x86-64) images. Standard library only.
//!
//! Every read goes through checked arithmetic and is validated against the
//! input slice, so malformed or truncated input yields a `PeError` and never
//! a panic.

use std::fmt;

pub mod inittable;
pub mod rebase;

pub const MACHINE_AMD64: u16 = 0x8664;
const DOS_MAGIC: u16 = 0x5a4d;
const PE_SIGNATURE: u32 = 0x0000_4550;
const OPT_MAGIC_PE32_PLUS: u16 = 0x20b;
const OPT_FIXED_SIZE: u64 = 112;
const MAX_DATA_DIRS: u32 = 16;
const MAX_SECTIONS: u16 = 96;
const MAX_STRING: usize = 4096;
const MAX_IMPORT_DESCRIPTORS: usize = 4096;
const MAX_THUNKS: usize = 1 << 20;
const MAX_EXPORT_ENTRIES: u32 = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeError {
    Truncated { what: &'static str, offset: u64, len: u64, file_len: u64 },
    BadDosSignature(u16),
    BadPeSignature(u32),
    /// Optional header magic is not PE32+ (0x20b). 0x10b is PE32 and is rejected.
    NotPe32Plus { magic: u16 },
    UnsupportedMachine(u16),
    Malformed(&'static str),
    RvaUnmapped(u32),
    RvaNotFileBacked(u32),
    VaOutOfImage(u64),
    BadString { rva: u32 },
    LimitExceeded(&'static str),
}

impl fmt::Display for PeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeError::Truncated { what, offset, len, file_len } => write!(
                f,
                "truncated {what}: need {len} bytes at offset {offset:#x}, file length {file_len:#x}"
            ),
            PeError::BadDosSignature(v) => write!(f, "bad DOS signature {v:#06x}"),
            PeError::BadPeSignature(v) => write!(f, "bad PE signature {v:#010x}"),
            PeError::NotPe32Plus { magic } => {
                write!(f, "optional header magic {magic:#x} is not PE32+ (0x20b)")
            }
            PeError::UnsupportedMachine(m) => write!(f, "unsupported machine {m:#06x}"),
            PeError::Malformed(s) => write!(f, "malformed PE: {s}"),
            PeError::RvaUnmapped(r) => write!(f, "RVA {r:#x} is not mapped by any section or header"),
            PeError::RvaNotFileBacked(r) => write!(f, "RVA {r:#x} has no backing bytes in the file"),
            PeError::VaOutOfImage(v) => write!(f, "VA {v:#x} is outside the image"),
            PeError::BadString { rva } => write!(f, "bad or unterminated string at RVA {rva:#x}"),
            PeError::LimitExceeded(s) => write!(f, "limit exceeded: {s}"),
        }
    }
}

impl std::error::Error for PeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: u32,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalHeader {
    pub magic: u16,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub image_base: u64,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub number_of_rva_and_sizes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataDirectory {
    pub rva: u32,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub characteristics: u32,
}

/// IMAGE_SCN_CNT_CODE (Microsoft Learn, "PE Format", Section Flags): the section contains executable code.
pub const IMAGE_SCN_CNT_CODE: u32 = 0x0000_0020;
/// IMAGE_SCN_MEM_EXECUTE (same table): the section can be executed as code.
pub const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
/// IMAGE_SCN_MEM_WRITE (same table): the section can be written to.
pub const IMAGE_SCN_MEM_WRITE: u32 = 0x8000_0000;

impl Section {
    fn virtual_span(&self) -> u64 {
        if self.virtual_size != 0 { self.virtual_size as u64 } else { self.size_of_raw_data as u64 }
    }

    /// True if the section has IMAGE_SCN_MEM_EXECUTE ("can be executed as code"). The name is not looked at.
    /// A section with only IMAGE_SCN_CNT_CODE (see `has_code_content`) is not executable by this test.
    pub fn is_executable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_EXECUTE != 0
    }

    /// True if the section has IMAGE_SCN_CNT_CODE ("contains executable code"). Reported next to
    /// `is_executable` by the survey tools; the closure uses `is_executable`.
    pub fn has_code_content(&self) -> bool {
        self.characteristics & IMAGE_SCN_CNT_CODE != 0
    }

    /// True if the section has IMAGE_SCN_MEM_WRITE ("can be written to"). The name is not looked at.
    pub fn is_writable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_WRITE != 0
    }

    /// True if `rva` lies in the section's virtual range (the same range `Pe::slice_at_rva` searches).
    pub fn contains_rva(&self, rva: u32) -> bool {
        let r = rva as u64;
        r >= self.virtual_address as u64 && r < self.virtual_address as u64 + self.virtual_span()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub name: Option<String>,
    pub ordinal: u32,
    pub rva: u32,
    pub va: u64,
    pub forwarder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSymbol {
    Name { hint: u16, name: String },
    Ordinal(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportEntry {
    pub symbol: ImportSymbol,
    pub iat_rva: u32,
    pub iat_va: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub dll: String,
    pub original_first_thunk: u32,
    pub first_thunk: u32,
    pub entries: Vec<ImportEntry>,
}

pub mod dir {
    pub const EXPORT: usize = 0;
    pub const IMPORT: usize = 1;
    pub const RESOURCE: usize = 2;
    /// IMAGE_DIRECTORY_ENTRY_EXCEPTION: the `.pdata` array of RUNTIME_FUNCTION entries.
    pub const EXCEPTION: usize = 3;
    /// IMAGE_DIRECTORY_ENTRY_BASERELOC.
    pub const BASERELOC: usize = 5;
}

pub struct Pe<'a> {
    data: &'a [u8],
    pub file_header: FileHeader,
    pub optional: OptionalHeader,
    pub data_dirs: Vec<DataDirectory>,
    pub sections: Vec<Section>,
}

fn read_bytes<'a>(data: &'a [u8], off: u64, len: u64, what: &'static str) -> Result<&'a [u8], PeError> {
    let file_len = data.len() as u64;
    let end = off
        .checked_add(len)
        .ok_or(PeError::Truncated { what, offset: off, len, file_len })?;
    if end > file_len {
        return Err(PeError::Truncated { what, offset: off, len, file_len });
    }
    Ok(&data[off as usize..end as usize])
}

fn le16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn le64(b: &[u8], o: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(a)
}

impl<'a> Pe<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Pe<'a>, PeError> {
        let dos = read_bytes(data, 0, 64, "DOS header")?;
        let magic = le16(dos, 0);
        if magic != DOS_MAGIC {
            return Err(PeError::BadDosSignature(magic));
        }
        let lfanew = le32(dos, 0x3c) as u64;

        let nt = read_bytes(data, lfanew, 24, "NT signature and file header")?;
        let sig = le32(nt, 0);
        if sig != PE_SIGNATURE {
            return Err(PeError::BadPeSignature(sig));
        }
        let file_header = FileHeader {
            machine: le16(nt, 4),
            number_of_sections: le16(nt, 6),
            time_date_stamp: le32(nt, 8),
            pointer_to_symbol_table: le32(nt, 12),
            number_of_symbols: le32(nt, 16),
            size_of_optional_header: le16(nt, 20),
            characteristics: le16(nt, 22),
        };

        let opt_off = lfanew + 24;
        let opt_magic_bytes = read_bytes(data, opt_off, 2, "optional header magic")?;
        let opt_magic = le16(opt_magic_bytes, 0);
        if opt_magic != OPT_MAGIC_PE32_PLUS {
            return Err(PeError::NotPe32Plus { magic: opt_magic });
        }
        if file_header.machine != MACHINE_AMD64 {
            return Err(PeError::UnsupportedMachine(file_header.machine));
        }
        let opt_size = file_header.size_of_optional_header as u64;
        if opt_size < OPT_FIXED_SIZE {
            return Err(PeError::Malformed("SizeOfOptionalHeader smaller than PE32+ fixed part"));
        }
        let opt = read_bytes(data, opt_off, opt_size, "optional header")?;
        let nrva = le32(opt, 108);
        if nrva > MAX_DATA_DIRS {
            return Err(PeError::LimitExceeded("NumberOfRvaAndSizes > 16"));
        }
        if OPT_FIXED_SIZE + 8 * nrva as u64 > opt_size {
            return Err(PeError::Malformed("data directories exceed SizeOfOptionalHeader"));
        }
        let optional = OptionalHeader {
            magic: opt_magic,
            size_of_code: le32(opt, 4),
            size_of_initialized_data: le32(opt, 8),
            size_of_uninitialized_data: le32(opt, 12),
            address_of_entry_point: le32(opt, 16),
            base_of_code: le32(opt, 20),
            image_base: le64(opt, 24),
            section_alignment: le32(opt, 32),
            file_alignment: le32(opt, 36),
            size_of_image: le32(opt, 56),
            size_of_headers: le32(opt, 60),
            subsystem: le16(opt, 68),
            dll_characteristics: le16(opt, 70),
            number_of_rva_and_sizes: nrva,
        };
        let data_dirs = (0..nrva as usize)
            .map(|i| DataDirectory { rva: le32(opt, 112 + 8 * i), size: le32(opt, 116 + 8 * i) })
            .collect();

        if file_header.number_of_sections > MAX_SECTIONS {
            return Err(PeError::LimitExceeded("NumberOfSections > 96"));
        }
        let sec_off = opt_off + opt_size;
        let nsec = file_header.number_of_sections as u64;
        let sec_bytes = read_bytes(data, sec_off, 40 * nsec, "section table")?;
        let mut sections = Vec::with_capacity(nsec as usize);
        for i in 0..nsec as usize {
            let s = &sec_bytes[40 * i..40 * i + 40];
            let end = s[..8].iter().position(|&b| b == 0).unwrap_or(8);
            sections.push(Section {
                name: String::from_utf8_lossy(&s[..end]).into_owned(),
                virtual_size: le32(s, 8),
                virtual_address: le32(s, 12),
                size_of_raw_data: le32(s, 16),
                pointer_to_raw_data: le32(s, 20),
                characteristics: le32(s, 36),
            });
        }

        if optional.size_of_headers > optional.size_of_image {
            return Err(PeError::Malformed("SizeOfHeaders exceeds SizeOfImage"));
        }
        for s in &sections {
            if (s.virtual_address as u64) < optional.size_of_headers as u64 {
                return Err(PeError::Malformed("section virtual address overlaps headers"));
            }
        }

        Ok(Pe { data, file_header, optional, data_dirs, sections })
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Returns (file offset, contiguous bytes available from that offset).
    fn locate(&self, rva: u32) -> Result<(usize, usize), PeError> {
        let file_len = self.data.len() as u64;
        let r = rva as u64;
        if r < self.optional.size_of_headers as u64 {
            if r >= file_len {
                return Err(PeError::RvaNotFileBacked(rva));
            }
            let avail = (self.optional.size_of_headers as u64).min(file_len) - r;
            return Ok((r as usize, avail as usize));
        }
        for s in &self.sections {
            let va = s.virtual_address as u64;
            if r < va || r >= va + s.virtual_span() {
                continue;
            }
            let delta = r - va;
            let raw = s.size_of_raw_data as u64;
            if delta >= raw {
                return Err(PeError::RvaNotFileBacked(rva));
            }
            let off = s.pointer_to_raw_data as u64 + delta;
            if off >= file_len {
                return Err(PeError::RvaNotFileBacked(rva));
            }
            let avail = (raw - delta).min(file_len - off);
            return Ok((off as usize, avail as usize));
        }
        Err(PeError::RvaUnmapped(rva))
    }

    pub fn rva_to_offset(&self, rva: u32) -> Result<usize, PeError> {
        self.locate(rva).map(|(o, _)| o)
    }

    pub fn va_to_rva(&self, va: u64) -> Result<u32, PeError> {
        let rva = va.checked_sub(self.optional.image_base).ok_or(PeError::VaOutOfImage(va))?;
        u32::try_from(rva).map_err(|_| PeError::VaOutOfImage(va))
    }

    pub fn va_to_offset(&self, va: u64) -> Result<usize, PeError> {
        self.rva_to_offset(self.va_to_rva(va)?)
    }

    pub fn rva_to_va(&self, rva: u32) -> u64 {
        self.optional.image_base + rva as u64
    }

    /// `len` bytes at `rva`; the whole range must lie in one file-backed region.
    pub fn slice_at_rva(&self, rva: u32, len: usize) -> Result<&'a [u8], PeError> {
        let (off, avail) = self.locate(rva)?;
        if len > avail {
            return Err(PeError::Truncated {
                what: "RVA range",
                offset: off as u64,
                len: len as u64,
                file_len: self.data.len() as u64,
            });
        }
        Ok(&self.data[off..off + len])
    }

    pub fn cstring_at_rva(&self, rva: u32) -> Result<String, PeError> {
        let (off, avail) = self.locate(rva)?;
        let window = &self.data[off..off + avail.min(MAX_STRING + 1)];
        let n = window.iter().position(|&b| b == 0).ok_or(PeError::BadString { rva })?;
        if n > MAX_STRING {
            return Err(PeError::BadString { rva });
        }
        String::from_utf8(window[..n].to_vec()).map_err(|_| PeError::BadString { rva })
    }

    pub fn exports(&self) -> Result<Vec<Export>, PeError> {
        let d = match self.data_dirs.get(dir::EXPORT) {
            Some(d) if d.rva != 0 => *d,
            _ => return Ok(Vec::new()),
        };
        let h = self.slice_at_rva(d.rva, 40)?;
        let base = le32(h, 16);
        let n_funcs = le32(h, 20);
        let n_names = le32(h, 24);
        let aof = le32(h, 28);
        let aon = le32(h, 32);
        let aoo = le32(h, 36);
        if n_funcs > MAX_EXPORT_ENTRIES || n_names > MAX_EXPORT_ENTRIES {
            return Err(PeError::LimitExceeded("export entry count"));
        }
        let funcs = self.slice_at_rva(aof, 4 * n_funcs as usize)?;
        let names = self.slice_at_rva(aon, 4 * n_names as usize)?;
        let ords = self.slice_at_rva(aoo, 2 * n_names as usize)?;

        let mut name_of: Vec<Option<String>> = vec![None; n_funcs as usize];
        for i in 0..n_names as usize {
            let idx = le16(ords, 2 * i) as usize;
            if idx >= n_funcs as usize {
                return Err(PeError::Malformed("export name ordinal index out of range"));
            }
            name_of[idx] = Some(self.cstring_at_rva(le32(names, 4 * i))?);
        }

        let dir_start = d.rva as u64;
        let dir_end = dir_start + d.size as u64;
        let mut out = Vec::new();
        for (idx, name) in name_of.into_iter().enumerate() {
            let rva = le32(funcs, 4 * idx);
            if rva == 0 {
                continue;
            }
            let forwarder = if (rva as u64) >= dir_start && (rva as u64) < dir_end {
                Some(self.cstring_at_rva(rva)?)
            } else {
                None
            };
            out.push(Export {
                name,
                ordinal: base.checked_add(idx as u32).ok_or(PeError::Malformed("ordinal overflow"))?,
                rva,
                va: self.rva_to_va(rva),
                forwarder,
            });
        }
        Ok(out)
    }

    pub fn imports(&self) -> Result<Vec<Import>, PeError> {
        let d = match self.data_dirs.get(dir::IMPORT) {
            Some(d) if d.rva != 0 => *d,
            _ => return Ok(Vec::new()),
        };
        let mut out = Vec::new();
        let mut terminated = false;
        for i in 0..MAX_IMPORT_DESCRIPTORS {
            let at = (d.rva as u64)
                .checked_add(20 * i as u64)
                .and_then(|v| u32::try_from(v).ok())
                .ok_or(PeError::Malformed("import descriptor RVA overflow"))?;
            let desc = self.slice_at_rva(at, 20)?;
            if desc.iter().all(|&b| b == 0) {
                terminated = true;
                break;
            }
            let oft = le32(desc, 0);
            let name_rva = le32(desc, 12);
            let ft = le32(desc, 16);
            if ft == 0 {
                return Err(PeError::Malformed("import descriptor with FirstThunk == 0"));
            }
            let dll = self.cstring_at_rva(name_rva)?;
            let src = if oft != 0 { oft } else { ft };
            let mut entries = Vec::new();
            let mut done = false;
            for k in 0..MAX_THUNKS {
                let step = 8 * k as u64;
                let src_at = u32::try_from(src as u64 + step)
                    .map_err(|_| PeError::Malformed("thunk RVA overflow"))?;
                let iat_rva = u32::try_from(ft as u64 + step)
                    .map_err(|_| PeError::Malformed("IAT RVA overflow"))?;
                let v = le64(self.slice_at_rva(src_at, 8)?, 0);
                if v == 0 {
                    done = true;
                    break;
                }
                let symbol = if v >> 63 == 1 {
                    if (v & 0x7fff_ffff_ffff_0000) != 0 {
                        return Err(PeError::Malformed("ordinal import with reserved bits set"));
                    }
                    ImportSymbol::Ordinal((v & 0xffff) as u16)
                } else {
                    if v >> 31 != 0 {
                        return Err(PeError::Malformed("hint/name RVA exceeds 31 bits"));
                    }
                    let hn = v as u32;
                    let hint = le16(self.slice_at_rva(hn, 2)?, 0);
                    let name = self.cstring_at_rva(
                        hn.checked_add(2).ok_or(PeError::Malformed("hint/name RVA overflow"))?,
                    )?;
                    ImportSymbol::Name { hint, name }
                };
                entries.push(ImportEntry { symbol, iat_rva, iat_va: self.rva_to_va(iat_rva) });
            }
            if !done {
                return Err(PeError::LimitExceeded("import thunk count"));
            }
            out.push(Import { dll, original_first_thunk: oft, first_thunk: ft, entries });
        }
        if !terminated {
            return Err(PeError::LimitExceeded("import descriptor count"));
        }
        Ok(out)
    }
}

const MAX_RESOURCE_LEAVES: usize = 1 << 20;
/// RT_STRING resource type (Microsoft Learn, "Resource Types": MAKEINTRESOURCE(6)).
pub const RT_STRING: u32 = 6;

/// A Type, Name or Language identifier of a resource: an integer ID or a name string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceId {
    Id(u32),
    Name(String),
}

/// One leaf of the resource tree (Type, Name, Language), see "The .rsrc Section" in the PE format
/// specification (Microsoft Learn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLeaf {
    pub type_id: ResourceId,
    pub name: ResourceId,
    pub language: ResourceId,
    /// RVA of the 16-byte Resource Data Entry itself.
    pub entry_rva: u32,
    pub data_rva: u32,
    pub size: u32,
    pub codepage: u32,
}

/// One string of an RT_STRING block: its string ID, the language leaf it came from and its UTF-16 units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEntry {
    pub id: u32,
    pub language: u32,
    pub text: Vec<u16>,
}

/// Splits one RT_STRING block into its strings. `block` is the resource name (block number, from 1):
/// it holds the 16 strings with IDs `(block - 1) * 16 ..= (block - 1) * 16 + 15`. Each string is a
/// 16-bit length in UTF-16 code units followed by that many units, with no terminating NUL; an unused
/// ID is a zero length (Microsoft Learn, "Resource Types", String Table Resources). Unused IDs are not
/// returned. Bytes after the 16th string (padding) are ignored.
pub fn parse_string_block(block: u32, bytes: &[u8]) -> Result<Vec<(u32, Vec<u16>)>, PeError> {
    if block == 0 {
        return Err(PeError::Malformed("string block number 0"));
    }
    let first = (block - 1).checked_mul(16).ok_or(PeError::Malformed("string block number too large"))?;
    let mut out = Vec::new();
    let mut p = 0usize;
    for i in 0..16u32 {
        let len_b = bytes.get(p..p + 2).ok_or(PeError::Malformed("string block shorter than 16 strings"))?;
        let n = le16(len_b, 0) as usize;
        p += 2;
        let body = bytes.get(p..p + 2 * n).ok_or(PeError::Malformed("string block shorter than 16 strings"))?;
        p += 2 * n;
        if n > 0 {
            let text = (0..n).map(|k| le16(body, 2 * k)).collect();
            out.push((first + i, text));
        }
    }
    Ok(out)
}

impl<'a> Pe<'a> {
    fn res_slice(&self, d: &DataDirectory, off: u32, len: usize) -> Result<&'a [u8], PeError> {
        let end = off as u64 + len as u64;
        if end > d.size as u64 {
            return Err(PeError::Malformed("resource offset outside the resource directory"));
        }
        let rva = d.rva.checked_add(off).ok_or(PeError::Malformed("resource RVA overflow"))?;
        self.slice_at_rva(rva, len)
    }

    fn res_name(&self, d: &DataDirectory, off: u32) -> Result<String, PeError> {
        let n = le16(self.res_slice(d, off, 2)?, 0) as usize;
        let b = self.res_slice(d, off + 2, 2 * n)?;
        let units: Vec<u16> = (0..n).map(|k| le16(b, 2 * k)).collect();
        Ok(String::from_utf16_lossy(&units))
    }

    /// The entries of the directory table at `off`: (identifier, raw 4-byte offset field).
    fn res_table(&self, d: &DataDirectory, off: u32) -> Result<Vec<(ResourceId, u32)>, PeError> {
        let h = self.res_slice(d, off, 16)?;
        let nn = le16(h, 12) as usize;
        let ni = le16(h, 14) as usize;
        let ents = self.res_slice(d, off + 16, 8 * (nn + ni))?;
        let mut v = Vec::with_capacity(nn + ni);
        for i in 0..nn + ni {
            let name = le32(ents, 8 * i);
            let field = le32(ents, 8 * i + 4);
            let id = if i < nn {
                if name >> 31 == 0 {
                    return Err(PeError::Malformed("resource name entry without the string flag"));
                }
                ResourceId::Name(self.res_name(d, name & 0x7fff_ffff)?)
            } else {
                if name >> 31 != 0 {
                    return Err(PeError::Malformed("resource ID entry with the string flag"));
                }
                ResourceId::Id(name)
            };
            v.push((id, field));
        }
        Ok(v)
    }

    /// All leaves of the resource tree (data directory 2), in directory order. Offsets in the tree are
    /// relative to the start of the resource directory. The tree must have exactly three levels
    /// (Type, Name, Language): a data entry above the language level or a subdirectory below it is
    /// `Malformed`. No resource directory gives an empty list.
    pub fn resources(&self) -> Result<Vec<ResourceLeaf>, PeError> {
        let d = match self.data_dirs.get(dir::RESOURCE) {
            Some(d) if d.rva != 0 => *d,
            _ => return Ok(Vec::new()),
        };
        let mut out = Vec::new();
        for (t, tf) in self.res_table(&d, 0)? {
            if tf >> 31 == 0 {
                return Err(PeError::Malformed("resource data entry at the type level"));
            }
            for (n, nf) in self.res_table(&d, tf & 0x7fff_ffff)? {
                if nf >> 31 == 0 {
                    return Err(PeError::Malformed("resource data entry at the name level"));
                }
                for (l, lf) in self.res_table(&d, nf & 0x7fff_ffff)? {
                    if lf >> 31 != 0 {
                        return Err(PeError::Malformed("resource subdirectory below the language level"));
                    }
                    if out.len() >= MAX_RESOURCE_LEAVES {
                        return Err(PeError::LimitExceeded("resource leaf count"));
                    }
                    let e = self.res_slice(&d, lf, 16)?;
                    out.push(ResourceLeaf {
                        type_id: t.clone(),
                        name: n.clone(),
                        language: l,
                        entry_rva: d.rva + lf,
                        data_rva: le32(e, 0),
                        size: le32(e, 4),
                        codepage: le32(e, 8),
                    });
                }
            }
        }
        Ok(out)
    }

    /// The bytes of a resource leaf (`size` bytes at `data_rva`; must lie in one file-backed region).
    pub fn resource_data(&self, leaf: &ResourceLeaf) -> Result<&'a [u8], PeError> {
        self.slice_at_rva(leaf.data_rva, leaf.size as usize)
    }

    /// Every string of every RT_STRING block whose type, name and language are integer IDs, ordered by
    /// block then by string ID. Blocks with a name identifier are skipped.
    pub fn strings(&self) -> Result<Vec<StringEntry>, PeError> {
        let mut out = Vec::new();
        for leaf in self.resources()? {
            let (ResourceId::Id(RT_STRING), ResourceId::Id(block), ResourceId::Id(language)) =
                (&leaf.type_id, &leaf.name, &leaf.language)
            else {
                continue;
            };
            for (id, text) in parse_string_block(*block, self.resource_data(&leaf)?)? {
                out.push(StringEntry { id, language: *language, text });
            }
        }
        Ok(out)
    }
}

const MAX_RUNTIME_FUNCTIONS: u32 = 1 << 22;
const MAX_UNWIND_CHAIN: usize = 64;
const RUNTIME_FUNCTION_SIZE: u32 = 12;
/// UNWIND_INFO flags (Microsoft Learn, "x64 exception handling", struct UNWIND_INFO).
pub const UNW_FLAG_EHANDLER: u8 = 0x01;
pub const UNW_FLAG_UHANDLER: u8 = 0x02;
pub const UNW_FLAG_CHAININFO: u8 = 0x04;

/// One RUNTIME_FUNCTION: three image-relative 32-bit addresses. `end_rva` is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeFunction {
    pub begin_rva: u32,
    pub end_rva: u32,
    pub unwind_rva: u32,
}

/// The fixed part of an UNWIND_INFO and its tail: the exception handler RVA (UNW_FLAG_EHANDLER or
/// UNW_FLAG_UHANDLER) or the chained RUNTIME_FUNCTION (UNW_FLAG_CHAININFO). The unwind codes are not decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnwindInfo {
    pub version: u8,
    pub flags: u8,
    pub size_of_prolog: u8,
    pub count_of_codes: u8,
    pub frame_register: u8,
    pub frame_offset: u8,
    pub handler_rva: Option<u32>,
    pub chained: Option<RuntimeFunction>,
}

/// A `.pdata` entry with its chain resolved. `parent` is the RUNTIME_FUNCTION stored in this entry's
/// chained UNWIND_INFO; `primary` is the end of the chain (the entry whose UNWIND_INFO has no
/// UNW_FLAG_CHAININFO, which "points to the actual procedure entry point"). Without a chain both
/// are the entry itself (`parent` is `None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionEntry {
    pub function: RuntimeFunction,
    pub parent: Option<RuntimeFunction>,
    pub primary: RuntimeFunction,
}

impl FunctionEntry {
    /// True if this entry is a chained part (a noncontiguous piece or a shrink-wrapped region) of
    /// another function and so not a function start.
    pub fn is_chained(&self) -> bool {
        self.parent.is_some()
    }
}

impl<'a> Pe<'a> {
    /// The RUNTIME_FUNCTION array of the exception directory (data directory 3), in file order.
    /// No exception directory gives an empty list. A directory size that is not a multiple of 12 is
    /// `Malformed`.
    pub fn runtime_functions(&self) -> Result<Vec<RuntimeFunction>, PeError> {
        let d = match self.data_dirs.get(dir::EXCEPTION) {
            Some(d) if d.rva != 0 && d.size != 0 => *d,
            _ => return Ok(Vec::new()),
        };
        if d.size % RUNTIME_FUNCTION_SIZE != 0 {
            return Err(PeError::Malformed("exception directory size is not a multiple of 12"));
        }
        let n = d.size / RUNTIME_FUNCTION_SIZE;
        if n > MAX_RUNTIME_FUNCTIONS {
            return Err(PeError::LimitExceeded("runtime function count"));
        }
        let b = self.slice_at_rva(d.rva, d.size as usize)?;
        Ok((0..n as usize)
            .map(|i| RuntimeFunction {
                begin_rva: le32(b, 12 * i),
                end_rva: le32(b, 12 * i + 4),
                unwind_rva: le32(b, 12 * i + 8),
            })
            .collect())
    }

    /// Parses the UNWIND_INFO at `rva`. The low bit of the address of a RUNTIME_FUNCTION's unwind data
    /// is not described for x64 in the Microsoft Learn page, so an address with it set is `Malformed`.
    /// UNW_FLAG_CHAININFO together with UNW_FLAG_EHANDLER or UNW_FLAG_UHANDLER is `Malformed` (the page
    /// says they must be cleared). The version is returned as it is.
    pub fn unwind_info(&self, rva: u32) -> Result<UnwindInfo, PeError> {
        if rva & 1 != 0 {
            return Err(PeError::Malformed("unwind info address has the low bit set"));
        }
        let h = self.slice_at_rva(rva, 4)?;
        let flags = h[0] >> 3;
        let count = h[2];
        let tail = u32::try_from(rva as u64 + 4 + 2 * ((count as u64 + 1) & !1))
            .map_err(|_| PeError::Malformed("unwind info tail RVA overflow"))?;
        let mut info = UnwindInfo {
            version: h[0] & 7,
            flags,
            size_of_prolog: h[1],
            count_of_codes: count,
            frame_register: h[3] & 0x0f,
            frame_offset: h[3] >> 4,
            handler_rva: None,
            chained: None,
        };
        if flags & UNW_FLAG_CHAININFO != 0 {
            if flags & (UNW_FLAG_EHANDLER | UNW_FLAG_UHANDLER) != 0 {
                return Err(PeError::Malformed("chained unwind info with a handler flag"));
            }
            let t = self.slice_at_rva(tail, 12)?;
            info.chained = Some(RuntimeFunction {
                begin_rva: le32(t, 0),
                end_rva: le32(t, 4),
                unwind_rva: le32(t, 8),
            });
        } else if flags & (UNW_FLAG_EHANDLER | UNW_FLAG_UHANDLER) != 0 {
            info.handler_rva = Some(le32(self.slice_at_rva(tail, 4)?, 0));
        }
        Ok(info)
    }

    /// Every `.pdata` entry with its chain resolved, in file order. A chain longer than 64 links
    /// (this includes a cycle) is `LimitExceeded`; an unreadable UNWIND_INFO is the error of `unwind_info`.
    pub fn function_table(&self) -> Result<Vec<FunctionEntry>, PeError> {
        let mut out = Vec::new();
        for rf in self.runtime_functions()? {
            let parent = self.unwind_info(rf.unwind_rva)?.chained;
            let mut primary = rf;
            if let Some(mut cur) = parent {
                let mut links = 0usize;
                loop {
                    links += 1;
                    if links > MAX_UNWIND_CHAIN {
                        return Err(PeError::LimitExceeded("unwind info chain length"));
                    }
                    match self.unwind_info(cur.unwind_rva)?.chained {
                        Some(next) => cur = next,
                        None => break,
                    }
                }
                primary = cur;
            }
            out.push(FunctionEntry { function: rf, parent, primary });
        }
        Ok(out)
    }

    /// RVAs of the function starts: the `begin_rva` of the primary entry of every `.pdata` entry,
    /// ascending, without duplicates. A chained entry contributes its primary's start, not its own.
    pub fn function_starts(&self) -> Result<Vec<u32>, PeError> {
        let set: std::collections::BTreeSet<u32> =
            self.function_table()?.iter().map(|e| e.primary.begin_rva).collect();
        Ok(set.into_iter().collect())
    }

    /// `function_starts` as virtual addresses.
    pub fn function_start_vas(&self) -> Result<Vec<u64>, PeError> {
        Ok(self.function_starts()?.into_iter().map(|r| self.rva_to_va(r)).collect())
    }
}

const MAX_RELOC_BLOCKS_BYTES: u32 = 1 << 28;
const IMAGE_REL_BASED_ABSOLUTE: u16 = 0;
const IMAGE_REL_BASED_DIR64: u16 = 10;

impl<'a> Pe<'a> {
    /// VAs of the 8-byte slots that the base relocation directory (data directory 5) marks
    /// IMAGE_REL_BASED_DIR64, in file order. Padding entries (ABSOLUTE) are skipped; any other type is
    /// not a 64-bit pointer and is skipped too. No directory gives an empty list. A block whose size is
    /// below 8, odd, or past the directory is `Malformed`.
    pub fn dir64_relocation_vas(&self) -> Result<Vec<u64>, PeError> {
        let d = match self.data_dirs.get(dir::BASERELOC) {
            Some(d) if d.rva != 0 && d.size != 0 => *d,
            _ => return Ok(Vec::new()),
        };
        if d.size > MAX_RELOC_BLOCKS_BYTES {
            return Err(PeError::LimitExceeded("base relocation directory size"));
        }
        let b = self.slice_at_rva(d.rva, d.size as usize)?;
        let mut out = Vec::new();
        let mut o = 0usize;
        while o + 8 <= b.len() {
            let page = le32(b, o);
            let size = le32(b, o + 4) as usize;
            if size < 8 || size % 2 != 0 || o + size > b.len() {
                return Err(PeError::Malformed("base relocation block size"));
            }
            for k in 0..(size - 8) / 2 {
                let e = le16(b, o + 8 + 2 * k);
                let ty = e >> 12;
                if ty == IMAGE_REL_BASED_ABSOLUTE || ty != IMAGE_REL_BASED_DIR64 {
                    continue;
                }
                out.push(self.rva_to_va(page.wrapping_add((e & 0x0fff) as u32)));
            }
            o += size;
        }
        Ok(out)
    }
}
