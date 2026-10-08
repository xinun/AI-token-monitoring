import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

test('Claude bridge only persists quota fields and replaces missing limits', { skip: process.platform !== 'win32' }, () => {
  const temp = mkdtempSync(join(tmpdir(), 'ai-token-bridge-'));
  try {
    const output = join(temp, 'quota.json');
    const script = fileURLToPath(new URL('./claude-statusline.ps1', import.meta.url));
    for (const rate_limits of [{ five_hour: { used_percentage: 64, resets_at: 2000000000 } }, {}]) {
      const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', script, '-OutputPath', output], {
        input: JSON.stringify({ rate_limits, api_key: 'DO_NOT_PERSIST', transcript_path: 'PRIVATE', context_window: { used_percentage: 99 } }), encoding: 'utf8', windowsHide: true,
      });
      assert.equal(result.status, 0, result.stderr);
      const raw = readFileSync(output, 'utf8');
      assert.ok(!raw.includes('DO_NOT_PERSIST') && !raw.includes('PRIVATE') && !raw.includes('context_window'));
      assert.deepEqual(JSON.parse(raw).rate_limits, rate_limits);
    }
  } finally { rmSync(temp, { recursive: true, force: true }); }
});

test('Claude bridge output follows each Windows user AppData folder', { skip: process.platform !== 'win32' }, () => {
  const temp = mkdtempSync(join(tmpdir(), 'ai-token-profiles-'));
  try {
    const script = fileURLToPath(new URL('./claude-statusline.ps1', import.meta.url));
    for (const profile of ['profile one', '다른 사용자']) {
      const appData = join(temp, profile);
      const result = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', script], {
        input: JSON.stringify({ rate_limits: { five_hour: { used_percentage: 12 } } }),
        env: { ...process.env, APPDATA: appData }, encoding: 'utf8', windowsHide: true,
      });
      assert.equal(result.status, 0, result.stderr);
      const data = JSON.parse(readFileSync(join(appData, 'com.aitoken.desktop', 'claude-usage.json'), 'utf8'));
      assert.equal(data.rate_limits.five_hour.used_percentage, 12);
    }
  } finally { rmSync(temp, { recursive: true, force: true }); }
});
