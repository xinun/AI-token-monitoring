import test from 'node:test';
import assert from 'node:assert/strict';
import { remaining, isFresh, resetLabel, escapeHtml } from '../src/presentation.ts';

test('expired windows become unknown, never automatically 100%', () => {
  assert.equal(remaining({ usedPercent: 72, resetsAt: 100 }, 99), 28);
  assert.equal(remaining({ usedPercent: 72, resetsAt: 100 }, 100), null);
  assert.match(resetLabel(100, 100), /재조회/);
});
test('invalid and missing provider data is not displayed as full quota', () => {
  for (const usedPercent of [-1, 101, NaN, Infinity]) assert.equal(remaining({ usedPercent, resetsAt: null }, 1), null);
  assert.equal(isFresh({ status: 'ready', fetchedAt: null }, 10), false);
  assert.equal(isFresh({ status: 'error', fetchedAt: 10 }, 10), false);
});
test('old or future-dated bridge data is marked stale', () => {
  assert.equal(isFresh({ status: 'ready', fetchedAt: 10 }, 911), false);
  assert.equal(isFresh({ status: 'ready', fetchedAt: 10 }, 910), true);
  assert.equal(isFresh({ status: 'ready', fetchedAt: 200 }, 10), false);
});
test('upstream text and paths cannot inject HTML', () => {
  assert.equal(escapeHtml('<img onerror="bad">&'), '&lt;img onerror=&quot;bad&quot;&gt;&amp;');
});
