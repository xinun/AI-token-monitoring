import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

// No inference, auth-token reads, full response dumps, or transcript access.
const child = spawn(process.env.AI_TOKEN_CODEX_PATH || 'codex', ['app-server', '--stdio', '-c', 'analytics.enabled=false'], {
  windowsHide: true, stdio: ['pipe', 'pipe', 'ignore'],
});
let complete = false;
const timer = setTimeout(() => finish({ ok: false, reason: 'timeout' }), 20_000);
function send(value) { child.stdin.write(JSON.stringify(value) + '\n'); }
function finish(value) {
  if (complete) return;
  complete = true; clearTimeout(timer); console.log(JSON.stringify(value, null, 2));
  child.kill(); process.exitCode = value.ok ? 0 : 1;
}
child.on('error', () => finish({ ok: false, reason: 'cli-unavailable' }));
child.on('exit', () => { if (!complete) finish({ ok: false, reason: 'connection-closed' }); });
const lines = createInterface({ input: child.stdout });
lines.on('line', line => {
  if (line.length > 1_048_576) return finish({ ok: false, reason: 'response-too-large' });
  let data; try { data = JSON.parse(line); } catch { return; }
  if (data.id === 1) {
    if (data.error) return finish({ ok: false, reason: 'initialize-failed' });
    send({ method: 'initialized', params: {} }); send({ id: 2, method: 'account/rateLimits/read' });
  }
  if (data.id === 2) {
    if (data.error) return finish({ ok: false, reason: 'quota-read-failed', errorCode: data.error.code });
    const result = data.result || {};
    const buckets = result.rateLimitsByLimitId && Object.keys(result.rateLimitsByLimitId).length
      ? Object.entries(result.rateLimitsByLimitId) : [['codex', result.rateLimits]];
    const windows = buckets.flatMap(([id, b]) => ['primary', 'secondary'].flatMap(key => {
      const w = b?.[key]; if (typeof w?.usedPercent !== 'number') return [];
      return [{ bucket: id, window: key, remainingPercent: Math.max(0, Math.min(100, 100 - w.usedPercent)), windowDurationMins: w.windowDurationMins, resetsAt: w.resetsAt }];
    }));
    finish({ ok: windows.length > 0, source: 'Codex app-server', windows });
  }
});
send({ id: 1, method: 'initialize', params: { clientInfo: { name: 'ai_token_probe', title: 'AI Token connection check', version: '0.1.0' } } });
