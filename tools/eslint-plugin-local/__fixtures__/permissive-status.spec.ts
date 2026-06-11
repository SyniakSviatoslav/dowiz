// Test fixture for no-permissive-status-assertion rule

function test(name: string, fn: () => void) { fn(); }
function expect(x: any) {
  return {
    toBe: (y: any) => {},
    toContain: (y: any) => {},
    not: { toBe: (y: any) => {} },
  };
}

// ANTI-PATTERN: should be flagged
test('bad — permissive status array', () => {
  const status = 200;
  expect([200, 201, 400, 500]).toContain(status);
});

// ANTI-PATTERN: two-element array
test('bad — permissive with 2 values', () => {
  expect([200, 400]).toContain(200);
});

// CORRECT: single value, no array
test('good — exact status', () => {
  expect(200).toBe(200);
});

// CORRECT: non-numeric array
test('good — string array', () => {
  expect(['a', 'b']).toContain('a');
});
