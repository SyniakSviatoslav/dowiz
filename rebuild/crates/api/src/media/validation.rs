//! Pure, dependency-free validation helpers — ports `apps/api/src/lib/product-media-validation.ts`
//! verbatim: mime allow-list (SVG deliberately excluded — Q-NO-SVG, an active-content/XSS
//! invariant, CARRY), magic-byte sniff (the server-side defence against a claimed Content-Type
//! lying about the actual bytes), per-file size ceilings, per-location storage budget, and spin
//! frame-count bounds. Kept side-effect-free (no DB/HTTP) so the security-critical logic here is
//! unit-testable without a database or a running server — same rationale as the TS original.

/// Mime allow-list (`product-media-validation.ts:13-15`). SVG is never allowed — it has no magic
/// number (it's text/XML) and is an active-content vector; `sniff_mime` deliberately never
/// recognizes it, so it is rejected by omission at BOTH the declared-mime and sniffed-mime gates
/// (Q-NO-SVG — CARRY verbatim, a security invariant, not a gap).
pub const IMAGE_MIMES: [&str; 2] = ["image/webp", "image/jpeg"];
pub const VIDEO_MIMES: [&str; 1] = ["video/mp4"];

/// Per-file size ceilings (bytes) — `product-media-validation.ts:19-20`.
pub const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_VIDEO_BYTES: u64 = 25 * 1024 * 1024;

/// Per-location storage budget (`product-media-validation.ts:23`) — `SUM(existing) + incoming`
/// must stay under this. CARRY: this is a client-declared-bytes budget (breaker M2 — TOCTOU over
/// declared size; a real enforcement fix is a product decision, deferred, REV-S4-8 register).
pub const LOCATION_BUDGET_BYTES: u64 = 150 * 1024 * 1024;

/// Spin frame-count bounds, inclusive (`product-media-validation.ts:27-28`).
pub const SPIN_MIN_FRAMES: usize = 12;
pub const SPIN_MAX_FRAMES: usize = 72;

pub fn is_allowed_mime(mime: &str) -> bool {
    IMAGE_MIMES.contains(&mime) || VIDEO_MIMES.contains(&mime)
}

/// Poster images are raster-only — webp/jpeg, NEVER svg (`product-media-validation.ts:39-41`).
pub fn is_allowed_poster_mime(mime: &str) -> bool {
    IMAGE_MIMES.contains(&mime)
}

/// File extension for a content-addressed key, per allowed mime (`:44-53`).
pub fn ext_for_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/webp" => Some("webp"),
        "image/jpeg" => Some("jpg"),
        "video/mp4" => Some("mp4"),
        _ => None,
    }
}

pub fn max_bytes_for_mime(mime: &str) -> u64 {
    if VIDEO_MIMES.contains(&mime) {
        MAX_VIDEO_BYTES
    } else {
        MAX_IMAGE_BYTES
    }
}

/// Sniff the leading bytes of a buffer and return the detected container mime, or `None` if
/// unrecognised (`product-media-validation.ts::sniffMime`, verbatim). Server-side defence: a
/// client can claim any Content-Type, so this re-checks the actual bytes before persisting.
/// Recognises WebP (RIFF....WEBP), JPEG (FF D8 FF), MP4/ISO-BMFF (....ftyp). Deliberately does
/// NOT recognise SVG or executables (Q-NO-SVG).
pub fn sniff_mime(buf: &[u8]) -> Option<&'static str> {
    if buf.len() < 12 {
        return None;
    }
    if buf[0] == 0xff && buf[1] == 0xd8 && buf[2] == 0xff {
        return Some("image/jpeg");
    }
    if buf[0] == 0x52
        && buf[1] == 0x49
        && buf[2] == 0x46
        && buf[3] == 0x46
        && buf[8] == 0x57
        && buf[9] == 0x45
        && buf[10] == 0x42
        && buf[11] == 0x50
    {
        return Some("image/webp");
    }
    if buf[4] == 0x66 && buf[5] == 0x74 && buf[6] == 0x79 && buf[7] == 0x70 {
        return Some("video/mp4");
    }
    None
}

/// Bytes match the claimed mime iff the sniffed type is identical.
pub fn magic_bytes_match(buf: &[u8], claimed_mime: &str) -> bool {
    sniff_mime(buf) == Some(claimed_mime)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameCountResult {
    pub ok: bool,
    pub reason: Option<String>,
}

/// Spin frame-count must be within `[SPIN_MIN_FRAMES, SPIN_MAX_FRAMES]` (`:57-62`).
pub fn check_frame_count(count: usize) -> FrameCountResult {
    if count < SPIN_MIN_FRAMES {
        return FrameCountResult {
            ok: false,
            reason: Some(format!("spin needs at least {SPIN_MIN_FRAMES} frames")),
        };
    }
    if count > SPIN_MAX_FRAMES {
        return FrameCountResult {
            ok: false,
            reason: Some(format!("spin allows at most {SPIN_MAX_FRAMES} frames")),
        };
    }
    FrameCountResult {
        ok: true,
        reason: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetResult {
    pub ok: bool,
    pub used: u64,
    pub incoming: u64,
    pub limit: u64,
}

/// `SUM(existing) + incoming <= limit` (`product-media-validation.ts::checkBudget`).
pub fn check_budget(used: u64, incoming: u64, limit: u64) -> BudgetResult {
    let total = used.saturating_add(incoming);
    BudgetResult {
        ok: total <= limit,
        used,
        incoming,
        limit,
    }
}

pub fn sum_incoming_bytes(items: &[u64]) -> u64 {
    items.iter().fold(0u64, |acc, b| acc.saturating_add(*b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_list_never_admits_svg() {
        assert!(!is_allowed_mime("image/svg+xml"));
        assert!(!is_allowed_poster_mime("image/svg+xml"));
        assert!(ext_for_mime("image/svg+xml").is_none());
    }

    #[test]
    fn allow_list_admits_webp_jpeg_mp4() {
        assert!(is_allowed_mime("image/webp"));
        assert!(is_allowed_mime("image/jpeg"));
        assert!(is_allowed_mime("video/mp4"));
    }

    #[test]
    fn poster_mime_excludes_video() {
        assert!(is_allowed_poster_mime("image/webp"));
        assert!(!is_allowed_poster_mime("video/mp4"));
    }

    #[test]
    fn sniff_mime_detects_jpeg_webp_mp4() {
        assert_eq!(
            sniff_mime(&[0xff, 0xd8, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Some("image/jpeg")
        );
        let mut riff = vec![0x52, 0x49, 0x46, 0x46, 0, 0, 0, 0, 0x57, 0x45, 0x42, 0x50];
        riff.push(0);
        assert_eq!(sniff_mime(&riff), Some("image/webp"));
        let mut mp4 = vec![0, 0, 0, 0, 0x66, 0x74, 0x79, 0x70, 0, 0, 0, 0];
        mp4.push(0);
        assert_eq!(sniff_mime(&mp4), Some("video/mp4"));
    }

    #[test]
    fn sniff_mime_never_recognizes_svg_text() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        assert_eq!(sniff_mime(svg), None);
    }

    #[test]
    fn sniff_mime_rejects_short_buffers() {
        assert_eq!(sniff_mime(&[0xff, 0xd8, 0xff]), None);
    }

    #[test]
    fn magic_bytes_match_requires_exact_agreement() {
        let jpeg = [0xff, 0xd8, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(magic_bytes_match(&jpeg, "image/jpeg"));
        assert!(!magic_bytes_match(&jpeg, "image/webp"));
    }

    #[test]
    fn frame_count_bounds() {
        assert!(!check_frame_count(11).ok);
        assert!(check_frame_count(12).ok);
        assert!(check_frame_count(72).ok);
        assert!(!check_frame_count(73).ok);
    }

    #[test]
    fn budget_check_at_and_over_the_limit() {
        assert!(check_budget(100, 50, 150).ok, "exactly at the limit passes");
        assert!(!check_budget(100, 51, 150).ok, "one byte over fails");
    }

    #[test]
    fn budget_constants_match_the_ts_contract() {
        assert_eq!(LOCATION_BUDGET_BYTES, 150 * 1024 * 1024);
        assert_eq!(MAX_IMAGE_BYTES, 8 * 1024 * 1024);
        assert_eq!(MAX_VIDEO_BYTES, 25 * 1024 * 1024);
    }
}
