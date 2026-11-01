//! What a result line is worth depends on what produced it, so a `full` run opens with `# machine`
//! lines. Linux's /proc only, read at run time; a field this machine does not offer says `unknown`,
//! and a stranger's disk model is theirs to add: nothing here can see it through a mount. Inside
//! a container or proot the storage line is what that guest's /proc/mounts claims, not the disk.
use std::fs;
use std::path::Path;

/// A `key: value` file's first `key`, as /proc/cpuinfo and /proc/meminfo write them.
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == key).then(|| v.trim().to_string())
    })
}

/// Device and filesystem of the longest mount point above `dir`, from /proc/mounts text.
pub fn mount_of(mounts: &str, dir: &Path) -> Option<(String, String)> {
    mounts
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let at = f.get(1)?.replace("\\040", " ");
            let ty = f.get(2)?.to_string();
            dir.starts_with(&at).then(|| (at.len(), f[0].to_string(), ty))
        })
        .max_by_key(|m| m.0)
        .map(|(_, dev, fs)| (dev, fs))
}

/// `name=version` of the engines this binary was built with, from its own lock file.
pub fn pins(lock: &str, names: &[&str]) -> String {
    let lines: Vec<&str> = lock.lines().collect();
    let one = |n: &&str| {
        let at = lines.iter().position(|l| *l == format!("name = \"{n}\""))?;
        Some(format!("{n}={}", lines.get(at + 1)?.strip_prefix("version = ")?.trim_matches('"')))
    };
    names.iter().filter_map(|n| one(n)).collect::<Vec<_>>().join(" ")
}

fn features() -> String {
    let mut f = vec![];
    if cfg!(feature = "competitors") {
        f.push("competitors");
    }
    if cfg!(feature = "mldsa") {
        f.push("mldsa");
    }
    f.join(",")
}

pub fn print(dir: &Path) {
    let read = |p: &str| fs::read_to_string(p).unwrap_or_default();
    let cpu = read("/proc/cpuinfo");
    let model = field(&cpu, "model name").or_else(|| field(&cpu, "Hardware")).or_else(|| field(&cpu, "CPU part").map(|p| format!("CPU part {p}"))).unwrap_or_else(|| "unknown".into());
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    println!("# machine cpu={model} threads={threads} arch={}", std::env::consts::ARCH);
    println!("# machine ram={}", field(&read("/proc/meminfo"), "MemTotal").unwrap_or_else(|| "unknown".into()));
    let kernel = read("/proc/sys/kernel/osrelease");
    println!("# machine kernel={}", if kernel.trim().is_empty() { "unknown" } else { kernel.trim() });
    let _ = fs::create_dir_all(dir);
    let at = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let (dev, ty) = mount_of(&read("/proc/mounts"), &at).unwrap_or_else(|| ("unknown".into(), "unknown".into()));
    println!("# machine storage dir={} device={dev} fs={ty}", at.display());
    let rustc = std::process::Command::new("rustc").arg("-V").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    println!("# machine rustc={}", rustc.filter(|s| !s.is_empty()).unwrap_or_else(|| "unknown".into()));
    let commit = std::env::var("ES_BENCH_COMMIT").or_else(|_| std::env::var("GITHUB_SHA")).unwrap_or_else(|_| "unknown (set ES_BENCH_COMMIT)".into());
    println!("# machine commit={commit} features={}", features());
    println!("# versions {}", pins(include_str!("../Cargo.lock"), &["redb", "heed", "rusqlite", "libsqlite3-sys", "lmdb-master-sys"]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_longest_mount_above_the_dir_names_it() {
        let m = "/dev/a / ext4 rw 0 0\n/dev/b /data xfs rw 0 0\n/dev/c /data2 btrfs rw 0 0\n";
        assert_eq!(mount_of(m, Path::new("/data/x")), Some(("/dev/b".into(), "xfs".into())));
        assert_eq!(mount_of(m, Path::new("/home/x")), Some(("/dev/a".into(), "ext4".into())));
        assert_eq!(mount_of("", Path::new("/data")), None);
    }

    #[test]
    fn a_pin_comes_from_the_lock_and_an_absent_crate_prints_nothing() {
        let lock = "[[package]]\nname = \"redb\"\nversion = \"4.4.0\"\n";
        assert_eq!(pins(lock, &["redb", "heed"]), "redb=4.4.0");
    }
}
