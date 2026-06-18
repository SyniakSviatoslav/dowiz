#!/usr/bin/env python3
"""On-demand PaddleOCR (PP-OCRv5) text extraction for the menu-import OCR seam.

dev/ops integration point for Stage A: invoked as a SUBPROCESS by the Node
`AiOcrParser` (behind `MenuParserProvider`) only when the OCR engine is set to
'paddle'. NOT a daemon / always-on service (I4) — it runs per import, then exits.
Touches menu-image content only; no PII, no product runtime state (I1).

Usage:   paddle-ocr.py <image-path> [lang]
Output:  a single JSON line on stdout: {"text", "confidence", "engine", "version", "lines"}
         all PaddleOCR logging is forced to stderr so stdout stays clean JSON.
"""
import sys
import os
import json
import io
import contextlib

def main() -> int:
    if len(sys.argv) < 2:
        print(json.dumps({"error": "usage: paddle-ocr.py <image-path> [lang]"}))
        return 2
    img_path = sys.argv[1]
    # PP-OCRv5 multilingual. Default 'sq' (Albanian, the product locale; Latin
    # script — natively supported). Override via PADDLE_OCR_LANG (e.g. 'en').
    lang = sys.argv[2] if len(sys.argv) > 2 else os.environ.get("PADDLE_OCR_LANG", "sq")

    if not os.path.exists(img_path):
        print(json.dumps({"error": f"image not found: {img_path}"}))
        return 2

    # Keep stdout pristine: route paddle's chatty init/inference logs to stderr.
    with contextlib.redirect_stdout(sys.stderr):
        from paddleocr import PaddleOCR
        ocr = PaddleOCR(
            lang=lang,
            use_doc_orientation_classify=False,
            use_doc_unwarping=False,
            use_textline_orientation=False,
            # paddle 3.3.x CPU builds hit a PIR/oneDNN runtime error; disabling
            # MKL-DNN routes around it (correctness unaffected, slightly slower).
            enable_mkldnn=False,
        )
        result = ocr.predict(img_path)

    texts, scores = [], []
    # paddleocr 3.x: predict() -> list of result objects, each dict-like with
    # 'rec_texts' / 'rec_scores'. Be defensive across minor API shapes.
    for page in result or []:
        d = page if isinstance(page, dict) else getattr(page, "json", None) or {}
        if isinstance(d, dict) and "res" in d:
            d = d["res"]
        rt = (d or {}).get("rec_texts") or []
        rs = (d or {}).get("rec_scores") or []
        for i, t in enumerate(rt):
            if t and str(t).strip():
                texts.append(str(t))
                scores.append(float(rs[i]) if i < len(rs) else 1.0)

    confidence = round(sum(scores) / len(scores), 4) if scores else 0.0
    import paddleocr as _p
    print(json.dumps({
        "text": "\n".join(texts),
        "confidence": confidence,
        "lines": len(texts),
        "engine": "paddleocr",
        "version": getattr(_p, "__version__", "unknown"),
    }))
    return 0

if __name__ == "__main__":
    sys.exit(main())
