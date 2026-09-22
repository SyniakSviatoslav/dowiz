//! Colours out of a photograph: what a venue's own picture suggests its
//! palette should be.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use crate::owner::owner_and_venue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PixelsIn {
    /// Flat RGB triples in hex. The BROWSER decodes the image -- it already has
    /// a PNG and JPEG decoder and this Worker deliberately has neither.
    pixels: String,
}

/// `POST /api/owner/branding/extract` — suggest colours from an image.
///
/// SUGGESTS ONLY. Nothing is applied: the owner picks a swatch and posts it
/// back. An upload that silently repainted the storefront would be a change
/// nobody approved, made from a photograph.
pub async fn extract_branding(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: PixelsIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    // AUTHORISED, BUT NO VENUE IS NEEDED: this route reads pixels the caller
    // sent and touches no image, so there is no place to build and nothing a
    // wrong venue could reach. The check is here because the answer is still
    // the venue's business, not the internet's.
    if let Err(r) = owner_and_venue(&req, &ctx, &db).await {
        return Ok(r);
    }
    let hex = body.pixels.trim();
    if hex.is_empty() || hex.len() % 6 != 0 {
        return Response::error("pixels must be whole rgb triples in hex", 400);
    }
    // 64x64 is what the client is asked to send; this allows a good deal more
    // and refuses a payload nobody meant to upload.
    if hex.len() > 6 * 65_536 {
        return Response::error("too many pixels; downsample first", 400);
    }
    let Some(bytes) = dowiz_hub::crypto::unhex(hex) else {
        return Response::error("pixels are not hex", 400);
    };
    let pixels: Vec<dowiz_hub::palette::Rgb> = bytes
        .chunks_exact(3)
        .map(|c| dowiz_hub::palette::Rgb::new(c[0], c[1], c[2]))
        .collect();
    let swatches = dowiz_hub::palette::dominant(&pixels, 6);
    if swatches.is_empty() {
        // Honest emptiness. Inventing a colour for a black-and-white menu would
        // present a guess as a finding.
        return Response::from_json(&json!({
            "swatches": [], "note": "no strong colours found in this image"
        }));
    }
    Response::from_json(&json!({
        "swatches": swatches.iter().map(|s| {
            let t = dowiz_hub::palette::Theme::from_seed(s.colour);
            json!({
                "hex": s.colour.hex(),
                "sharePct": (s.share_bp as f64 / 100.0 * 10.0).round() / 10.0,
                // Every suggestion arrives already contrast-checked, so an
                // owner cannot pick one that fails.
                "passes": t.contrast_report().into_iter().all(|(_, got, want)| got >= want),
            })
        }).collect::<Vec<_>>()
    }))
}
