//! vfs.rs — virtual filesystem seam (ledger item 4: fs → VFS).
//!
//! The no_std audit found 36 lib modules whose only `std` dependency is the
//! filesystem (`std::fs`). A kernel module has no `std::fs` — it routes through
//! a ramfs / block-device / procfs VFS. This module is the single seam, in the
//! same shape as [`crate::clock`]: a no_std-compatible [`Vfs`] trait (paths as
//! `&str`, errors as [`VfsError`], no `std::path::Path` / `std::io::Error`), a
//! userspace impl [`StdFs`] that bridges to `std::fs`, and free functions
//! ([`read`], [`write`], …) that are the single authority call sites route
//! through. The kernel port swaps the *impl* (StdFs → kernel VFS), never the
//! call sites.
//!
//! # Accepted surface reduction
//! - Paths are `&str` (no `Path`/`PathBuf`): 3 call sites that passed a
//!   `PathBuf` (all `.join(...)` products) convert with `.to_str()`.
//! - `read_dir` returns an eager `Vec<DirEntry>` (not a lazy `ReadDir`): the
//!   kernel VFS has no lazy iterator; the 7 call sites iterate immediately.
//! - `metadata` is reduced to `{ len, is_dir }` (the only field read anywhere
//!   is `len`).

use alloc::string::String;
use alloc::vec::Vec;

/// no_std-compatible filesystem error (the `std::io::Error` replacement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    Io,
}

impl core::fmt::Display for VfsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            VfsError::NotFound => "not found",
            VfsError::PermissionDenied => "permission denied",
            VfsError::AlreadyExists => "already exists",
            VfsError::NotADirectory => "not a directory",
            VfsError::IsADirectory => "is a directory",
            VfsError::Io => "I/O error",
        };
        f.write_str(s)
    }
}

// Bridge so migrated call sites that still return `std::io::Result` keep
// compiling unchanged: `?` on a `Result<_, VfsError>` auto-converts via this
// `From` impl. (The kernel port never links `std::io`, so it uses `VfsError`
// directly; this impl exists only for the userspace migration.)
impl From<VfsError> for std::io::Error {
    fn from(e: VfsError) -> Self {
        use std::io::ErrorKind::*;
        let kind = match e {
            VfsError::NotFound => NotFound,
            VfsError::PermissionDenied => PermissionDenied,
            VfsError::AlreadyExists => AlreadyExists,
            VfsError::NotADirectory => NotADirectory,
            VfsError::IsADirectory => IsADirectory,
            VfsError::Io => Other,
        };
        std::io::Error::new(kind, e)
    }
}

// So `VfsError` can ride `Box<dyn std::error::Error>` (e.g. `io::Error::new`).
impl std::error::Error for VfsError {}

/// A directory entry (path + name + kind).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Full path (parent + separator + name).
    pub path: String,
    /// File name only (no parent component).
    pub name: String,
    /// Kind of entry.
    pub kind: FileKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Dir,
    Other,
}

impl DirEntry {
    pub fn is_dir(&self) -> bool {
        self.kind == FileKind::Dir
    }
    pub fn is_file(&self) -> bool {
        self.kind == FileKind::File
    }
    /// File extension (after the last `.`, ignoring a leading dot) — mirrors
    /// `Path::extension()` for the names the kernel consumes.
    pub fn extension(&self) -> Option<&str> {
        let name = &self.name;
        let dot = name.rfind('.')?;
        if dot == 0 {
            return None; // ".hidden" has no extension
        }
        Some(&name[dot + 1..])
    }
}

/// Reduced file metadata — the subset the kernel reads (only `len` today).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    pub len: u64,
    pub is_dir: bool,
}

impl Metadata {
    /// File size in bytes.
    pub fn len(&self) -> u64 {
        self.len
    }
    /// Whether the path is a directory.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
}

/// The virtual filesystem. no_std-compatible signature: `&str` paths, [`VfsError`].
pub trait Vfs {
    fn read(&self, path: &str) -> Result<Vec<u8>, VfsError>;
    fn read_to_string(&self, path: &str) -> Result<String, VfsError>;
    fn write(&self, path: &str, contents: &[u8]) -> Result<(), VfsError>;
    fn append(&self, path: &str, contents: &[u8]) -> Result<(), VfsError>;
    fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>, VfsError>;
    fn create_dir_all(&self, path: &str) -> Result<(), VfsError>;
    fn remove_file(&self, path: &str) -> Result<(), VfsError>;
    fn remove_dir_all(&self, path: &str) -> Result<(), VfsError>;
    fn rename(&self, from: &str, to: &str) -> Result<(), VfsError>;
    fn metadata(&self, path: &str) -> Result<Metadata, VfsError>;
}

/// Map a `std::io::Error` onto the no_std [`VfsError`].
fn map_io_err(e: &std::io::Error) -> VfsError {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => VfsError::NotFound,
        PermissionDenied => VfsError::PermissionDenied,
        AlreadyExists => VfsError::AlreadyExists,
        NotADirectory => VfsError::NotADirectory,
        IsADirectory => VfsError::IsADirectory,
        _ => VfsError::Io,
    }
}

/// The userspace VFS: `std::fs` behind the no_std seam.
pub struct StdFs;

impl Vfs for StdFs {
    fn read(&self, path: &str) -> Result<Vec<u8>, VfsError> {
        std::fs::read(path).map_err(|e| map_io_err(&e))
    }
    fn read_to_string(&self, path: &str) -> Result<String, VfsError> {
        std::fs::read_to_string(path).map_err(|e| map_io_err(&e))
    }
    fn write(&self, path: &str, contents: &[u8]) -> Result<(), VfsError> {
        std::fs::write(path, contents).map_err(|e| map_io_err(&e))
    }
    fn append(&self, path: &str, contents: &[u8]) -> Result<(), VfsError> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| map_io_err(&e))?;
        f.write_all(contents).map_err(|e| map_io_err(&e))
    }
    fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>, VfsError> {
        let rd = std::fs::read_dir(path).map_err(|e| map_io_err(&e))?;
        let mut out = Vec::new();
        for entry in rd {
            let entry = entry.map_err(|e| map_io_err(&e))?;
            let kind = entry
                .file_type()
                .map(|t| {
                    if t.is_dir() {
                        FileKind::Dir
                    } else if t.is_file() {
                        FileKind::File
                    } else {
                        FileKind::Other
                    }
                })
                .unwrap_or(FileKind::Other);
            out.push(DirEntry {
                path: entry.path().to_string_lossy().into_owned(),
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
            });
        }
        Ok(out)
    }
    fn create_dir_all(&self, path: &str) -> Result<(), VfsError> {
        std::fs::create_dir_all(path).map_err(|e| map_io_err(&e))
    }
    fn remove_file(&self, path: &str) -> Result<(), VfsError> {
        std::fs::remove_file(path).map_err(|e| map_io_err(&e))
    }
    fn remove_dir_all(&self, path: &str) -> Result<(), VfsError> {
        std::fs::remove_dir_all(path).map_err(|e| map_io_err(&e))
    }
    fn rename(&self, from: &str, to: &str) -> Result<(), VfsError> {
        std::fs::rename(from, to).map_err(|e| map_io_err(&e))
    }
    fn metadata(&self, path: &str) -> Result<Metadata, VfsError> {
        let m = std::fs::metadata(path).map_err(|e| map_io_err(&e))?;
        Ok(Metadata {
            len: m.len(),
            is_dir: m.is_dir(),
        })
    }
}

// ── Free functions — the single authority (call sites route through these) ──
//
// Paths are `impl AsRef<Path>` so `&str` / `String` / `PathBuf` / `&Path` all
// pass unchanged; the no_std [`Vfs`] trait stays `&str` (the kernel port swaps
// these wrappers' impls). Non-UTF-8 paths map to [`VfsError::Io`].

fn path_str(p: &std::path::Path) -> Result<&str, VfsError> {
    p.to_str().ok_or(VfsError::Io)
}

/// Read a whole file to bytes.
pub fn read<P: AsRef<std::path::Path>>(path: P) -> Result<Vec<u8>, VfsError> {
    StdFs.read(path_str(path.as_ref())?)
}

/// Read a whole file to a UTF-8 string.
pub fn read_to_string<P: AsRef<std::path::Path>>(path: P) -> Result<String, VfsError> {
    StdFs.read_to_string(path_str(path.as_ref())?)
}

/// Write bytes (or anything `AsRef<[u8]>`: `&str`, `String`, `Vec<u8>`, `&[u8]`).
pub fn write<P: AsRef<std::path::Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> Result<(), VfsError> {
    StdFs.write(path_str(path.as_ref())?, contents.as_ref())
}

/// Append bytes to a file (creating it if absent).
pub fn append<P: AsRef<std::path::Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> Result<(), VfsError> {
    StdFs.append(path_str(path.as_ref())?, contents.as_ref())
}

/// List a directory (eager; returns `(path, name, kind)` per entry).
pub fn read_dir<P: AsRef<std::path::Path>>(path: P) -> Result<Vec<DirEntry>, VfsError> {
    StdFs.read_dir(path_str(path.as_ref())?)
}

/// `mkdir -p`.
pub fn create_dir_all<P: AsRef<std::path::Path>>(path: P) -> Result<(), VfsError> {
    StdFs.create_dir_all(path_str(path.as_ref())?)
}

/// Remove a single file.
pub fn remove_file<P: AsRef<std::path::Path>>(path: P) -> Result<(), VfsError> {
    StdFs.remove_file(path_str(path.as_ref())?)
}

/// Remove a directory tree.
pub fn remove_dir_all<P: AsRef<std::path::Path>>(path: P) -> Result<(), VfsError> {
    StdFs.remove_dir_all(path_str(path.as_ref())?)
}

/// Rename / move.
pub fn rename<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    from: P,
    to: Q,
) -> Result<(), VfsError> {
    StdFs.rename(path_str(from.as_ref())?, path_str(to.as_ref())?)
}

/// File metadata (reduced: `{ len, is_dir }`).
pub fn metadata<P: AsRef<std::path::Path>>(path: P) -> Result<Metadata, VfsError> {
    StdFs.metadata(path_str(path.as_ref())?)
}

// ── Held file handle (open → write/read/sync → drop) ──
//
// The one-shot free functions above cover whole-file read/write/append. Three
// durability modules need a *held* handle: `fdr/ring` (append + `sync_data` on
// alarm/segment-switch), `brain/hydra` (lazy append handle + `sync_all` group
// commit), `backup` (open + `sync_all` before atomic rename). This seam is the
// held-handle analogue of the free functions: a no_std [`VfsFile`] trait (no
// `std::fs::File` / `std::io`), a userspace [`StdFile`] impl, and an
// [`open_file`] free function as the single authority. The kernel port maps
// `open_file` to a block-device / ramfs inode handle; call sites are unchanged.

/// Open mode for a held file handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    /// Read-only (existing file).
    Read,
    /// Write + create + truncate (fresh writer session).
    WriteTruncate,
    /// Append + create (append-only log).
    Append,
}

/// A held file handle. no_std-compatible signature (no `std::fs::File`).
pub trait VfsFile {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), VfsError>;
    fn flush(&mut self) -> Result<(), VfsError>;
    /// `fdatasync`-style: file data durable, metadata may lag.
    fn sync_data(&mut self) -> Result<(), VfsError>;
    /// `fsync`-style: data + metadata durable.
    fn sync_all(&mut self) -> Result<(), VfsError>;
}

/// The userspace held-handle impl (`std::fs::File`).
pub struct StdFile {
    inner: std::fs::File,
}

impl VfsFile for StdFile {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), VfsError> {
        use std::io::Write;
        self.inner.write_all(buf).map_err(|e| map_io_err(&e))
    }
    fn flush(&mut self) -> Result<(), VfsError> {
        use std::io::Write;
        self.inner.flush().map_err(|e| map_io_err(&e))
    }
    fn sync_data(&mut self) -> Result<(), VfsError> {
        self.inner.sync_data().map_err(|e| map_io_err(&e))
    }
    fn sync_all(&mut self) -> Result<(), VfsError> {
        self.inner.sync_all().map_err(|e| map_io_err(&e))
    }
}

/// Open a held file handle in the given mode.
pub fn open_file<P: AsRef<std::path::Path>>(path: P, mode: OpenMode) -> Result<StdFile, VfsError> {
    let mut o = std::fs::OpenOptions::new();
    match mode {
        OpenMode::Read => {
            o.read(true);
        }
        OpenMode::WriteTruncate => {
            o.write(true).create(true).truncate(true);
        }
        OpenMode::Append => {
            o.append(true).create(true);
        }
    }
    let f = o
        .open(path_str(path.as_ref())?)
        .map_err(|e| map_io_err(&e))?;
    Ok(StdFile { inner: f })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_roundtrip() {
        let path = std::env::temp_dir().join(format!("vfs_test_{}.txt", std::process::id()));
        let path = path.to_str().unwrap();
        write(path, b"hello vfs").expect("write");
        let got = read_to_string(path).expect("read back");
        assert_eq!(got, "hello vfs");
        remove_file(path).ok();
    }

    #[test]
    fn append_adds_to_file() {
        let path = std::env::temp_dir().join(format!("vfs_append_test_{}.txt", std::process::id()));
        let path = path.to_str().unwrap();
        write(path, "first\n").expect("write");
        append(path, "second\n").expect("append");
        assert_eq!(read_to_string(path).expect("read"), "first\nsecond\n");
        remove_file(path).ok();
    }

    #[test]
    fn missing_file_is_not_found() {
        assert_eq!(read("/definitely/not/here/vfs"), Err(VfsError::NotFound));
    }

    #[test]
    fn error_display_is_stable() {
        assert_eq!(VfsError::NotFound.to_string(), "not found");
        assert_eq!(VfsError::PermissionDenied.to_string(), "permission denied");
    }

    #[test]
    fn dir_entry_extension_ignores_leading_dot() {
        let e = DirEntry {
            path: "/a/.hidden".into(),
            name: ".hidden".into(),
            kind: FileKind::File,
        };
        assert_eq!(e.extension(), None);
        let e = DirEntry {
            path: "/a/doc.md".into(),
            name: "doc.md".into(),
            kind: FileKind::File,
        };
        assert_eq!(e.extension(), Some("md"));
    }
}
