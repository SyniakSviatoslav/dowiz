// lib/ui/dom-shim.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Document } from './dom-shim.mjs';

// A failed assertion prints the element it was handed. The back references
// (parent, document) must stay out of that print: followed, they turn the tree
// into a graph whose paths multiply with its size.
const tree = () => {
  const doc = new Document();
  const box = doc.createElement('div');
  doc.body.appendChild(box);
  box.innerHTML = '<div><p><b id="a">x</b><i>y</i></p><section><button id="target" class="btn">z</button><span>q</span></section><ul><li>1</li><li>2</li><li>3</li></ul></div>';
  return box;
};

test('a failed assertion on an element prints a small message', () => {
  let message = '';
  try { assert.equal(tree().querySelector('#target'), null, 'gone'); } catch (e) { message = String(e.message); }
  assert.ok(message.length > 0, 'the assertion failed');
  assert.ok(message.length < 8000, `message of ${message.length} chars`);
});

test('the back references still work with the print kept small', () => {
  const box = tree();
  const b = box.querySelector('#target');
  assert.equal(b.parentNode.tagName, 'SECTION');
  assert.equal(b.ownerDocument, box.ownerDocument);
  b.remove();
  assert.equal(b.parentNode, null);
  assert.equal(box.querySelector('#target'), null);
  box.appendChild(b);
  assert.equal(b.parentNode, box);
  assert.ok(box.contains(b));
});
