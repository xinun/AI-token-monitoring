import test from 'node:test';
import assert from 'node:assert/strict';
import { downloadProgress, hasUpdate, releaseNotesHtml, updateBusy, updateStatusText } from '../src/update-presentation.ts';

const idle = { currentVersion: '0.2.0', availableVersion: null, releaseNotes: null, status: 'idle', checking: false, checkedAt: null, message: '', downloadedBytes: 0, totalBytes: null };

test('update badge is retained for retry after failed download and absent for current version', () => {
  assert.equal(hasUpdate(idle), false);
  assert.equal(hasUpdate({ ...idle, availableVersion: '0.2.0', status: 'upToDate' }), false);
  assert.equal(hasUpdate({ ...idle, availableVersion: '0.2.1', status: 'error' }), true);
  assert.equal(updateBusy({ ...idle, status: 'downloading' }), true);
  assert.equal(updateBusy({ ...idle, status: 'installing' }), true);
  assert.equal(updateBusy({ ...idle, status: 'error' }), false);
});

test('download without content length uses an indeterminate indicator, never NaN percent', () => {
  const unknown = { ...idle, status: 'downloading', downloadedBytes: 2 * 1024 * 1024 };
  assert.equal(downloadProgress(unknown), null);
  assert.match(updateStatusText(unknown), /2\.0 MB/);
  assert.doesNotMatch(updateStatusText(unknown), /NaN|Infinity|%/);
  for (const totalBytes of [0, -1, NaN, Infinity]) assert.equal(downloadProgress({ ...unknown, totalBytes }), null);
});

test('known-length progress is bounded and installation explains application restart', () => {
  assert.equal(downloadProgress({ ...idle, totalBytes: 100, downloadedBytes: 50 }), 50);
  assert.equal(downloadProgress({ ...idle, totalBytes: 100, downloadedBytes: 120 }), 100);
  assert.equal(downloadProgress({ ...idle, totalBytes: 100, downloadedBytes: -10 }), 0);
  assert.match(updateStatusText({ ...idle, status: 'installing' }), /종료.*다시 실행/);
});

test('release text remains plain escaped text, including Markdown and attacker HTML', () => {
  const notes = '# Fixes\n<script>alert("bad")</script>\n[link](javascript:bad)&';
  const html = releaseNotesHtml(notes);
  assert.ok(html.startsWith('# Fixes\n&lt;script&gt;'));
  assert.doesNotMatch(html, /<script>|<a\s/);
  assert.match(html, /&quot;bad&quot;/);
  assert.match(html, /&amp;$/);
  assert.equal(releaseNotesHtml('x'.repeat(6000)).length, 4000);
  assert.equal(releaseNotesHtml(null), '등록된 변경 사항이 없습니다.');
});
