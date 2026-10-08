import './style.css';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { escapeHtml as esc, isFresh, remaining, resetLabel } from './presentation.ts';
import type { ProviderState, Snapshot } from './types.ts';

const native = isTauri();
const app = document.querySelector<HTMLDivElement>('#app')!;
const serviceInfo: Record<string, { name: string; subtitle: string; mark: string }> = {
  claude: { name: 'Claude', subtitle: 'Claude Code 구독', mark: '✳' },
  codex: { name: 'Codex', subtitle: 'OpenAI 구독', mark: '◎' },
  antigravity: { name: 'Antigravity', subtitle: 'Google 모델별 한도', mark: '△' },
  grok: { name: 'Grok', subtitle: 'xAI 웹 구독', mark: '𝕏' },
};
let settingsOpen = false;
let demo = false;
let saving = false;
let notice = '';
let snapshot: Snapshot = {
  providers: Object.keys(serviceInfo).map(id => ({ id, status: 'unavailable', message: '데스크톱 앱에서 계정을 연결해 주세요.', windows: [], fetchedAt: null, source: '' })),
  settings: { codexEnabled: true, claudeEnabled: false, refreshMinutes: 5, miniEnabled: true, miniProviders: ["codex"], miniLocked: true, miniX: null, miniY: null }, refreshing: false,
  claudePath: '', bridgeCommand: '',
};

function preview(): Snapshot {
  const now = Math.floor(Date.now() / 1000);
  return { ...snapshot, providers: [
    { id: 'claude', status: 'ready', message: '예시 데이터입니다.', fetchedAt: now, source: '예시', windows: [{ label: '5시간', usedPercent: 68, resetsAt: now + 8340 }, { label: '주간', usedPercent: 23, resetsAt: now + 235800 }] },
    { id: 'codex', status: 'ready', message: '예시 데이터입니다.', fetchedAt: now, source: '예시', windows: [{ label: 'Codex · 5시간', usedPercent: 26, resetsAt: now + 6600 }, { label: 'Codex · 주간', usedPercent: 81, resetsAt: now + 126000 }] },
    { id: 'antigravity', status: 'unsupported', message: '외부 한도 조회 경로를 검증 중입니다.', windows: [], fetchedAt: null, source: '' },
    { id: 'grok', status: 'unsupported', message: '웹 구독 한도 연동 경로가 아직 확인되지 않았습니다.', windows: [], fetchedAt: null, source: '' },
  ] };
}

function card(provider: ProviderState, now: number): string {
  const info = serviceInfo[provider.id];
  if (!info) return '';
  const fresh = isFresh(provider, now);
  const known = fresh ? provider.windows.map(w => remaining(w, now)).filter((p): p is number => p !== null) : [];
  const status = known.length ? '연결됨' : provider.status === 'unsupported' ? '지원 준비 중' : provider.status === 'disabled' ? '연결 꺼짐' : '확인 필요';
  const bars = provider.windows.map(w => {
    const value = remaining(w, now);
    const usable = fresh && value !== null;
    return `<div class="quota ${usable ? '' : 'stale'}"><div class="quota-line"><span>${esc(w.label)}</span><span class="quota-value">${usable ? `<b>${Math.round(value)}<small>%</small></b> <span>잔여</span>` : '<b>—</b> <span>최신 값 없음</span>'}</span></div>
      <div class="bar" role="${usable ? 'progressbar' : 'presentation'}" ${usable ? `aria-label="${esc(info.name + ' ' + w.label)} 잔여 한도" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${value}"` : ''}><i class="${usable && value <= 20 ? 'low' : ''}" style="width:${usable ? value : 0}%"></i></div>
      <div class="reset">${esc(resetLabel(w.resetsAt, now))}${!usable && value !== null ? ` · 마지막 기록 잔여 ${Math.round(value)}%` : ''}</div></div>`;
  }).join('');
  return `<article class="provider ${provider.id}"><div class="provider-heading"><span class="service-mark" aria-hidden="true"><img src="/brands/${provider.id}.png" alt="" width="28" height="28"></span><div><h2>${info.name}</h2><p>${info.subtitle}</p></div><span class="status ${known.length ? 'connected' : ''}"><i></i>${status}</span></div>
    ${bars || `<div class="empty-quota"><span class="empty-value">—<small>잔여 한도</small></span><p>${esc(provider.message)}</p>${provider.id === 'claude' || provider.id === 'codex' ? '<button class="text-button connect" type="button">연결 설정 <span aria-hidden="true">↗</span></button>' : '<span class="pending-note">확인되지 않은 값은 표시하지 않아요</span>'}</div>`}
    ${bars ? `<div class="provider-foot">${esc(provider.source)}<span>${provider.fetchedAt !== null ? new Date(provider.fetchedAt * 1000).toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' }) + ' 갱신' : ''}</span></div><p class="source-note">${esc(provider.message)}</p>` : ''}</article>`;
}

function render() {
  const data = demo ? preview() : snapshot;
  const now = Math.floor(Date.now() / 1000);
  const connected = data.providers.filter(p => isFresh(p, now) && p.windows.some(w => remaining(w, now) !== null)).length;
  app.innerHTML = `<main><header><div class="brand"><span class="app-mark" aria-hidden="true">◒</span><span>AI Token<small>QUOTA MONITOR</small></span></div><div class="header-actions"><button id="settings" class="icon-button" aria-label="연결 설정" aria-expanded="${settingsOpen}" title="연결 설정">⚙</button>${native ? '<button id="close" class="icon-button" aria-label="창 닫기, 트레이에서 계속 실행" title="트레이로 돌아가기">×</button>' : ''}</div></header>
    <section class="intro"><div><p class="eyebrow">YOUR AI, AT A GLANCE</p><h1>남은 한도, 한눈에.</h1><p class="intro-copy">다음 작업 전에, AI의 여유를 확인하세요.</p></div><button id="refresh" class="refresh-button" ${snapshot.refreshing || !native ? 'disabled' : ''}><span aria-hidden="true">↻</span>${snapshot.refreshing ? '확인 중…' : '새로고침'}</button></section>
    ${!native ? `<div class="preview-banner"><span>${demo ? '화면 미리보기 · 실제 계정 사용량이 아닙니다.' : '브라우저 미리보기 · 계정 조회는 데스크톱 앱에서 실행됩니다.'}</span><button id="demo">${demo ? '예시 끄기' : '예시 보기'}</button></div>` : ''}
    <div class="overview"><span><i class="live-dot"></i>${connected}개 서비스 ${demo ? '예시 연결' : '최신 한도 확인'}</span><span>${data.settings.refreshMinutes}분 간격으로 갱신</span></div>
    <section class="providers" aria-label="서비스별 남은 한도">${data.providers.map(p => card(p, now)).join('')}</section>
    <div class="notice" role="status" aria-live="polite">${esc(notice)}</div>
    <footer><span>◌ 한도 확인에는 AI 토큰을 사용하지 않아요.</span><span>v0.1.0</span></footer>
    ${settingsOpen ? `<dialog open aria-labelledby="settings-title"><div class="dialog-heading"><div><p class="eyebrow">CONNECTIONS</p><h2 id="settings-title">연결 설정</h2></div><button id="dismiss" class="icon-button" aria-label="설정 닫기">×</button></div><form id="settings-form"><label class="toggle-row"><span><b>Codex</b><small>설치된 Codex CLI의 로그인 계정으로 조회</small></span><input type="checkbox" name="codex" ${data.settings.codexEnabled ? 'checked' : ''}></label><label class="toggle-row"><span><b>Claude Code</b><small>상태줄에서 전달한 한도 데이터 읽기</small></span><input type="checkbox" name="claude" ${data.settings.claudeEnabled ? 'checked' : ''}></label><label class="interval-row">자동 조회 간격<select name="interval">${[5,10,15].map(m => `<option value="${m}" ${m === data.settings.refreshMinutes ? 'selected' : ''}>${m}분</option>`).join('')}</select></label>
    <fieldset class="mini-settings"><legend>작업표시줄 미니바</legend>
    <label class="toggle-row"><span><b>미니바 상시 표시</b></span><input type="checkbox" name="miniEnabled" ${data.settings.miniEnabled ? 'checked' : ''}></label>
    <div class="mini-choices">${Object.entries(serviceInfo).map(([id,info]) => `<label><input type="checkbox" name="miniProvider" value="${id}" ${data.settings.miniProviders.includes(id) ? 'checked' : ''}><img src="/brands/${id}.png" alt="" width="20" height="20">${info.name}</label>`).join('')}</div>
    <label class="toggle-row"><span><b>위치 잠금</b><small>잠금을 풀고 미니바 왼쪽 손잡이를 끌어 이동</small></span><input type="checkbox" name="miniLocked" ${data.settings.miniLocked ? 'checked' : ''}></label>
    <p class="settings-hint">클릭: 상세 창 · 우클릭: 위치 잠금/초기화/숨기기. 단기·주간 잔여 한도를 별도로 표시합니다. 현재 미니바는 Windows에서 제공됩니다.</p></fieldset>
    <details><summary>Claude Code 연결 방법</summary><p>아래는 <b>이 PC용으로 자동 생성된 명령</b>입니다. 사용자 이름이나 경로를 직접 바꿀 필요가 없습니다. Claude Code 사용자 설정의 <code>statusLine.command</code>에 연결하세요. 기존 상태줄 설정이 있으면 덮어쓰지 말고 함께 실행하도록 구성하세요.</p><textarea readonly aria-label="이 PC용 Claude 상태줄 연결 명령">${esc(data.bridgeCommand || '데스크톱 앱에서 연결 명령을 확인할 수 있습니다.')}</textarea><div class="bridge-actions"><button type="button" id="copy-bridge" class="secondary-button" ${!native ? 'disabled' : ''}>이 PC용 명령 복사</button><button type="button" id="copy-bridge-json" class="secondary-button" ${!native ? 'disabled' : ''}>설정 JSON 복사</button><span id="copy-feedback" role="status"></span></div><p>다른 PC에서는 그 PC의 AI Token에서 명령을 복사하세요. 기존 JSON 설정에는 <code>statusLine</code> 항목만 병합하세요.</p><p>한 번 작업하면 한도가 전달됩니다. 앱은 대화 내용과 인증 정보를 읽지 않습니다. Claude Code가 쉬는 동안에는 마지막 기록만 유지됩니다.</p><p class="path">${esc(data.claudePath)}</p></details>
    <p class="settings-hint">Antigravity·Grok는 조회 경로 검증 후 연결을 제공할 예정입니다. 구독 잔여 한도와 대화의 컨텍스트 사용률은 다릅니다.</p><p class="notice" role="status">${esc(notice)}</p><div class="dialog-actions"><button id="cancel" type="button" class="secondary-button">취소</button><button type="submit" class="primary-button" ${!native || saving ? 'disabled' : ''}>${saving ? '저장 중…' : '설정 저장'}</button></div></form></dialog><div class="scrim"></div>` : ''}
  </main>`;
  document.querySelector('#settings')?.addEventListener('click', openSettings);
  document.querySelectorAll('.connect').forEach(b => b.addEventListener('click', openSettings));
  document.querySelector('#dismiss')?.addEventListener('click', dismissSettings);
  document.querySelector('#cancel')?.addEventListener('click', dismissSettings);
  document.querySelector('#copy-bridge')?.addEventListener('click', () => void copyBridge(false));
  document.querySelector('#copy-bridge-json')?.addEventListener('click', () => void copyBridge(true));
  document.querySelector('#demo')?.addEventListener('click', () => { demo = !demo; render(); });
  document.querySelector('#close')?.addEventListener('click', () => void getCurrentWindow().close());
  document.querySelector('#refresh')?.addEventListener('click', async () => {
    notice = ''; snapshot.refreshing = true; render();
    try { await invoke('refresh_now'); snapshot = await invoke<Snapshot>('get_snapshot'); }
    catch (e) { notice = typeof e === 'string' ? e : '갱신하지 못했습니다.'; snapshot.refreshing = false; }
    render();
  });
  document.querySelector<HTMLFormElement>('#settings-form')?.addEventListener('submit', async event => {
    event.preventDefault();
    const form = new FormData(event.currentTarget as HTMLFormElement);
    saving = true; notice = ''; render();
    try {
      await invoke('save_settings', { settings: { codexEnabled: form.has('codex'), claudeEnabled: form.has('claude'), refreshMinutes: Number(form.get('interval')), miniEnabled: form.has('miniEnabled'), miniProviders: form.getAll('miniProvider').map(String), miniLocked: form.has('miniLocked'), miniX: snapshot.settings.miniX, miniY: snapshot.settings.miniY } });
      snapshot = await invoke<Snapshot>('get_snapshot'); settingsOpen = false; notice = '설정을 저장했습니다.';
    } catch (e) { notice = typeof e === 'string' ? e : '설정 저장 실패 · 파일 접근 권한을 확인하세요.'; }
    saving = false; render();
  });
}
async function copyBridge(json: boolean) {
  const feedback = document.querySelector('#copy-feedback');
  try {
    const text = json ? JSON.stringify({ statusLine: { type: 'command', command: snapshot.bridgeCommand } }, null, 2) : snapshot.bridgeCommand;
    await navigator.clipboard.writeText(text);
    if (feedback) feedback.textContent = '복사했습니다.';
  } catch { if (feedback) feedback.textContent = '자동 복사 실패 · 위 명령을 직접 복사하세요.'; }
}
function openSettings() { settingsOpen = true; render(); document.querySelector<HTMLButtonElement>('#dismiss')?.focus(); }
function dismissSettings() { if (saving) return; settingsOpen = false; render(); document.querySelector<HTMLButtonElement>('#settings')?.focus(); }
document.addEventListener('keydown', event => {
  if (event.key === 'Escape' && settingsOpen) dismissSettings();
  if (event.key === 'Tab' && settingsOpen) {
    const elements = [...document.querySelectorAll<HTMLElement>('dialog button:not([disabled]), dialog input, dialog select, dialog summary, dialog textarea')];
    const first = elements[0], last = elements.at(-1);
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
  }
});

render();
async function connectDesktop() {
  if (!native) return;
  // Subscribe before reading to avoid dropping a refresh that completes during startup.
  await listen<Snapshot>('quota-updated', event => { snapshot = event.payload; if (!settingsOpen) render(); });
  try { snapshot = await invoke<Snapshot>('get_snapshot'); render(); }
  catch { notice = '사용량을 읽지 못했습니다. 앱을 다시 실행해 주세요.'; render(); }
}
void connectDesktop();
