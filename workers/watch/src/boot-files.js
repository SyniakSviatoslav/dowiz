// The storefront's six largest boot files, as tools/evals/collect/live.mjs::bootPaths names them.
// GENERATED, do not hand-edit: `node workers/watch/scripts/boot-files.mjs > workers/watch/src/boot-files.js`.
// test/evals.test.mjs fails when this list and bootPaths(repo) disagree, and the GitHub nightly
// fails (watch.boot_files_match) when the DEPLOYED list does.
export const BOOT_FILES = ["/store/store.css","/lib/icons.css","/store/i18n.js","/lib/ui/ui.css","/store/booking.js","/store/state.js"];
