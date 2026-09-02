# Bebop on Linux & Android (W^X-clean loader)

## Status
- **Linux aarch64**: native — the whole self-hosted toolchain (seed loader +
  bebop.bin compiler) runs directly on the Linux syscall ABI.
- **Android**: Android IS aarch64 Linux — the same syscall numbers apply
  (arm64). The one blocker WAS the loader's anonymous RWX mmap (SELinux
  `execmem` denial); that is now fixed. Everything below is real, no
  emulation layer.

## The loader (seed/seed.S) — W^X-clean since 2026-09-02
The .bin (word stream + LE64 entry footer) is mmap'd as
**PROT_READ|PROT_EXEC** (`x2 = #5`), MAP_PRIVATE, FILE-BACKED. No page is
ever writable+executable simultaneously:
- Linux: fine (was already fine, now also hardening-clean).
- Android: file-backed RX is the standard JIT pattern — SELinux blocks
  anonymous RWX (`execmem`) but allows RX on an app-owned file. Put the
  .bin under the app's code-cache dir (`context.getCodeCacheDir()`).

## Embedding contract (in-process execution)
Android apps cannot exec arbitrary binaries from `/data` (noexec), and the
bebop model does not need exec: the loader is ~160 bytes of position-
independent logic. Embed it as an in-process function:

1. `mmap(0, size, PROT_READ|PROT_EXEC, MAP_PRIVATE, fd, 0)` on the .bin file;
2. read the LE64 entry offset from `base[size-8]`;
3. set up the arena: `mmap(256MB, PROT_READ|WRITE, MAP_PRIVATE|ANON)` →
   x27 = base, x28 = end;
4. M4 args contract: `x0 = argc`, `x1 = pointer array of arena-copied argv
   strings` (copy each NUL-terminated string into the arena, 8-align the
   cursor);
5. `blr (base + entry)`; the result is in x0 (print it or return it).

The `seed.S` file is the reference implementation of exactly these steps —
port it verbatim into an NDK `.S` file (it assembles with any aarch64
toolchain: clang/gas).

## Compiler on Android
`bebop.bin` (the self-hosted compiler) is itself a `.bin` — load it with
the same loader inside the app process and call it with
`argv = {prog, "compile", src_path, out_path}`. The compile path already
uses atomic publish (Ф6, `sys_export` + `renameat`) which is exactly what
an Android JIT wants: the artifact appears atomically in the code cache,
then gets mmap'd RX with the loader above.

## Syscall surface (everything the runtime uses)
openat(56), close(57), read(63), write(64), lseek(62), mmap(222),
munmap(215), ftruncate(46), renameat(38), exit(94) — all present on
Android's arm64 kernel ABI, no libc, no bionic dependency.

## Verified
- W^X loader: 42/42 std_golden gates, parity 9/0, construct 20/20 with
  PROT_READ|PROT_EXEC mapping (2026-09-02).
- `seed` built with `aarch64-linux-gnu-as` + `ld` (works with NDK clang the
  same way — plain armv8-a, no extensions).
