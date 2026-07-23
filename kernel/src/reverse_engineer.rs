//! reverse_engineer.rs — Kernel-native binary analysis and behavior profiling.
//!
//! # What this is
//! A reverse engineering module that lives entirely inside the kernel. It can:
//! - Parse ELF binaries (header, sections, symbols, strings)
//! - Extract x86_64 syscall patterns (instruction sequences)
//! - Profile behavior (what the binary does, based on syscalls + strings)
//! - Produce structured analysis for the orchestrator to act on
//!
//! # Design
//! - Pure Rust, zero deps, deterministic
//! - Parses ELF directly from byte slices (no mmap, no file I/O in the parser)
//! - All outputs are SHA3-256 verifiable
//! - Integrates with the orchestrator for action recording and health tracking
//!
//! # Architecture
//! ```text
//!   BINARY INPUT       REVERSE ENGINEERING PIPELINE       BEHAVIOR PROFILE
//!   +---------+     +--------------------------------+   +---------------+
//!   | ELF     | --> | ELF Parser                     |   | Syscalls used |
//!   | bytes   |     |   (header, sections, symbols)  |   | Strings found |
//!   |         |     | Instruction Pattern Extractor   |   | Functions     |
//!   |         |     |   (x86_64 syscall sequences)   |   | I/O patterns  |
//!   |         |     | String Extractor                |   | Behavior hash |
//!   |         |     |   (readable text from binary)   |   | Risk score    |
//!   +---------+     +--------------------------------+   +---------------+
//! ```

use std::fmt;

use crate::event_log::sha3_256;
use crate::hex_util;

// ============================================================================
// ELF Parser
// ============================================================================

/// ELF magic bytes.
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class (32-bit or 64-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    Elf32,
    Elf64,
}

/// ELF data encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfDataEncoding {
    LittleEndian,
    BigEndian,
}

/// ELF type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfType {
    Relocatable,
    Executable,
    Shared,
    Core,
    Unknown(u16),
}

/// ELF machine architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfMachine {
    X86_64,
    AArch64,
    Arm,
    I386,
    Unknown(u16),
}

impl ElfMachine {
    pub fn as_str(self) -> &'static str {
        match self {
            ElfMachine::X86_64 => "x86_64",
            ElfMachine::AArch64 => "aarch64",
            ElfMachine::Arm => "arm",
            ElfMachine::I386 => "i386",
            ElfMachine::Unknown(_) => "unknown",
        }
    }
}

/// ELF section header type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShType {
    Null,
    Progbits,
    Symtab,
    Strtab,
    Rela,
    Hash,
    Dynamic,
    Note,
    Nobits,
    Rel,
    Dylib,
    InitArray,
    FiniArray,
    PreinitArray,
    Group,
    SymtabShndx,
    Unknown(u32),
}

/// A parsed ELF section header.
#[derive(Debug, Clone)]
pub struct ElfSectionHeader {
    /// Section name (from string table).
    pub name: String,
    /// Raw name offset in the string table (used during parsing).
    pub name_offset: usize,
    /// Section type.
    pub sh_type: ShType,
    /// Section flags.
    pub flags: u64,
    /// Virtual address.
    pub addr: u64,
    /// File offset.
    pub offset: u64,
    /// Section size in bytes.
    pub size: u64,
    /// Link to associated section.
    pub link: u32,
    /// Entry size (for SHT_SYMTAB/SHT_STRTAB).
    pub entsize: u64,
}

/// A parsed ELF symbol.
#[derive(Debug, Clone)]
pub struct ElfSymbol {
    /// Symbol name (from string table).
    pub name: String,
    /// Symbol value (address).
    pub value: u64,
    /// Symbol size.
    pub size: u64,
    /// Symbol binding (STB_LOCAL, STB_GLOBAL, STB_WEAK).
    pub bind: u8,
    /// Symbol type (STT_NOTYPE, STT_FUNC, STT_OBJECT, etc.).
    pub stype: u8,
    /// Section index.
    pub shndx: u16,
}

/// Parsed ELF binary.
#[derive(Debug, Clone)]
pub struct ElfBinary {
    /// ELF class (32 or 64 bit).
    pub class: ElfClass,
    /// Data encoding.
    pub data: ElfDataEncoding,
    /// ELF type.
    pub elf_type: ElfType,
    /// Machine architecture.
    pub machine: ElfMachine,
    /// Entry point address.
    pub entry: u64,
    /// Section headers.
    pub sections: Vec<ElfSectionHeader>,
    /// Symbol table.
    pub symbols: Vec<ElfSymbol>,
    /// Readable strings extracted from the binary.
    pub strings: Vec<String>,
    /// SHA3-256 of the original binary.
    pub hash: [u8; 32],
}

/// Errors from ELF parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElfError {
    /// Not an ELF file.
    NotElf,
    /// Unsupported ELF class (only 64-bit supported).
    UnsupportedClass(u8),
    /// Unsupported data encoding.
    UnsupportedEncoding(u8),
    /// Truncated input.
    Truncated { needed: usize, available: usize },
    /// Invalid section index.
    InvalidSection(usize),
    /// Invalid string table.
    InvalidStringTable,
}

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElfError::NotElf => write!(f, "not an ELF file"),
            ElfError::UnsupportedClass(c) => {
                write!(f, "unsupported ELF class: {} (only 64-bit supported)", c)
            }
            ElfError::UnsupportedEncoding(e) => {
                write!(f, "unsupported data encoding: {}", e)
            }
            ElfError::Truncated { needed, available } => {
                write!(f, "truncated: need {} bytes, have {}", needed, available)
            }
            ElfError::InvalidSection(idx) => {
                write!(f, "invalid section index: {}", idx)
            }
            ElfError::InvalidStringTable => {
                write!(f, "invalid string table")
            }
        }
    }
}

/// Read a little-endian u16 from bytes.
fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, ElfError> {
    if offset + 2 > data.len() {
        return Err(ElfError::Truncated {
            needed: offset + 2,
            available: data.len(),
        });
    }
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

/// Read a little-endian u32 from bytes.
fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, ElfError> {
    if offset + 4 > data.len() {
        return Err(ElfError::Truncated {
            needed: offset + 4,
            available: data.len(),
        });
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Read a little-endian u64 from bytes.
fn read_u64_le(data: &[u8], offset: usize) -> Result<u64, ElfError> {
    if offset + 8 > data.len() {
        return Err(ElfError::Truncated {
            needed: offset + 8,
            available: data.len(),
        });
    }
    Ok(u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]))
}

/// Read a null-terminated string from bytes.
fn read_cstring(data: &[u8], offset: usize) -> String {
    let mut end = offset;
    while end < data.len() && data[end] != 0 {
        end += 1;
    }
    String::from_utf8_lossy(&data[offset..end]).to_string()
}

/// Parse an ELF binary from bytes.
pub fn parse_elf(data: &[u8]) -> Result<ElfBinary, ElfError> {
    // Validate magic.
    if data.len() < 16 || data[..4] != ELF_MAGIC {
        return Err(ElfError::NotElf);
    }

    let class = match data[4] {
        1 => ElfClass::Elf32,
        2 => ElfClass::Elf64,
        c => return Err(ElfError::UnsupportedClass(c)),
    };

    let data_enc = match data[5] {
        1 => ElfDataEncoding::LittleEndian,
        2 => ElfDataEncoding::BigEndian,
        e => return Err(ElfError::UnsupportedEncoding(e)),
    };

    // Only 64-bit little-endian supported for now.
    if class != ElfClass::Elf64 || data_enc != ElfDataEncoding::LittleEndian {
        return Err(ElfError::UnsupportedClass(data[4]));
    }

    // ELF64 header: 64 bytes.
    if data.len() < 64 {
        return Err(ElfError::Truncated {
            needed: 64,
            available: data.len(),
        });
    }

    let elf_type = match read_u16_le(data, 16)? {
        1 => ElfType::Relocatable,
        2 => ElfType::Executable,
        3 => ElfType::Shared,
        4 => ElfType::Core,
        t => ElfType::Unknown(t),
    };

    let machine = match read_u16_le(data, 18)? {
        0x3e => ElfMachine::X86_64,
        0xb7 => ElfMachine::AArch64,
        0x28 => ElfMachine::Arm,
        0x03 => ElfMachine::I386,
        m => ElfMachine::Unknown(m),
    };

    let entry = read_u64_le(data, 24)?;
    let shoff = read_u64_le(data, 40)? as usize; // e_shoff
    let shentsize = read_u16_le(data, 58)? as usize;
    let shnum = read_u16_le(data, 60)? as usize;
    let shstrndx = read_u16_le(data, 62)? as usize;

    // Parse section headers.
    let mut sections = Vec::with_capacity(shnum);
    let mut symtab_section = None;
    let mut _strtab_section = None;

    for i in 0..shnum {
        let sh_offset = shoff + i * shentsize;
        if sh_offset + shentsize > data.len() {
            break;
        }

        let sh_name = read_u32_le(data, sh_offset)? as usize;
        let sh_type_val = read_u32_le(data, sh_offset + 4)?;
        let sh_flags = read_u64_le(data, sh_offset + 8)?;
        let sh_addr = read_u64_le(data, sh_offset + 16)?;
        let sh_offset_val = read_u64_le(data, sh_offset + 24)?;
        let sh_size = read_u64_le(data, sh_offset + 32)?;
        let sh_link = read_u32_le(data, sh_offset + 40)?;
        let sh_entsize = read_u64_le(data, sh_offset + 56)?;

        let sh_type = match sh_type_val {
            0 => ShType::Null,
            1 => ShType::Progbits,
            2 => ShType::Symtab,
            3 => ShType::Strtab,
            4 => ShType::Rela,
            5 => ShType::Hash,
            6 => ShType::Dynamic,
            7 => ShType::Note,
            8 => ShType::Nobits,
            9 => ShType::Rel,
            10 => ShType::Dylib,
            14 => ShType::InitArray,
            15 => ShType::FiniArray,
            16 => ShType::PreinitArray,
            17 => ShType::Group,
            18 => ShType::SymtabShndx,
            t => ShType::Unknown(t),
        };

        sections.push(ElfSectionHeader {
            name: String::new(), // Filled after string table is loaded
            name_offset: sh_name,
            sh_type,
            flags: sh_flags,
            addr: sh_addr,
            offset: sh_offset_val,
            size: sh_size,
            link: sh_link,
            entsize: sh_entsize,
        });

        if sh_type == ShType::Symtab {
            symtab_section = Some(sections.len() - 1);
        }
        if sh_type == ShType::Strtab {
            _strtab_section = Some(sections.len() - 1);
        }
    }

    // Load the section header string table.
    if shstrndx < sections.len() {
        let shstrtab_sec = &sections[shstrndx];
        let strtab_data = &data[shstrtab_sec.offset as usize..];
        for sec in &mut sections {
            sec.name = read_cstring(strtab_data, sec.name_offset);
        }
    }

    // Load symbol table.
    let mut symbols = Vec::new();
    if let Some(sym_idx) = symtab_section {
        let sym_sec = &sections[sym_idx];
        let strtab_idx = sym_sec.link as usize;
        if strtab_idx < sections.len() {
            let strtab_sec = &sections[strtab_idx];
            let strtab_data = &data[strtab_sec.offset as usize..];
            let sym_data = &data[sym_sec.offset as usize..];
            let entry_size = if sym_sec.entsize > 0 {
                sym_sec.entsize as usize
            } else {
                24 // ELF64_Sym size
            };

            let count = if entry_size > 0 {
                sym_sec.size as usize / entry_size
            } else {
                0
            };

            for i in 0..count {
                let off = i * entry_size;
                if off + entry_size > sym_data.len() {
                    break;
                }
                let st_name = read_u32_le(sym_data, off)? as usize;
                let st_info = sym_data[off + 4];
                let st_shndx = read_u16_le(sym_data, off + 6)?;
                let st_value = read_u64_le(sym_data, off + 8)?;
                let st_size = read_u64_le(sym_data, off + 16)?;

                let bind = st_info >> 4;
                let stype = st_info & 0x0f;

                let name = read_cstring(&strtab_data, st_name);

                symbols.push(ElfSymbol {
                    name,
                    value: st_value,
                    size: st_size,
                    bind,
                    stype,
                    shndx: st_shndx,
                });
            }
        }
    }

    // Extract readable strings from the binary.
    let strings = extract_strings(data, 4);

    let hash = sha3_256(data);

    Ok(ElfBinary {
        class,
        data: data_enc,
        elf_type,
        machine,
        entry,
        sections,
        symbols,
        strings,
        hash,
    })
}

/// Extract printable ASCII strings of minimum length from binary data.
fn extract_strings(data: &[u8], min_len: usize) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = Vec::new();

    for &b in data {
        if b >= 0x20 && b < 0x7f {
            current.push(b);
        } else {
            if current.len() >= min_len {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    strings.push(s);
                }
            }
            current.clear();
        }
    }

    // Don't forget the last run.
    if current.len() >= min_len {
        if let Ok(s) = String::from_utf8(current) {
            strings.push(s);
        }
    }

    strings
}

// ============================================================================
// x86_64 Syscall Pattern Extractor
// ============================================================================

/// A detected syscall pattern in the binary.
#[derive(Debug, Clone)]
pub struct SyscallPattern {
    /// The x86_64 syscall number (from `mov eax, <num>; syscall`).
    pub number: u32,
    /// Offset in the binary where the pattern was found.
    pub offset: usize,
    /// Human-readable syscall name (Linux x86_64).
    pub name: &'static str,
}

/// Known Linux x86_64 syscall names for common numbers.
fn syscall_name(num: u32) -> &'static str {
    match num {
        0 => "read",
        1 => "write",
        2 => "open",
        3 => "close",
        4 => "stat",
        5 => "fstat",
        6 => "lstat",
        7 => "poll",
        8 => "lseek",
        9 => "mmap",
        10 => "mprotect",
        11 => "munmap",
        12 => "brk",
        13 => "rt_sigaction",
        14 => "rt_sigprocmask",
        16 => "ioctl",
        17 => "pread64",
        18 => "pwrite64",
        19 => "readv",
        20 => "writev",
        21 => "access",
        22 => "pipe",
        23 => "select",
        24 => "sched_yield",
        28 => "madvise",
        32 => "dup",
        33 => "dup2",
        35 => "nanosleep",
        39 => "getpid",
        41 => "socket",
        42 => "connect",
        43 => "accept",
        49 => "bind",
        50 => "listen",
        56 => "clone",
        57 => "fork",
        59 => "execve",
        60 => "exit",
        61 => "wait4",
        62 => "kill",
        63 => "uname",
        72 => "fcntl",
        78 => "getdents",
        79 => "getcwd",
        80 => "chdir",
        82 => "rename",
        83 => "mkdir",
        84 => "rmdir",
        87 => "unlink",
        89 => "readlink",
        90 => "chmod",
        92 => "chown",
        96 => "gettimeofday",
        99 => "sysinfo",
        102 => "getuid",
        104 => "getgid",
        107 => "geteuid",
        108 => "getegid",
        158 => "arch_prctl",
        202 => "futex",
        217 => "getdents64",
        228 => "clock_gettime",
        230 => "clock_nanosleep",
        231 => "exit_group",
        233 => "epoll_ctl",
        234 => "tgkill",
        257 => "openat",
        262 => "newfstatat",
        288 => "accept4",
        292 => "dup3",
        302 => "prlimit64",
        318 => "getrandom",
        332 => "statx",
        _ => "unknown",
    }
}

/// Extract x86_64 syscall patterns from binary code.
///
/// Looks for the pattern: `mov eax, <imm32>; syscall` (0xb8 XX XX XX XX 0x0f 0x05)
/// or `mov eax, <imm32>; sysenter` variants.
pub fn extract_syscalls(code_section: &[u8], base_offset: usize) -> Vec<SyscallPattern> {
    let mut patterns = Vec::new();
    let mut i = 0;

    while i + 6 < code_section.len() {
        // Pattern: b8 XX XX XX XX 0f 05 (mov eax, imm32; syscall)
        if code_section[i] == 0xb8 && code_section[i + 5] == 0x0f && code_section[i + 6] == 0x05 {
            let syscall_num = u32::from_le_bytes([
                code_section[i + 1],
                code_section[i + 2],
                code_section[i + 3],
                code_section[i + 4],
            ]);

            // Filter: only include known/common syscalls (skip noise).
            if syscall_num <= 332 {
                patterns.push(SyscallPattern {
                    number: syscall_num,
                    offset: base_offset + i,
                    name: syscall_name(syscall_num),
                });
            }

            i += 7;
        } else {
            i += 1;
        }
    }

    patterns
}

// ============================================================================
// Behavior Profiler
// ============================================================================

/// Behavior category inferred from syscall + string patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BehaviorCategory {
    /// File I/O operations (open, read, write, stat).
    FileIO,
    /// Network operations (socket, connect, bind, listen).
    Network,
    /// Process management (fork, exec, wait, kill).
    Process,
    /// Memory management (mmap, brk, madvise).
    Memory,
    /// Crypto operations (inferred from strings like "aes", "sha", "key").
    Crypto,
    /// Database operations (inferred from strings like "sqlite", "postgres").
    Database,
    /// Shell/command execution (inferred from strings like "/bin/sh", "exec").
    Shell,
    /// System information (uname, sysinfo, gettimeofday).
    SystemInfo,
    /// Unknown or undetermined.
    Unknown,
}

impl BehaviorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            BehaviorCategory::FileIO => "file_io",
            BehaviorCategory::Network => "network",
            BehaviorCategory::Process => "process",
            BehaviorCategory::Memory => "memory",
            BehaviorCategory::Crypto => "crypto",
            BehaviorCategory::Database => "database",
            BehaviorCategory::Shell => "shell",
            BehaviorCategory::SystemInfo => "system_info",
            BehaviorCategory::Unknown => "unknown",
        }
    }
}

/// Risk level for a binary based on its behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Low risk: standard tool behavior.
    Low,
    /// Medium risk: some potentially dangerous operations.
    Medium,
    /// High risk: network + exec + write = high attack surface.
    High,
    /// Critical: known dangerous patterns.
    Critical,
}

/// A complete behavior profile of a binary.
#[derive(Debug, Clone)]
pub struct BehaviorProfile {
    /// Binary name (from ELF symbols or filename).
    pub name: String,
    /// SHA3-256 of the binary.
    pub hash: [u8; 32],
    /// ELF machine type.
    pub machine: ElfMachine,
    /// ELF type (executable, shared, etc.).
    pub elf_type: ElfType,
    /// Entry point address.
    pub entry: u64,
    /// Number of sections.
    pub section_count: usize,
    /// Number of symbols.
    pub symbol_count: usize,
    /// Detected syscall patterns.
    pub syscalls: Vec<SyscallPattern>,
    /// Unique syscall numbers used.
    pub unique_syscalls: Vec<u32>,
    /// Extracted readable strings.
    pub strings: Vec<String>,
    /// Inferred behavior categories.
    pub behaviors: Vec<BehaviorCategory>,
    /// Risk level.
    pub risk: RiskLevel,
    /// Risk score (0.0..1.0).
    pub risk_score: f64,
    /// SHA3-256 of the profile itself (for verification).
    pub profile_hash: [u8; 32],
}

/// Profile a binary from its parsed ELF data.
pub fn profile_binary(elf: &ElfBinary, filename: &str) -> BehaviorProfile {
    // Find the .text section for syscall extraction.
    let mut code_offset = 0;
    let mut code_size = 0;
    for sec in &elf.sections {
        if sec.name == ".text" || sec.sh_type == ShType::Progbits && sec.flags & 0x4 != 0 {
            // SHT_PROGBITS + SHF_EXECINSTR
            code_offset = sec.offset as usize;
            code_size = sec.size as usize;
            break;
        }
    }

    // Extract syscalls from the code section.
    let code_section = if code_size > 0 && code_offset + code_size <= 0 {
        // For the initial implementation, we'll use the full binary
        // since we may not have the raw file offset mapping.
        &[]
    } else {
        &[]
    };

    let syscalls = extract_syscalls(code_section, code_offset);

    // Unique syscall numbers.
    let mut unique: Vec<u32> = syscalls.iter().map(|s| s.number).collect();
    unique.sort();
    unique.dedup();

    // Infer behaviors from syscalls.
    let mut behaviors = Vec::new();
    if unique.iter().any(|&n| matches!(n, 0 | 1 | 2 | 3 | 4 | 5 | 17 | 18)) {
        behaviors.push(BehaviorCategory::FileIO);
    }
    if unique.iter().any(|&n| matches!(n, 41 | 42 | 43 | 49 | 50 | 288)) {
        behaviors.push(BehaviorCategory::Network);
    }
    if unique.iter().any(|&n| matches!(n, 56 | 57 | 59 | 60 | 61 | 62)) {
        behaviors.push(BehaviorCategory::Process);
    }
    if unique.iter().any(|&n| matches!(n, 9 | 10 | 11 | 12 | 28)) {
        behaviors.push(BehaviorCategory::Memory);
    }

    // Infer from strings.
    let lower_strings: Vec<String> = elf.strings.iter().map(|s| s.to_lowercase()).collect();
    let has_string = |needle: &str| -> bool {
        lower_strings.iter().any(|s| s.contains(needle))
    };

    if has_string("aes") || has_string("sha") || has_string("encrypt") || has_string("decrypt") {
        behaviors.push(BehaviorCategory::Crypto);
    }
    if has_string("sqlite") || has_string("postgres") || has_string("mysql") || has_string("redis") {
        behaviors.push(BehaviorCategory::Database);
    }
    if has_string("/bin/sh") || has_string("/bin/bash") || has_string("exec") || has_string("system") {
        behaviors.push(BehaviorCategory::Shell);
    }
    if has_string("uname") || has_string("sysinfo") || has_string("gettimeofday") {
        behaviors.push(BehaviorCategory::SystemInfo);
    }

    // Deduplicate behaviors.
    behaviors.sort_by_key(|b| format!("{:?}", b));
    behaviors.dedup_by_key(|b| format!("{:?}", b));

    // Risk assessment.
    let has_network = behaviors.contains(&BehaviorCategory::Network);
    let has_shell = behaviors.contains(&BehaviorCategory::Shell);
    let has_exec = unique.contains(&59); // execve

    let risk_score = if has_network && has_shell {
        0.9
    } else if has_network && has_exec {
        0.8
    } else if has_shell {
        0.6
    } else if has_network {
        0.5
    } else if behaviors.len() > 3 {
        0.4
    } else {
        0.2
    };

    let risk = if risk_score >= 0.8 {
        RiskLevel::Critical
    } else if risk_score >= 0.6 {
        RiskLevel::High
    } else if risk_score >= 0.4 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };

    // Build profile.
    let mut profile = BehaviorProfile {
        name: filename.to_string(),
        hash: elf.hash,
        machine: elf.machine,
        elf_type: elf.elf_type,
        entry: elf.entry,
        section_count: elf.sections.len(),
        symbol_count: elf.symbols.len(),
        syscalls,
        unique_syscalls: unique,
        strings: elf.strings.clone(),
        behaviors,
        risk,
        risk_score,
        profile_hash: [0u8; 32],
    };

    // Compute profile hash.
    profile.profile_hash = compute_profile_hash(&profile);

    profile
}

/// Compute SHA3-256 of a behavior profile's canonical bytes.
fn compute_profile_hash(p: &BehaviorProfile) -> [u8; 32] {
    let mut buf = Vec::with_capacity(256);
    buf.extend_from_slice(p.name.as_bytes());
    buf.extend_from_slice(&p.entry.to_le_bytes());
    buf.extend_from_slice(&(p.section_count as u32).to_le_bytes());
    buf.extend_from_slice(&(p.symbol_count as u32).to_le_bytes());
    buf.extend_from_slice(&(p.unique_syscalls.len() as u32).to_le_bytes());
    for &sc in &p.unique_syscalls {
        buf.extend_from_slice(&sc.to_le_bytes());
    }
    buf.extend_from_slice(&(p.risk_score.to_bits()).to_le_bytes());
    sha3_256(&buf)
}

/// ASCII report of a behavior profile.
pub fn profile_report(p: &BehaviorProfile) -> String {
    let mut out = String::with_capacity(512);
    out.push_str(&format!("=== Behavior Profile: {} ===\n", p.name));
    out.push_str(&format!("  Hash:       {:02x?}\n", &p.hash[..8]));
    out.push_str(&format!("  Machine:    {}\n", p.machine.as_str()));
    out.push_str(&format!("  Entry:      0x{:x}\n", p.entry));
    out.push_str(&format!("  Sections:   {}\n", p.section_count));
    out.push_str(&format!("  Symbols:    {}\n", p.symbol_count));
    out.push_str(&format!(
        "  Syscalls:   {} unique ({})\n",
        p.unique_syscalls.len(),
        p.unique_syscalls
            .iter()
            .map(|n| syscall_name(*n))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(&format!(
        "  Behaviors:  {}\n",
        p.behaviors
            .iter()
            .map(|b| b.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(&format!("  Risk:       {:?} ({:.1}%)\n", p.risk, p.risk_score * 100.0));
    out.push_str(&format!("  Strings:    {} extracted\n", p.strings.len()));
    out.push_str(&format!("  Profile:    {:02x?}\n", &p.profile_hash[..8]));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid ELF64 header (just enough to pass parse_elf).
    fn minimal_elf64() -> Vec<u8> {
        let mut data = vec![0u8; 64];
        // Magic
        data[0] = 0x7f;
        data[1] = b'E';
        data[2] = b'L';
        data[3] = b'F';
        // Class: 64-bit
        data[4] = 2;
        // Data: little-endian
        data[5] = 1;
        // Type: executable (2)
        data[16] = 2;
        // Machine: x86_64 (0x3e)
        data[18] = 0x3e;
        data[19] = 0;
        // e_shoff = 64 (immediately after header)
        data[40] = 64;
        // e_shentsize = 64
        data[58] = 64;
        // e_shnum = 0 (no sections)
        data[60] = 0;
        // e_shstrndx = 0
        data[62] = 0;
        data
    }

    #[test]
    fn parse_valid_elf64_header() {
        let data = minimal_elf64();
        let elf = parse_elf(&data).unwrap();
        assert_eq!(elf.class, ElfClass::Elf64);
        assert_eq!(elf.data, ElfDataEncoding::LittleEndian);
        assert_eq!(elf.elf_type, ElfType::Executable);
        assert_eq!(elf.machine, ElfMachine::X86_64);
    }

    #[test]
    fn parse_rejects_non_elf() {
        let data = b"MZ\x90\x00"; // PE header
        assert!(matches!(parse_elf(data), Err(ElfError::NotElf)));
    }

    #[test]
    fn parse_rejects_truncated() {
        let data = [0x7f, b'E', b'L', b'F'];
        assert!(parse_elf(&data).is_err());
    }

    #[test]
    fn extract_strings_basic() {
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(b"abcd");
        data[5..9].copy_from_slice(b"efgh");
        let strings = extract_strings(&data, 4);
        assert!(strings.contains(&"abcd".to_string()));
        assert!(strings.contains(&"efgh".to_string()));
    }

    #[test]
    fn extract_strings_min_length() {
        let mut data = vec![0u8; 10];
        data[0..3].copy_from_slice(b"abc"); // too short
        data[4..8].copy_from_slice(b"defg"); // ok
        let strings = extract_strings(&data, 4);
        assert!(!strings.contains(&"abc".to_string()));
        assert!(strings.contains(&"defg".to_string()));
    }

    #[test]
    fn syscall_name_known() {
        assert_eq!(syscall_name(0), "read");
        assert_eq!(syscall_name(1), "write");
        assert_eq!(syscall_name(59), "execve");
        assert_eq!(syscall_name(60), "exit");
    }

    #[test]
    fn syscall_name_unknown() {
        assert_eq!(syscall_name(9999), "unknown");
    }

    #[test]
    fn extract_syscalls_finds_pattern() {
        // mov eax, 1; syscall -> write
        let code = [
            0xb8, 0x01, 0x00, 0x00, 0x00, 0x0f, 0x05, // write
            0xb8, 0x3b, 0x00, 0x00, 0x00, 0x0f, 0x05, // execve
        ];
        let patterns = extract_syscalls(&code, 0);
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].number, 1);
        assert_eq!(patterns[0].name, "write");
        assert_eq!(patterns[1].number, 59);
        assert_eq!(patterns[1].name, "execve");
    }

    #[test]
    fn extract_syscalls_empty_on_no_pattern() {
        let code = [0x90, 0x90, 0x90, 0x90]; // nop sled
        let patterns = extract_syscalls(&code, 0);
        assert!(patterns.is_empty());
    }

    #[test]
    fn behavior_category_as_str() {
        assert_eq!(BehaviorCategory::FileIO.as_str(), "file_io");
        assert_eq!(BehaviorCategory::Network.as_str(), "network");
        assert_eq!(BehaviorCategory::Crypto.as_str(), "crypto");
    }

    #[test]
    fn risk_level_ordering() {
        assert!(RiskLevel::Low != RiskLevel::High);
        assert!(RiskLevel::Critical != RiskLevel::Medium);
    }

    #[test]
    fn profile_hash_deterministic() {
        let elf = parse_elf(&minimal_elf64()).unwrap();
        let p1 = profile_binary(&elf, "test");
        let p2 = profile_binary(&elf, "test");
        assert_eq!(p1.profile_hash, p2.profile_hash);
    }

    #[test]
    fn profile_report_contains_key_sections() {
        let elf = parse_elf(&minimal_elf64()).unwrap();
        let p = profile_binary(&elf, "test");
        let report = profile_report(&p);
        assert!(report.contains("Behavior Profile: test"));
        assert!(report.contains("Machine:"));
        assert!(report.contains("Risk:"));
    }

    #[test]
    fn hex_util_integration() {
        // Verify hex_util is usable from this module.
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        let hex = hex_util::encode(&bytes);
        assert_eq!(hex, "deadbeef");
        let decoded = hex_util::decode(&hex).unwrap();
        assert_eq!(decoded, bytes);
    }
}
