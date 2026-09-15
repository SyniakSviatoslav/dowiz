// Re-encode a photo in the browser before it is uploaded.
//
// Used by the admin pane's dish photos. It lives in lib/ rather than inside
// admin/app.js because "how large is a photo allowed to be" is a decision that
// should have one home: the moment a second surface needs it, the alternative
// is a copy, and the copy is the one that drifts and the one nobody tested.
//
// THREE THINGS IT DOES, all of which matter on a phone:
//  - caps the longest edge, so a 12-megapixel camera photo does not become a
//    six-megabyte upload over a cafe's connection;
//  - re-encodes as JPEG, which is what strips the EXIF -- and EXIF on a photo
//    taken in a restaurant carries that restaurant's GPS coordinates;
//  - resolves a Blob rather than a data URI, so the bytes are never doubled in
//    memory as base64.
export function shrinkImage(file, max = 1600, quality = 0.82){
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
