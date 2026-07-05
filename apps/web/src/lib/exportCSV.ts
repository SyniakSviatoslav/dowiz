function download(content: string, filename: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// Keys starting with `_` are internal (ids, raw rows) and never exported — keep
// this filter identical across formats so JSON can't leak fields CSV hides.
const cleanRow = (row: Record<string, any>) =>
  Object.fromEntries(Object.entries(row).filter(([k]) => !k.startsWith('_')));

export function exportCSV(data: Record<string, any>[], filename: string) {
  if (data.length === 0) return;
  const firstRow = data[0];
  if (!firstRow) return;
  const headers = Object.keys(firstRow).filter(k => !k.startsWith('_'));
  const csv = [headers.join(','), ...data.map(row => headers.map(h => {
    const v = row[h];
    if (v === null || v === undefined) return '';
    const s = String(v).replace(/"/g, '""');
    return /[,"\n]/.test(s) ? `"${s}"` : s;
  }).join(','))].join('\n');
  download('\uFEFF' + csv, filename, 'text/csv;charset=utf-8');
}

// Agent-friendly: a single JSON array, pretty-printed. LLM/agent toolchains
// parse this cleaner than CSV (typed values, nested structure, no quoting rules).
export function exportJSON(data: Record<string, any>[], filename: string) {
  if (data.length === 0) return;
  download(JSON.stringify(data.map(cleanRow), null, 2), filename, 'application/json;charset=utf-8');
}

// JSONL / NDJSON: one object per line. Ideal for streaming row-by-row into an
// agent context window or appending to a change log without re-parsing an array.
export function exportJSONL(data: Record<string, any>[], filename: string) {
  if (data.length === 0) return;
  download(data.map(r => JSON.stringify(cleanRow(r))).join('\n'), filename, 'application/x-ndjson;charset=utf-8');
}
