# /lib/ocr: the invoice reader, vendored (W-OCR, 2026-10-04)

The owner console's "From a photo" sheet (`/admin/receipt-photo.js`) reads an invoice photo
in the owner's own browser. Everything it needs is served from this folder. Nothing is loaded
from a CDN at runtime, because the CSP (`public/_headers`) allows `'self'` only. The only CSP
change is `'wasm-unsafe-eval'` in `script-src`, which lets the page compile the WebAssembly
core. It does not allow JavaScript `eval`.

None of this loads until the owner taps "From a photo". `ingredients.js` imports the sheet
lazily, and the sheet imports `tesseract.esm.min.js` only on that first photo.

## Pinned files

| file | from (npm) | bytes | sha256 |
|---|---|---|---|
| tesseract.esm.min.js | tesseract.js 7.0.0 `dist/` | 63220 | 64871d76c75609fd5413b88a8171e2ef40deedd77d5875ba23df104b2d05eb29 |
| worker.min.js | tesseract.js 7.0.0 `dist/` | 111307 | 576b7df7e3393e137e51849357c9adb53fe7ac1bb69bfa06cf3d61520f182c6d |
| tesseract-core-simd-lstm.js | tesseract.js-core 7.0.0 | 89271 | e48e2f02ddae3716c8dd24bf41cd290d4efa96892d689cdc4013c2545d63f469 |
| tesseract-core-simd-lstm.wasm | tesseract.js-core 7.0.0 | 2857601 | 34e8d50cac216427d86bf397d610fdd9f49492539bbcdfbfccc4eda20c810bea |
| tesseract-core-lstm.js | tesseract.js-core 7.0.0 (no-SIMD fallback) | 89261 | 6510efc4e8b45c5465df30679b9911ffe0071cd2ee982fa064e6f5136ef2de85 |
| tesseract-core-lstm.wasm | tesseract.js-core 7.0.0 (no-SIMD fallback) | 2855361 | 66b17df6e20c5329a17ffa9c202a47eaa3e32500b253d4c7f38e7f2bc01457c3 |
| sqi.traineddata.gz | @tesseract.js-data/sqi `4.0.0_best_int` (tessdata_best, int) | 962962 | a84023c64e1c0910f8fddc25d13f2dbe1157ce2f478abb48adc8013de9b108f1 |
| eng.traineddata.gz | @tesseract.js-data/eng `4.0.0_best_int` | 2952873 | 45b4cb346724ac1774f1c36f42f182b887bcdb28ebe63e6fff90ac41f3fcff91 |
| LICENSE-tesseract.js.txt | tesseract.js, Apache-2.0 | 11357 | b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1 |
| LICENSE-tesseract.js-core.txt | tesseract.js-core, Apache-2.0 (Tesseract itself: Apache-2.0) | 11358 | c6596eb7be8581c18be736c846fb9173b69eccf6ef94c5135893ec56bd92ba08 |
| tesseract.esm.min.js.LICENSE.txt, worker.min.js.LICENSE.txt | the bundles' own notices | 149, 466 | cdf963ce..., 45f54171... |

The traineddata packages are MIT (@tesseract.js-data), and the models are Tesseract's tessdata_best
(Apache-2.0).

## Size

There are 9.6 MB on disk. A device downloads only one core, either SIMD or plain. On the
first photo it downloads about 6.9 MB raw: 63 KB + 111 KB + 89 KB of JS, the 2.86 MB wasm, and
3.9 MB of traineddata, which is already gzipped. Compressed on the wire (gzip -9 measured on the
box), that is about 5.0 MB, once. After that the browser serves it from cache.

## Verify

`sha256sum *` in this folder must print the table above. `tools/live-proof/fixtures/receipt/measure.mjs`
loads `sqi+eng` from THIS folder with `langPath` and reads the fixture invoices. A missing or corrupt
`sqi.traineddata.gz` fails there, not silently.

## Update

Bump all three npm packages together, copy the same file names, then re-run `sha256sum`,
`measure.mjs` and the headless check. Then update this table in the same change.
