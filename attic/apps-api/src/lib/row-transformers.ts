export function toCategoryApiShape(row: Record<string, any>) {
  return {
    id: row.id,
    name: row.name,
    sortOrder: row.sort_order ?? row.sortOrder ?? 0,
    productCount: Number(row.product_count ?? row.productCount ?? 0),
  };
}
