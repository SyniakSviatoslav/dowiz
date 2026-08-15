//! academia.rs — std host shim (pure 8D crystal-lattice core lives in
//! `dowiz_core::academia`; the `load_snapshot(path)` file-read seam stays here).
//!
//! The no_std core exposes `Academia::from_snapshot(&[u8])` (pure parse); this
//! shim adds the convenience `load_snapshot` that reads a matrix snapshot from
//! the VFS and delegates to it.

pub use dowiz_core::academia::*;

/// Завантажити матричний снепшот та верифікувати lattice (читає файл через VFS).
pub fn load_snapshot(path: &str) -> Result<Academia, String> {
    let data = crate::vfs::read(path).map_err(|e| format!("read: {}", e))?;
    let lib = Academia::from_snapshot(&data)?;
    Ok(lib)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_extracted_snapshot() {
        let path = "/tmp/academia_matrix.bin";
        if let Ok(lib) = load_snapshot(path) {
            assert!(lib.len() > 500000);
            let mut total = 0u64;
            for cell in &lib.lattice { total += cell.len() as u64; }
            assert_eq!(total as usize, lib.len());
        }
    }
}
