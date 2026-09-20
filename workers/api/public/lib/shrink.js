// Re-encode a photo in the browser before it is uploaded.
//
// Used by the admin pane's dish photos and the venue logo. It lives in lib/
// rather than inside admin/app.js because "how large is a photo allowed to be"
// is a decision that should have one home: the moment a second surface needs
// it, the alternative is a copy, and the copy is the one that drifts and the
// one nobody tested.
//
// THREE THINGS IT DOES, all of which matter on a phone:
//  - caps the longest edge, so a 12-megapixel camera photo does not become a
//    six-megabyte upload over a cafe's connection;
//  - re-encodes as JPEG, which is what strips the EXIF -- and EXIF on a photo
//    taken in a restaurant carries that restaurant's GPS coordinates;
//  - resolves a Blob rather than a data URI, so the bytes are never doubled in
//    memory as base64.
//
// IT TAKES AN OPTIONS OBJECT, and for two days it did not while both callers
// passed one. `shrinkImage(file, { max: 1600, quality: 0.86 })` bound the
// OBJECT to the positional `max`, so the scale was `{...} / 4032` = NaN, the
// canvas refused the size, the promise rejected, and `.catch(() => f)` sent
// the untouched camera file. Every photograph and every logo this platform
// holds was uploaded at full resolution WITH its EXIF -- the measured "150 KB
// each, stored as uploaded" in the cost blueprint is this bug, and so is the
// GPS in the metadata of a photo taken in somebody's kitchen. Both shapes are
// accepted now, and a failure says so out loud instead of returning the
// original silently.
export function shrinkImage(file, opts = {}, maybeQuality){
  const { max, quality } = typeof opts === 'object' && opts !== null
    ? { max: opts.max ?? 1600, quality: opts.quality ?? 0.82 }
    : { max: opts ?? 1600, quality: maybeQuality ?? 0.82 };
  if (!Number.isFinite(max) || max <= 0) {
    return Promise.reject(new Error(`shrinkImage: max must be a number, got ${JSON.stringify(opts)}`));
  }
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      try {
        const scale = Math.min(1, max / Math.max(img.width, img.height));
        const w = Math.max(1, Math.round(img.width * scale));
        const h = Math.max(1, Math.round(img.height * scale));
        const c = document.createElement('canvas');
        c.width = w; c.height = h;
        const ctx = c.getContext('2d');
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(img, 0, 0, w, h);
        c.toBlob(b => b ? resolve(b) : reject(new Error('Не вдалося обробити зображення')),
                 'image/jpeg', quality);
      } catch (e) { reject(e); }
      finally { URL.revokeObjectURL(url); }
    };
    img.onerror = () => { URL.revokeObjectURL(url); reject(new Error('Це не зображення')); };
    img.src = url;
  });
}

/// The two sizes a menu needs: the sheet's photograph and the grid's card.
///
/// The grid is what a cold visit pays for -- eighteen photographs before the
/// first scroll -- and it draws them at about 400 CSS pixels wide, so a 480 px
/// JPEG is the honest size for it and a 1600 px one is twenty times the bytes
/// for no visible difference. `menu.js` already emits the `srcset`; this is the
/// file it needs.
///
/// A failure of the SMALL one is not a failure of the upload: the sheet's photo
/// is what the dish must have, and a card without a small variant simply falls
/// back to it, which is what the `srcset` already does.
export async function shrinkPair(file, { max = 1600, quality = 0.86, smallMax = 480, smallQuality = 0.72 } = {}){
  const full = await shrinkImage(file, { max, quality });
  let small = null;
  try {
    small = await shrinkImage(file, { max: smallMax, quality: smallQuality });
    // A "small" version that is not smaller is not worth a second blob or a
    // second request: an already-tiny photograph re-encodes to about itself.
    if (small.size >= full.size) small = null;
  } catch { small = null; }
  return { full, small };
}
