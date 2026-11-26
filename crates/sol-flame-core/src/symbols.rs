use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;
use object::{Object, ObjectSection, ObjectSymbol};
use serde::{Deserialize, Serialize};
use crate::{FlameCoreError, Result};

/// Categories of Solana execution frames for styling and analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameCategory {
    /// Native program business logic and math (Default: blue tones)
    Native,
    /// Serialization & deserialization: Borsh / Anchor `try_from_slice`, `try_deserialize` (Red / Orange)
    Deserialization,
    /// Cross-Program Invocations, e.g. `cpi:spl_token` (Purple tones)
    Cpi,
    /// Runtime syscalls like `sol_invoke_signed`, `sol_log_` (Cyan tones)
    Syscall,
    /// Solana 32KB Heap allocator calls (Yellow warning)
    HeapAlloc,
}

impl FrameCategory {
    pub fn classify(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.starts_with("cpi:") || lower.contains("cpi") || lower.contains("invoke_signed") || lower.contains("sol_invoke") {
            FrameCategory::Cpi
        } else if lower.contains("try_from_slice")
            || lower.contains("try_deserialize")
            || lower.contains("accountdeserialize")
            || lower.contains("borsh")
            || lower.contains("deserialize")
        {
            FrameCategory::Deserialization
        } else if lower.contains("sol_alloc_free")
            || lower.contains("alloc::")
            || lower.contains("bump_allocator")
            || lower.contains("heap")
        {
            FrameCategory::HeapAlloc
        } else if lower.starts_with("sol_") || lower.contains("syscall") {
            FrameCategory::Syscall
        } else {
            FrameCategory::Native
        }
    }

    pub fn color_palette_tag(&self) -> &'static str {
        match self {
            FrameCategory::Native => "native",
            FrameCategory::Deserialization => "deser",
            FrameCategory::Cpi => "cpi",
            FrameCategory::Syscall => "syscall",
            FrameCategory::HeapAlloc => "heap",
        }
    }
}

/// Source file and line number mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// A resolved stack frame representing either a physical function or an inlined helper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedFrame {
    pub symbol: String,
    pub demangled: String,
    pub location: Option<SourceLocation>,
    pub category: FrameCategory,
    pub is_inlined: bool,
}

/// Interval mapping for PC address ranges -> function symbols.
#[derive(Debug, Clone)]
pub struct SymbolRange {
    pub start: u64,
    pub end: u64,
    pub name: String,
    pub demangled: String,
    pub location: Option<SourceLocation>,
}

/// In-memory symbol map index built from DWARF and ELF tables.
#[derive(Debug, Default, Clone)]
pub struct SymbolMap {
    /// BTreeMap sorted by start PC for range lookups
    ranges: BTreeMap<u64, SymbolRange>,
    /// Fallback direct address -> symbol mapping (from .symtab)
    exact_symbols: BTreeMap<u64, String>,
}

impl SymbolMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_range(&mut self, start: u64, end: u64, name: String, location: Option<SourceLocation>) {
        let demangled = demangle_symbol(&name);
        self.ranges.insert(
            start,
            SymbolRange {
                start,
                end,
                name,
                demangled,
                location,
            },
        );
    }

    pub fn insert_exact(&mut self, addr: u64, name: String) {
        self.exact_symbols.insert(addr, name);
    }

    pub fn resolve_pc(&self, pc: u64) -> Option<ResolvedFrame> {
        // Find candidate range where start <= pc
        if let Some((_, range)) = self.ranges.range(..=pc).next_back() {
            if pc < range.end {
                let category = FrameCategory::classify(&range.demangled);
                return Some(ResolvedFrame {
                    symbol: range.name.clone(),
                    demangled: range.demangled.clone(),
                    location: range.location.clone(),
                    category,
                    is_inlined: false,
                });
            }
        }

        // Fallback to exact symbol mapping
        if let Some(name) = self.exact_symbols.get(&pc) {
            let demangled = demangle_symbol(name);
            let category = FrameCategory::classify(&demangled);
            return Some(ResolvedFrame {
                symbol: name.clone(),
                demangled,
                location: None,
                category,
                is_inlined: false,
            });
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty() && self.exact_symbols.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ranges.len() + self.exact_symbols.len()
    }
}

/// Demangles Rust/Itanium symbol names to clean human-readable form.
pub fn demangle_symbol(mangled: &str) -> String {
    let demangled = rustc_demangle::demangle(mangled);
    let s = demangled.to_string();
    if s.is_empty() {
        mangled.to_string()
    } else {
        // Strip trailing hash e.g. ::h1234abcd if present
        strip_symbol_hash(&s)
    }
}

fn strip_symbol_hash(s: &str) -> String {
    if let Some(pos) = s.rfind("::h") {
        if pos + 3 < s.len() && s[pos + 3..].chars().all(|c| c.is_ascii_hexdigit()) {
            return s[..pos].to_string();
        }
    }
    s.to_string()
}

/// DWARF Symbol Resolver that inspects compiled sBPF ELF files.
pub struct DwarfSymbolResolver {
    symbol_map: SymbolMap,
}

impl DwarfSymbolResolver {
    /// Load symbols and DWARF debug info from raw ELF bytes.
    pub fn from_elf_bytes(bytes: &[u8]) -> Result<Self> {
        let mut symbol_map = SymbolMap::new();

        let obj = object::File::parse(bytes)
            .map_err(|e| FlameCoreError::ElfParse(e.to_string()))?;

        // 1. Load ELF symbol table as baseline
        for sym in obj.symbols() {
            if let Ok(name) = sym.name() {
                if !name.is_empty() {
                    let addr = sym.address();
                    let size = sym.size();
                    if size > 0 {
                        symbol_map.insert_range(addr, addr + size, name.to_string(), None);
                    } else {
                        symbol_map.insert_exact(addr, name.to_string());
                    }
                }
            }
        }

        // 2. Load DWARF sections
        let endian = if obj.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        // Extract DWARF sections into endian slices
        let load_section = |id: gimli::SectionId| -> Result<Cow<'_, [u8]>> {
            match obj.section_by_name(id.name()) {
                Some(ref section) => section
                    .uncompressed_data()
                    .map_err(|e| FlameCoreError::DwarfParse(format!("Failed to uncompress {}: {}", id.name(), e))),
                None => Ok(Cow::Borrowed(&[][..])),
            }
        };

        let dwarf_cow = gimli::DwarfSections::load(&load_section)?;
        let dwarf = dwarf_cow.borrow(|section| gimli::EndianSlice::new(section.as_ref(), endian));

        // Parse compilation units and subprogram DIEs
        let mut iter = dwarf.units();
        while let Ok(Some(header)) = iter.next() {
            let unit = match dwarf.unit(header) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let mut entries = unit.entries();
            while let Ok(Some(entry)) = entries.next_dfs() {
                if entry.tag() == gimli::DW_TAG_subprogram {
                    let mut name = None;
                    let mut low_pc = None;
                    let mut high_pc = None;
                    let mut file_idx = None;
                    let mut line_no = None;

                    for attr in entry.attrs() {
                        match attr.name() {
                            gimli::DW_AT_linkage_name | gimli::DW_AT_MIPS_linkage_name => {
                                if let Ok(s) = dwarf.attr_string(&unit, attr.value()) {
                                    if let Ok(st) = s.to_string() {
                                        name = Some(st.to_string());
                                    }
                                }
                            }
                            gimli::DW_AT_name => {
                                if name.is_none() {
                                    if let Ok(s) = dwarf.attr_string(&unit, attr.value()) {
                                        if let Ok(st) = s.to_string() {
                                            name = Some(st.to_string());
                                        }
                                    }
                                }
                            }
                            gimli::DW_AT_low_pc => {
                                if let gimli::AttributeValue::Addr(addr) = attr.value() {
                                    low_pc = Some(addr);
                                }
                            }
                            gimli::DW_AT_high_pc => {
                                match attr.value() {
                                    gimli::AttributeValue::Addr(addr) => high_pc = Some(addr),
                                    gimli::AttributeValue::Udata(size) => {
                                        if let Some(low) = low_pc {
                                            high_pc = Some(low.saturating_add(size));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            gimli::DW_AT_decl_file => {
                                if let gimli::AttributeValue::FileIndex(idx) = attr.value() {
                                    file_idx = Some(idx);
                                }
                            }
                            gimli::DW_AT_decl_line => {
                                if let gimli::AttributeValue::Udata(l) = attr.value() {
                                    line_no = Some(l as u32);
                                }
                            }
                            _ => {}
                        }
                    }

                    if let (Some(sym_name), Some(start), Some(end)) = (name, low_pc, high_pc) {
                        if start < end {
                            let location = if let (Some(f_idx), Some(line)) = (file_idx, line_no) {
                                resolve_dwarf_filename(&dwarf, &unit, f_idx).map(|f| SourceLocation {
                                    file: f,
                                    line,
                                    column: 0,
                                })
                            } else {
                                None
                            };
                            symbol_map.insert_range(start, end, sym_name, location);
                        }
                    }
                }
            }
        }

        Ok(Self { symbol_map })
    }

    /// Load symbols from an ELF file on disk.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::from_elf_bytes(&bytes)
    }

    /// Resolve an instruction pointer PC into its resolved stack frame.
    pub fn resolve_pc(&self, pc: u64) -> Option<ResolvedFrame> {
        self.symbol_map.resolve_pc(pc)
    }

    pub fn symbol_map(&self) -> &SymbolMap {
        &self.symbol_map
    }
}

fn resolve_dwarf_filename<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R>,
    file_idx: u64,
) -> Option<String> {
    if let Some(ref line_program) = unit.line_program {
        let header = line_program.header();
        if let Some(file_entry) = header.file(file_idx) {
            if let Ok(s) = dwarf.attr_string(unit, file_entry.path_name()) {
                if let Ok(st) = s.to_string() {
                    return Some(st.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_classification() {
        assert_eq!(
            FrameCategory::classify("anchor_lang::AccountDeserialize::try_deserialize"),
            FrameCategory::Deserialization
        );
        assert_eq!(
            FrameCategory::classify("borsh::de::BorshDeserialize::deserialize"),
            FrameCategory::Deserialization
        );
        assert_eq!(
            FrameCategory::classify("cpi:spl_token::transfer"),
            FrameCategory::Cpi
        );
        assert_eq!(
            FrameCategory::classify("sol_invoke_signed_c"),
            FrameCategory::Cpi
        );
        assert_eq!(
            FrameCategory::classify("sol_alloc_free_"),
            FrameCategory::HeapAlloc
        );
        assert_eq!(
            FrameCategory::classify("my_program::math::calculate_fees"),
            FrameCategory::Native
        );
    }

    #[test]
    fn test_symbol_map_range_lookup() {
        let mut map = SymbolMap::new();
        map.insert_range(0x1000, 0x1050, "_ZN11anchor_lang18AccountDeserialize15try_deserializeE".to_string(), Some(SourceLocation {
            file: "anchor-lang/src/lib.rs".to_string(),
            line: 18,
            column: 4,
        }));
        map.insert_range(0x1050, 0x1200, "my_program::process_instruction".to_string(), None);

        let f1 = map.resolve_pc(0x1020).expect("should resolve");
        assert_eq!(f1.category, FrameCategory::Deserialization);
        assert!(f1.demangled.contains("AccountDeserialize::try_deserialize"));
        assert_eq!(f1.location.unwrap().file, "anchor-lang/src/lib.rs");

        let f2 = map.resolve_pc(0x1100).expect("should resolve");
        assert_eq!(f2.category, FrameCategory::Native);
        assert_eq!(f2.demangled, "my_program::process_instruction");

        assert!(map.resolve_pc(0x2000).is_none());
    }

    #[test]
    fn test_rust_demangling() {
        let mangled = "_ZN11anchor_lang18AccountDeserialize15try_deserialize17h5f2b8c9d0e1a2b3cE";
        let demangled = demangle_symbol(mangled);
        assert_eq!(demangled, "anchor_lang::AccountDeserialize::try_deserialize");
    }
}
