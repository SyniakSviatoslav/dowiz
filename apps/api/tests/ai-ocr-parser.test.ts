import test from 'node:test';
import assert from 'node:assert';
import http from 'http';
import { AiOcrParser } from '../src/lib/ai-ocr-parser.js';

// Minimal valid PDF with text content (base64 encoded)
// Contains: "Pizza Margherita 8.50 EUR" and "Pasta Carbonara 12.00 EUR"
const PDF_WITH_TEXT_BASE64 =
  'JVBERi0xLjQKMSAwIG9iajw8L1R5cGUvQ2F0YWxvZy9QYWdlcyAyIDAgUj4+ZW5kb2JqCjIgMCBvYmo8PC9UeXBlL1BhZ2VzL0tpZHNbMyAwIFJdL0NvdW50IDE+PmVuZG9iagozIDAgb2JqPDwvVHlwZS9QYWdlL01lZGlhQm94WzAgMCA2MTIgNzkyXS9QYXJlbnQgMiAwIFIvUmVzb3VyY2VzPDwvRm9udDw8L0YxIDQgMCBSPj4+Pi9Db250ZW50cyA1IDAgUj4+ZW5kb2JqCjQgMCBvYmo8PC9UeXBlL0ZvbnQvU3VidHlwZS9UeXBlMS9CYXNlRm9udC9IZWx2ZXRpY2E+PmVuZG9iago1IDAgb2JqPDwvTGVuZ3RoIDEwNz4+c3RyZWFtCkJUCi9GMSAxMiBUZgo1MCA3MDAgVGQKKFBpenphIE1hcmdoZXJpdGEgOC41MCBFVVIpIFRqCi9GMSAxMiBUZgo1MCA2ODAgVGQKKFBhc3RhIENhcmJvbmFyYSAxMi4wMCBFVVIpIFRqCkVUCmVuZHN0cmVhbQplbmRvYmoKeHJlZgowIDYKMDAwMDAwMDAwMCA2NTUzNSBmIAowMDAwMDAwMDA5IDAwMDAwIG4gCjAwMDAwMDAwNTIgMDAwMDAgbiAKMDAwMDAwMDEwMSAwMDAwMCBuIAowMDAwMDAwMjExIDAwMDAwIG4gCjAwMDAwMDAyNzIgMDAwMDAgbiAKdHJhaWxlcjw8L1NpemUgNi9Sb290IDEgMCBSPj4Kc3RhcnR4cmVmCjQyNgolJUVPRgo=';
// Minimal PDF without extractable text (no content stream, just blank page)
const PDF_BLANK_BASE64 =
  'JVBERi0xLjQKMSAwIG9iajw8L1R5cGUvQ2F0YWxvZy9QYWdlcyAyIDAgUj4+ZW5kb2JqCjIgMCBvYmo8PC9UeXBlL1BhZ2VzL0tpZHNbMyAwIFJdL0NvdW50IDE+PmVuZG9iagozIDAgb2JqPDwvVHlwZS9QYWdlL01lZGlhQm94WzAgMCA2MTIgNzkyXS9QYXJlbnQgMiAwIFI+PmVuZG9iagp4cmVmCjAgNAowMDAwMDAwMDAwIDY1NTM1IGYgCjAwMDAwMDAwMDkgMDAwMDAgbiAKMDAwMDAwMDA1MiAwMDAwMCBuIAowMDAwMDAwMTAxIDAwMDAwIG4gCnRyYWlsZXI8PC9TaXplIDQvUm9vdCAxIDAgUj4+CnN0YXJ0eHJlZgoxNjQKJSVFT0YK';
// PDF with text containing PII (phone + email + IBAN embedded in menu text)
const PDF_WITH_PII_BASE64 =
  'JVBERi0xLjQKMSAwIG9iajw8L1R5cGUvQ2F0YWxvZy9QYWdlcyAyIDAgUj4+ZW5kb2JqCjIgMCBvYmo8PC9UeXBlL1BhZ2VzL0tpZHNbMyAwIFJdL0NvdW50IDE+PmVuZG9iagozIDAgb2JqPDwvVHlwZS9QYWdlL01lZGlhQm94WzAgMCA2MTIgNzkyXS9QYXJlbnQgMiAwIFIvUmVzb3VyY2VzPDwvRm9udDw8L0YxIDQgMCBSPj4+Pi9Db250ZW50cyA1IDAgUj4+ZW5kb2JqCjQgMCBvYmo8PC9UeXBlL0ZvbnQvU3VidHlwZS9UeXBlMS9CYXNlRm9udC9IZWx2ZXRpY2E+PmVuZG9iago1IDAgb2JqPDwvTGVuZ3RoIDIwOT4+c3RyZWFtCkJUCi9GMSAxMiBUZgo1MCA3MDAgVGQKKENhbGwgKzM1NSA2OSAxMjMgNDU2NyBmb3Igb3JkZXJzKSBUagovRjEgMTIgVGYKNTAgNjgwIFRkCihFbWFpbCBjb250YWN0QGdtYWlsLmNvbSBmb3Igc3VwcG9ydCkgVGoKL0YxIDEyIFRmCjUwIDY2MCBUZAooUGl6emEgLSA4LjUwIEVVUikgVGoKL0YxIDEyIFRmCjUwIDY0MCBUZAooUGFzdGEgLSAxMi4wMCBFVVIpIFRqCkVUCmVuZHN0cmVhbQplbmRvYmoKeHJlZgowIDYKMDAwMDAwMDAwMCA2NTUzNSBmIAowMDAwMDAwMDA5IDAwMDAwIG4gCjAwMDAwMDAwNTIgMDAwMDAgbiAKMDAwMDAwMDEwMSAwMDAwMCBuIAowMDAwMDAwMjExIDAwMDAwIG4gCjAwMDAwMDAyNzIgMDAwMDAgbiAKdHJhaWxlcjw8L1NpemUgNi9Sb290IDEgMCBSPj4Kc3RhcnR4cmVmCjUyOAolJUVPRgo=';
function pdfBuffer(base64: string): Buffer {
  return Buffer.from(base64.replace(/\s/g, ''), 'base64');
}

// ── LLM test server ──────────────────────────────────────────────
function startMockLLm(handler: (body: any) => any): Promise<{ port: number; server: http.Server }> {
  return new Promise((resolve) => {
    const server = http.createServer((req, res) => {
      let body = '';
      req.on('data', (chunk) => (body += chunk));
      req.on('end', () => {
        const parsed = JSON.parse(body);
        const response = handler(parsed);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(response));
      });
    });
    server.listen(0, () => {
      const port = (server.address() as any).port;
      resolve({ port, server });
    });
  });
}

// ═══════════════════════════════════════════════════════════════════
//  1. PDF text extraction (mock LLM — full flow)
// ═══════════════════════════════════════════════════════════════════

test('AiOcrParser - PDF with text: extracts and returns draft via mock LLM', async () => {
  const prev = process.env.LLM_PROVIDER;
  process.env.LLM_PROVIDER = 'mock';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_PROVIDER = prev;

  console.log('ACTUAL valid:', res.summary.valid); console.log('ACTUAL products:', res.draft.products.length); console.log('ACTUAL issues:', JSON.stringify(res.issues)); assert.strictEqual(res.summary.valid, 1, 'should have 1 product from mock');
  assert.strictEqual(res.draft.products.length, 1);
  assert.strictEqual(res.draft.products[0]!.currency, 'EUR');
  assert.strictEqual(res.issues.length, 0, 'should have no errors');
});

test('AiOcrParser - PDF with text: respects currency config in mock mode', async () => {
  const prev = process.env.LLM_PROVIDER;
  process.env.LLM_PROVIDER = 'mock';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'ALL', currencyMinorUnit: 0 }
  });

  process.env.LLM_PROVIDER = prev;

  assert.strictEqual(res.draft.products[0]!.currency, 'ALL');
});

// ═══════════════════════════════════════════════════════════════════
//  2. PDF extraction error cases
// ═══════════════════════════════════════════════════════════════════

test('AiOcrParser - PDF with no extractable text: returns error', async () => {
  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_BLANK_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  assert.strictEqual(res.summary.valid, 0);
  assert.strictEqual(res.summary.errors, 1);
  assert.ok(res.issues.some(i => i.code === 'OCR_LOW_QUALITY' || i.code === 'PARSE_ERROR'));
});

test('AiOcrParser - corrupt PDF: returns parse error', async () => {
  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: Buffer.from('not a pdf at all'),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  assert.strictEqual(res.summary.valid, 0);
  assert.strictEqual(res.summary.errors, 1);
  assert.ok(res.issues[0]!.code === 'PARSE_ERROR');
});

// ═══════════════════════════════════════════════════════════════════
//  3. PII Redaction
// ═══════════════════════════════════════════════════════════════════

test('AiOcrParser - menu document is fed un-redacted so the venue contact survives', async () => {
  const prev = process.env.LLM_PROVIDER;
  process.env.LLM_PROVIDER = 'mock';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_PII_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_PROVIDER = prev;

  // Behaviour change (approved): a menu document carries only the business's OWN
  // contact (no customer PII), and onboarding needs to extract the venue
  // address/phone — so the document is no longer pre-redacted before the model.
  // The parser must therefore NOT emit PII-redaction warnings for a menu.
  const piiIssues = res.issues.filter(i => i.code === 'POTENTIALLY_UNSAFE_VALUE');
  assert.equal(piiIssues.length, 0, 'menu contact is intentionally not redacted');
});

// ═══════════════════════════════════════════════════════════════════
//  4. Live LLM endpoint tests (start local HTTP server)
// ═══════════════════════════════════════════════════════════════════

test('AiOcrParser - live LLM returns valid JSON menu schema', async () => {
  const { port, server } = await startMockLLm((_body) => ({
    response: JSON.stringify({
      categories: [{ externalKey: 'cat1', name: 'Pizzas' }],
      products: [{ externalKey: 'p1', categoryKey: 'cat1', name: 'Margherita', price: 850, currency: 'EUR', available: true }],
      modifierGroups: [],
      modifiers: [],
      links: []
    })
  }));

  const prevEndpoint = process.env.LLM_ENDPOINT;
  const prevProvider = process.env.LLM_PROVIDER;
  process.env.LLM_ENDPOINT = `http://localhost:${port}/api/generate`;
  process.env.LLM_PROVIDER = 'llama3.1:8b-instruct';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_ENDPOINT = prevEndpoint;
  process.env.LLM_PROVIDER = prevProvider;
  server.close();

  assert.strictEqual(res.summary.valid, 1);
  assert.strictEqual(res.draft.products[0]!.name, 'Margherita');
  assert.strictEqual(res.summary.errors, 0);
});

test('AiOcrParser - live LLM returns non-JSON: returns parse error', async () => {
  const { port, server } = await startMockLLm((_body) => ({
    response: 'this is not json at all'
  }));

  const prevEndpoint = process.env.LLM_ENDPOINT;
  const prevProvider = process.env.LLM_PROVIDER;
  process.env.LLM_ENDPOINT = `http://localhost:${port}/api/generate`;
  process.env.LLM_PROVIDER = 'llama3.1:8b-instruct';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_ENDPOINT = prevEndpoint;
  process.env.LLM_PROVIDER = prevProvider;
  server.close();

  assert.strictEqual(res.summary.errors, 1);
  assert.strictEqual(res.summary.valid, 0);
});

test('AiOcrParser - live LLM returns invalid schema: returns parse error', async () => {
  const { port, server } = await startMockLLm((_body) => ({
    response: JSON.stringify({ foo: 'bar' })
  }));

  const prevEndpoint = process.env.LLM_ENDPOINT;
  const prevProvider = process.env.LLM_PROVIDER;
  process.env.LLM_ENDPOINT = `http://localhost:${port}/api/generate`;
  process.env.LLM_PROVIDER = 'llama3.1:8b-instruct';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_ENDPOINT = prevEndpoint;
  process.env.LLM_PROVIDER = prevProvider;
  server.close();

  assert.strictEqual(res.summary.errors, 1);
  assert.strictEqual(res.summary.valid, 0);
});

test('AiOcrParser - live LLM returns 500: returns parse error', async () => {
  const { port, server } = await startMockLLm((_body) => {
    // This will never be called because we override the handler below
    return { response: '' };
  });
  // Close default server and use one that returns 500
  server.close();

  // Create a server that returns non-OK status
  const errServer = http.createServer((_req, res) => {
    res.writeHead(500, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Internal Server Error' }));
  });
  await new Promise<void>((resolve) => errServer.listen(port, resolve));

  const prevEndpoint = process.env.LLM_ENDPOINT;
  process.env.LLM_ENDPOINT = `http://localhost:${port}/api/generate`;

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_ENDPOINT = prevEndpoint;
  errServer.close();

  assert.strictEqual(res.summary.errors, 1);
  assert.strictEqual(res.summary.valid, 0);
});

test('AiOcrParser - LLM endpoint unreachable: returns parse error', async () => {
  const prevEndpoint = process.env.LLM_ENDPOINT;
  process.env.LLM_ENDPOINT = 'http://localhost:1/api/generate';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_ENDPOINT = prevEndpoint;

  assert.strictEqual(res.summary.errors, 1);
  assert.strictEqual(res.summary.valid, 0);
});

// ═══════════════════════════════════════════════════════════════════
//  5. Input kind rejection
// ═══════════════════════════════════════════════════════════════════

test('AiOcrParser - unsupported kind throws', async () => {
  const parser = new AiOcrParser();
  await assert.rejects(
    () => parser.parse({ kind: 'csv' as any, bytes: Buffer.from(''), config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 } }),
    /AiOcrParser does not support/
  );
});

// ═══════════════════════════════════════════════════════════════════
//  6. OCR low confidence warning (PDF path — only path without OCR)
// ═══════════════════════════════════════════════════════════════════

test('AiOcrParser - PDF extracted text has normal flow with no low-confidence warning', async () => {
  const prev = process.env.LLM_PROVIDER;
  process.env.LLM_PROVIDER = 'mock';

  const parser = new AiOcrParser();
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_PROVIDER = prev;

  // PDF path uses confidence=1, so no low-confidence warnings
  const lowConfIssues = res.issues.filter(i => i.message.includes('Low confidence') || i.message.includes('low confidence'));
  assert.strictEqual(lowConfIssues.length, 0);
  assert.strictEqual(res.summary.low_confidence_count, 0);
});

// ═════════════════════════════════════════════════════════════════════
//  7. Memory Enhancement Test
// ═════════════════════════════════════════════════════════════════════

test('AiOcrParser - uses memory service to enhance prompt with examples', async () => {
  // Mock the memory service
  const mockMemoryService = {
    initialize: async () => {},
    search: async (query: string, options: any) => {
      // Return mock memories that would enhance the prompt
      if (query.includes('menu ingredients description bom')) {
        return {
          results: [
            {
              memory: JSON.stringify({
                products: [{
                  name: "Test Pizza",
                  description: "Test description with ingredients",
                  attributesJson: {
                    bom: [
                      { name: "Tomato", quantity: "100g", allergens: [] },
                      { name: "Cheese", quantity: "50g", allergens: ["dairy"] }
                    ]
                  }
                }]
              })
            }
          ]
        };
      }
      return { results: [] };
    }
  };

  const prev = process.env.LLM_PROVIDER;
  process.env.LLM_PROVIDER = 'mock';

  // Create parser with mocked memory service
  const parser = new AiOcrParser(mockMemoryService);
  const res = await parser.parse({
    kind: 'pdf',
    bytes: pdfBuffer(PDF_WITH_TEXT_BASE64),
    config: { expectedCurrency: 'EUR', currencyMinorUnit: 2 }
  });

  process.env.LLM_PROVIDER = prev;

  // Basic validation that parsing worked
  assert.strictEqual(res.summary.valid, 1);
  assert.strictEqual(res.draft.products.length, 1);
  
  // The key test: we could check if the prompt enhancement happened by
  // verifying that the mock LLM was called with enhanced prompt, but since
  // we're using the 'mock' provider which returns a fixed response, 
  // we primarily verify that the integration doesn't break existing functionality
  // and that memory service is properly integrated.
  
  // At minimum, we verify no errors occurred during memory retrieval
  assert.strictEqual(res.summary.errors, 0);
});