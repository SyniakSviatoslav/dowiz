// Runs INSIDE the staging container. Reads /tmp/packets.json (Maps-enrichment packets w/ base64 webp), pushes
// hero + logo to R2, and writes the venue-identity fields to locations + location_themes. Secrets stay in-box.
const fs = require('fs');
const { Client } = require('pg');
const { S3Client, PutObjectCommand } = require('@aws-sdk/client-s3');

(async () => {
  const packets = JSON.parse(fs.readFileSync('/tmp/packets.json', 'utf8'));
  const s3 = new S3Client({
    region: 'auto', endpoint: process.env.R2_ENDPOINT,
    credentials: { accessKeyId: process.env.R2_ACCESS_KEY_ID, secretAccessKey: process.env.R2_SECRET_ACCESS_KEY },
  });
  const bucket = process.env.R2_BUCKET;
  const pubBase = process.env.R2_PUBLIC_URL ? process.env.R2_PUBLIC_URL.replace(/\/+$/, '') : null;
  const pg = new Client({ connectionString: process.env.DATABASE_URL_MIGRATIONS });
  await pg.connect();

  for (const p of packets) {
    if (!p.location_id) { console.log('SKIP', p.slug, '(no location_id):', (p.notes || []).join('; ')); continue; }
    let heroOk = false, logoUrl = null;
    if (p.heroWebpB64) {
      await s3.send(new PutObjectCommand({ Bucket: bucket, Key: `${p.location_id}/hero/cover.webp`, Body: Buffer.from(p.heroWebpB64, 'base64'), ContentType: 'image/webp' }));
      heroOk = true;
    }
    if (p.logoWebpB64) {
      const key = `locations/${p.location_id}/logo.webp`;
      await s3.send(new PutObjectCommand({ Bucket: bucket, Key: key, Body: Buffer.from(p.logoWebpB64, 'base64'), ContentType: 'image/webp' }));
      logoUrl = pubBase ? `${pubBase}/${key}` : `/media/${key}`;
    }
    await pg.query(
      `UPDATE locations SET
         address = COALESCE($2, address),
         lat = COALESCE($3, lat), lng = COALESCE($4, lng),
         hours_json = $5::jsonb,
         phone = COALESCE($6, phone), public_phone = COALESCE($6, public_phone)
       WHERE id = $1`,
      [p.location_id, p.address, p.lat, p.lng, JSON.stringify(p.hoursJson), p.phone]);
    await pg.query(
      `UPDATE location_themes SET
         primary_color = COALESCE($2, primary_color),
         google_rating = $3, google_review_count = $4, google_maps_url = $5,
         logo_url = COALESCE($6, logo_url)
       WHERE location_id = $1`,
      [p.location_id, p.primaryColor, p.googleRating, p.googleReviewCount, p.googleMapsUrl, logoUrl]);
    console.log('WIRED', p.slug, '| hero=' + heroOk, 'logo=' + !!logoUrl, 'rating=' + (p.googleRating ?? '-'), 'color=' + p.primaryColor, 'addr=' + (p.address ? 'y' : 'n'), 'hours=' + (p.hoursJson ? 'y' : 'n'));
  }
  await pg.end();
})().catch((e) => { console.error('WIRE_ERR', e.message); process.exit(1); });
