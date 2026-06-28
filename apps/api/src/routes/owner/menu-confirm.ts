import { withTenant } from '@deliveryos/platform';

// P6 CLAIM PHASE (council K4 / CC3) — the owner's per-product allergen confirmation. A DISTINCT,
// deliberate, authenticated act: the AI write-stripped allergens (bom[].allergens = []), so the owner
// AUTHORS allergens into empty fields via the normal product editor, then confirms HERE. Confirmation
// flips `allergens_confirmed=true` ONLY (never mutates `source` — that preserves the 'place' provenance/
// liability audit, and the C2 read-gate keys on it). Until confirmed, the read-gate strips allergens.
export default async function menuConfirmRoutes(server: any) {
  // Confirm a single product's allergens (per-product; bulk is a client loop over this, each authenticated).
  server.post(
    '/api/owner/locations/:locationId/products/:productId/confirm-allergens',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner']), server.requireLocationAccess],
    },
    async (request: any, reply: any) => {
      const { locationId, productId } = request.params as { locationId: string; productId: string };
      const userId = (request.user as any).sub;
      const res = await withTenant(server.db, userId, async (client: any) =>
        client.query(
          `UPDATE products SET allergens_confirmed = true WHERE id = $1 AND location_id = $2 RETURNING id`,
          [productId, locationId],
        ),
      );
      if (res.rowCount === 0) return reply.code(404).send({ error: 'PRODUCT_NOT_FOUND' });
      return reply.code(200).send({ confirmed: true });
    },
  );
}
