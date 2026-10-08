import './style.css';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { escapeHtml as esc, isFresh, remaining, resetLabel } from './presentation.ts';
import { downloadProgress, hasUpdate, releaseNotesHtml, updateBusy, updateStatusText, updateViewKey } from './update-presentation.ts';
import type { ProviderState, Settings, Snapshot, UpdateState } from './types.ts';

const native = isTauri();
const app = document.querySelector<HTMLDivElement>('#app')!;
const serviceInfo: Record<string, { name: string; subtitle: string; mark: string }> = {
  claude: { name: 'Claude', subtitle: 'Claude Code 구독', mark: '✳' },
  codex: { name: 'Codex', subtitle: 'OpenAI 구독', mark: '◎' },
  antigravity: { name: 'Antigravity', subtitle: 'Google 모델별 한도', mark: '△' },
  grok: { name: 'Grok', subtitle: 'xAI 웹 구독', mark: '𝕏' },
};
let settingsOpen = false;
let updatesOpen = false;
let installVersion: string | null = null;
let updateNotice = '';
let settingsDraft: Settings | null = null;
let demo = false;
let saving = false;
let notice = '';
let snapshot: Snapshot = {
  providers: Object.keys(serviceInfo).map(id => ({ id, status: 'unavailable', message: '데스크톱 앱에서 계정을 연결해 주세요.', windows: [], fetchedAt: null, source: '' })),
  settings: { codexEnabled: true, claudeEnabled: false, refreshMinutes: 5, miniEnabled: true, miniProviders: ["codex"], miniLocked: true, miniX: null, miniY: null, updateCheckEnabled: true }, refreshing: false,
  claudePath: '', bridgeCommand: '',
  updates: { currentVersion: '', availableVersion: null, releaseNotes: null, status: 'idle', checking: false, checkedAt: null, message: '', downloadedBytes: 0, totalBytes: null },
};

function preview(): Snapshot {
  const now = Math.floor(Date.now() / 1000);
  return { ...snapshot, updates: { ...snapshot.updates, currentVersion: '0.2.0', availableVersion: '0.2.1', releaseNotes: '화면 예시입니다. 실제 배포된 버전이 아닙니다.\n\n- 업데이트 알림 예시\n- 변경 사항을 이곳에서 확인할 수 있습니다.', status: 'available', checkedAt: now, message: '예시 데이터입니다.' }, providers: [
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

function readSettingsForm(): Settings | null {
  const element = document.querySelector<HTMLFormElement>('#settings-form');
  if (!element) return null;
  const form = new FormData(element);
  return { ...snapshot.settings, codexEnabled: form.has('codex'), claudeEnabled: form.has('claude'), refreshMinutes: Number(form.get('interval')), miniEnabled: form.has('miniEnabled'), miniProviders: form.getAll('miniProvider').map(String), miniLocked: form.has('miniLocked'), updateCheckEnabled: form.has('updateCheckEnabled') };
}

function updateDialog(update: UpdateState): string {
  const available = hasUpdate(update);
  const busy = updateBusy(update);
  const progress = downloadProgress(update);
  const installing = update.status === 'installing';
  return `<dialog id="update-dialog" open aria-modal="true" aria-labelledby="update-title"><div class="dialog-heading"><div><p class="eyebrow">APP UPDATES</p><h2 id="update-title">앱 업데이트</h2></div><button id="dismiss-updates" class="icon-button" aria-label="업데이트 창 닫기">×</button></div>
    ${!native ? `<p class="update-preview">${demo ? '예시 업데이트 · 실제 배포된 버전이 아닙니다.' : '브라우저 미리보기 · 업데이트 확인과 설치는 데스크톱 앱에서 사용할 수 있습니다.'}</p>` : ''}
    <div class="update-versions"><span>현재 버전<b>${update.currentVersion ? `v${esc(update.currentVersion)}` : '—'}</b></span><span>새 버전<b>${available ? `v${esc(update.availableVersion!)}` : '—'}</b></span></div>
    <p id="update-status" class="update-status ${update.status === 'error' ? 'update-error' : ''}" role="status" aria-live="polite">${esc(updateStatusText(update))}</p>
    ${updateNotice ? `<p class="update-status update-error" role="status">${esc(updateNotice)}</p>` : ''}
    ${update.status === 'downloading' ? `<progress class="update-progress" aria-label="업데이트 다운로드 진행률" max="100" ${progress === null ? '' : `value="${progress}"`}></progress>` : ''}
    ${update.checkedAt !== null ? `<p class="update-checked">마지막 확인 ${esc(new Date(update.checkedAt * 1000).toLocaleString('ko-KR', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' }))}</p>` : ''}
    ${available ? `<section class="release-notes" aria-labelledby="release-notes-title"><h3 id="release-notes-title">변경 사항</h3><pre>${releaseNotesHtml(update.releaseNotes)}</pre></section>` : ''}
    <div class="release-page-row"><button id="open-release" class="secondary-button" ${!native || busy ? 'disabled' : ''}>릴리즈 페이지 보기 <span aria-hidden="true">↗</span></button><span>브라우저에서 변경 사항과 설치 파일을 확인합니다.</span></div>
    ${installVersion && available && !busy ? `<div class="install-confirm"><b>v${esc(installVersion)} 업데이트를 설치할까요?</b><p>아래 ‘승인하고 설치’를 누르면 파일을 내려받고 배포 서명을 확인한 뒤 앱을 종료합니다. 설치 후 앱이 다시 실행됩니다.</p><div class="dialog-actions"><button id="cancel-install" class="secondary-button">취소</button><button id="confirm-install" class="primary-button" ${!native ? 'disabled' : ''}>승인하고 설치</button></div></div>` : `<p class="settings-hint">새 버전 확인은 자동으로 진행합니다. 다운로드와 설치는 사용자가 승인한 뒤에만 시작됩니다.</p><div class="dialog-actions"><button id="check-updates" class="secondary-button" ${!native || busy ? 'disabled' : ''}>${update.checking ? '확인 중…' : '새 버전 확인'}</button>${available ? `<button id="install-update" class="primary-button" ${!native || busy ? 'disabled' : ''}>${installing ? '설치 중…' : update.status === 'downloading' ? '다운로드 중…' : '업데이트 설치'}</button>` : ''}</div>`}
    </dialog><div class="scrim"></div>`;
}

function render() {
  if (settingsOpen) settingsDraft = readSettingsForm() || settingsDraft;
  const active = document.activeElement instanceof HTMLElement && app.contains(document.activeElement) ? document.activeElement : null;
  const focusId = active?.id;
  const focusName = active?.getAttribute('name');
  const focusValue = active instanceof HTMLInputElement ? active.value : null;
  const oldDialog = document.querySelector<HTMLDialogElement>('dialog');
  const scrollTop = oldDialog?.scrollTop ?? 0;
  const notesScrollTop = document.querySelector<HTMLElement>('.release-notes pre')?.scrollTop ?? 0;
  const detailsOpen = document.querySelector<HTMLDetailsElement>('dialog details')?.open ?? false;
  const data = demo ? preview() : { ...snapshot };
  if (settingsOpen && settingsDraft) data.settings = settingsDraft;
  const now = Math.floor(Date.now() / 1000);
  const connected = data.providers.filter(p => isFresh(p, now) && p.windows.some(w => remaining(w, now) !== null)).length;
  app.innerHTML = `<main><div class="dashboard-content" ${settingsOpen || updatesOpen ? 'inert' : ''}><header><div class="brand"><span class="app-mark" aria-hidden="true">◒</span><span>AI Token<small>QUOTA MONITOR</small></span></div><div class="header-actions"><button id="updates" class="icon-button update-bell" aria-label="${hasUpdate(data.updates) ? '새 버전 있음 · 앱 업데이트' : '앱 업데이트'}" aria-expanded="${updatesOpen}" aria-haspopup="dialog" title="앱 업데이트"><svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9Z"/><path d="M10 21h4"/></svg>${hasUpdate(data.updates) ? '<span class="update-badge" aria-hidden="true"></span>' : ''}</button><button id="settings" class="icon-button" aria-label="연결 설정" aria-expanded="${settingsOpen}" aria-haspopup="dialog" title="연결 설정">⚙</button>${native ? '<button id="close" class="icon-button" aria-label="창 닫기, 트레이에서 계속 실행" title="트레이로 돌아가기">×</button>' : ''}</div></header>
    <section class="intro"><div><p class="eyebrow">YOUR AI, AT A GLANCE</p><h1>남은 한도, 한눈에.</h1><p class="intro-copy">다음 작업 전에, AI의 여유를 확인하세요.</p></div><button id="refresh" class="refresh-button" ${snapshot.refreshing || !native ? 'disabled' : ''}><span aria-hidden="true">↻</span>${snapshot.refreshing ? '확인 중…' : '새로고침'}</button></section>
    ${!native ? `<div class="preview-banner"><span>${demo ? '화면 미리보기 · 실제 계정 사용량이 아닙니다.' : '브라우저 미리보기 · 계정 조회는 데스크톱 앱에서 실행됩니다.'}</span><button id="demo">${demo ? '예시 끄기' : '예시 보기'}</button></div>` : ''}
    <div class="overview"><span><i class="live-dot"></i>${connected}개 서비스 ${demo ? '예시 연결' : '최신 한도 확인'}</span><span>${data.settings.refreshMinutes}분 간격으로 갱신</span></div>
    <section class="providers" aria-label="서비스별 남은 한도">${data.providers.map(p => card(p, now)).join('')}</section>
    <div class="notice" role="status" aria-live="polite">${esc(notice)}</div>
    <footer><span>◌ 한도 확인에는 AI 토큰을 사용하지 않아요.</span><span>${native ? data.updates.currentVersion ? `v${esc(data.updates.currentVersion)}` : '버전 확인 중…' : '브라우저 미리보기'}</span></footer></div>
    ${settingsOpen ? `<dialog id="settings-dialog" open aria-modal="true" aria-labelledby="settings-title"><div class="dialog-heading"><div><p class="eyebrow">CONNECTIONS</p><h2 id="settings-title">연결 설정</h2></div><button id="dismiss" class="icon-button" aria-label="설정 닫기">×</button></div><form id="settings-form"><label class="toggle-row"><span><b>Codex</b><small>설치된 Codex CLI의 로그인 계정으로 조회</small></span><input id="setting-codex" type="checkbox" name="codex" ${data.settings.codexEnabled ? 'checked' : ''}></label><label class="toggle-row"><span><b>Claude Code</b><small>상태줄에서 전달한 한도 데이터 읽기</small></span><input id="setting-claude" type="checkbox" name="claude" ${data.settings.claudeEnabled ? 'checked' : ''}></label><label class="interval-row">자동 조회 간격<select id="setting-interval" name="interval">${[5,10,15].map(m => `<option value="${m}" ${m === data.settings.refreshMinutes ? 'selected' : ''}>${m}분</option>`).join('')}</select></label>
    <label class="toggle-row"><span><b>새 버전 자동 확인</b><small>앱 시작 시와 6시간 간격으로 확인합니다.<br>설치는 직접 선택할 때만 진행합니다.</small></span><input id="setting-update-check" type="checkbox" name="updateCheckEnabled" ${data.settings.updateCheckEnabled ? 'checked' : ''}></label>
    <fieldset class="mini-settings"><legend>작업표시줄 미니바</legend>
    <label class="toggle-row"><span><b>미니바 상시 표시</b></span><input type="checkbox" name="miniEnabled" ${data.settings.miniEnabled ? 'checked' : ''}></label>
    <div class="mini-choices">${Object.entries(serviceInfo).map(([id,info]) => `<label><input type="checkbox" name="miniProvider" value="${id}" ${data.settings.miniProviders.includes(id) ? 'checked' : ''}><img src="/brands/${id}.png" alt="" width="20" height="20">${info.name}</label>`).join('')}</div>
    <label class="toggle-row"><span><b>위치 잠금</b><small>잠금을 풀고 미니바 왼쪽 손잡이를 끌어 이동</small></span><input type="checkbox" name="miniLocked" ${data.settings.miniLocked ? 'checked' : ''}></label>
    <p class="settings-hint">클릭: 상세 창 · 우클릭: 위치 잠금/초기화/숨기기. 단기·주간 잔여 한도를 별도로 표시합니다. 현재 미니바는 Windows에서 제공됩니다.</p></fieldset>
    <details><summary>Claude Code 연결 방법</summary><p>아래는 <b>이 PC용으로 자동 생성된 명령</b>입니다. 사용자 이름이나 경로를 직접 바꿀 필요가 없습니다. Claude Code 사용자 설정의 <code>statusLine.command</code>에 연결하세요. 기존 상태줄 설정이 있으면 덮어쓰지 말고 함께 실행하도록 구성하세요.</p><textarea readonly aria-label="이 PC용 Claude 상태줄 연결 명령">${esc(data.bridgeCommand || '데스크톱 앱에서 연결 명령을 확인할 수 있습니다.')}</textarea><div class="bridge-actions"><button type="button" id="copy-bridge" class="secondary-button" ${!native ? 'disabled' : ''}>이 PC용 명령 복사</button><button type="button" id="copy-bridge-json" class="secondary-button" ${!native ? 'disabled' : ''}>설정 JSON 복사</button><span id="copy-feedback" role="status"></span></div><p>다른 PC에서는 그 PC의 AI Token에서 명령을 복사하세요. 기존 JSON 설정에는 <code>statusLine</code> 항목만 병합하세요.</p><p>한 번 작업하면 한도가 전달됩니다. 앱은 대화 내용과 인증 정보를 읽지 않습니다. Claude Code가 쉬는 동안에는 마지막 기록만 유지됩니다.</p><p class="path">${esc(data.claudePath)}</p></details>
    <p class="settings-hint">Antigravity·Grok는 조회 경로 검증 후 연결을 제공할 예정입니다. 구독 잔여 한도와 대화의 컨텍스트 사용률은 다릅니다.</p><p class="notice" role="status">${esc(notice)}</p><div class="dialog-actions"><button id="cancel" type="button" class="secondary-button">취소</button><button id="save-settings" type="submit" class="primary-button" ${!native || saving ? 'disabled' : ''}>${saving ? '저장 중…' : '설정 저장'}</button></div></form></dialog><div class="scrim"></div>` : ''}
    ${updatesOpen ? updateDialog(data.updates) : ''}
  </main>`;
  document.querySelector('#settings')?.addEventListener('click', openSettings);
  document.querySelector('#updates')?.addEventListener('click', openUpdates);
  document.querySelectorAll('.connect').forEach(b => b.addEventListener('click', openSettings));
  document.querySelector('#dismiss')?.addEventListener('click', dismissSettings);
  document.querySelector('#cancel')?.addEventListener('click', dismissSettings);
  document.querySelector('#dismiss-updates')?.addEventListener('click', dismissUpdates);
  document.querySelector('#check-updates')?.addEventListener('click', () => void checkUpdates());
  document.querySelector('#open-release')?.addEventListener('click', () => void openReleasePage());
  document.querySelector('#install-update')?.addEventListener('click', () => { if (!native || updateBusy(snapshot.updates) || !hasUpdate(snapshot.updates)) return; installVersion = snapshot.updates.availableVersion; updateNotice = ''; render(); document.querySelector<HTMLButtonElement>('#cancel-install')?.focus(); });
  document.querySelector('#cancel-install')?.addEventListener('click', () => { installVersion = null; render(); document.querySelector<HTMLButtonElement>('#install-update')?.focus(); });
  document.querySelector('#confirm-install')?.addEventListener('click', () => void installUpdate());
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
    const settings = readSettingsForm();
    if (!settings || !native) return;
    settingsDraft = settings;
    saving = true; notice = ''; render();
    try {
      await invoke('save_settings', { settings });
      snapshot = await invoke<Snapshot>('get_snapshot'); settingsOpen = false; settingsDraft = null; notice = '설정을 저장했습니다.';
    } catch (e) { notice = typeof e === 'string' ? e : '설정 저장 실패 · 파일 접근 권한을 확인하세요.'; }
    saving = false; render();
    if (!settingsOpen) document.querySelector<HTMLButtonElement>('#settings')?.focus();
  });
  const newDialog = document.querySelector<HTMLDialogElement>('dialog');
  if (newDialog) newDialog.scrollTop = scrollTop;
  const notes = document.querySelector<HTMLElement>('.release-notes pre');
  if (notes) notes.scrollTop = notesScrollTop;
  const details = document.querySelector<HTMLDetailsElement>('dialog details');
  if (details) details.open = detailsOpen;
  const restoreTarget = focusId ? document.getElementById(focusId) : focusName ? [...app.querySelectorAll<HTMLElement>('[name]')].find(element => element.getAttribute('name') === focusName && (focusValue === null || element instanceof HTMLInputElement && element.value === focusValue)) : null;
  if (restoreTarget && !restoreTarget.closest('[inert]') && !restoreTarget.matches(':disabled')) restoreTarget.focus({ preventScroll: true });
  else if (newDialog && !newDialog.contains(document.activeElement)) newDialog.querySelector<HTMLButtonElement>('button:not([disabled])')?.focus({ preventScroll: true });
}
async function copyBridge(json: boolean) {
  const feedback = document.querySelector('#copy-feedback');
  try {
    const text = json ? JSON.stringify({ statusLine: { type: 'command', command: snapshot.bridgeCommand } }, null, 2) : snapshot.bridgeCommand;
    await navigator.clipboard.writeText(text);
    if (feedback) feedback.textContent = '복사했습니다.';
  } catch { if (feedback) feedback.textContent = '자동 복사 실패 · 위 명령을 직접 복사하세요.'; }
}
function openSettings() { updatesOpen = false; installVersion = null; settingsDraft = { ...snapshot.settings, miniProviders: [...snapshot.settings.miniProviders] }; settingsOpen = true; render(); document.querySelector<HTMLButtonElement>('#dismiss')?.focus(); }
function dismissSettings() { if (saving) return; settingsOpen = false; settingsDraft = null; render(); document.querySelector<HTMLButtonElement>('#settings')?.focus(); }
function openUpdates() { if (saving) return; settingsOpen = false; settingsDraft = null; updatesOpen = true; installVersion = null; updateNotice = ''; render(); document.querySelector<HTMLButtonElement>('#dismiss-updates')?.focus(); }
function dismissUpdates() { updatesOpen = false; installVersion = null; render(); document.querySelector<HTMLButtonElement>('#updates')?.focus(); }

async function openReleasePage() {
  if (!native || updateBusy(snapshot.updates)) return;
  updateNotice = '';
  try { await invoke('open_release_page'); }
  catch (error) { updateNotice = typeof error === 'string' ? error : '릴리즈 페이지를 열지 못했습니다.'; }
  if (updatesOpen) render();
}

async function checkUpdates() {
  if (!native || updateBusy(snapshot.updates)) return;
  snapshot.updates = { ...snapshot.updates, checking: true, status: 'checking' };
  installVersion = null;
  updateNotice = '';
  render();
  try {
    await invoke('check_for_updates');
    receiveSnapshot(await invoke<Snapshot>('get_snapshot'));
  } catch (error) {
    snapshot.updates = { ...snapshot.updates, checking: false, status: 'error', message: typeof error === 'string' ? error : '업데이트를 확인하지 못했습니다. 다시 시도해 주세요.' };
    render();
  }
}

async function installUpdate() {
  if (!native || !hasUpdate(snapshot.updates) || updateBusy(snapshot.updates) || !installVersion) return;
  const expectedVersion = installVersion;
  installVersion = null;
  updateNotice = '';
  snapshot.updates = { ...snapshot.updates, status: 'downloading', downloadedBytes: 0, totalBytes: null };
  render();
  try {
    await invoke('install_update', { expectedVersion });
    receiveSnapshot(await invoke<Snapshot>('get_snapshot'));
  } catch (error) {
    snapshot.updates = { ...snapshot.updates, checking: false, status: 'error', message: typeof error === 'string' ? error : '업데이트를 설치하지 못했습니다. 다시 시도해 주세요.' };
    render();
  }
}

function receiveSnapshot(next: Snapshot) {
  const updateChanged = updateViewKey(snapshot.updates) !== updateViewKey(next.updates);
  snapshot = next;
  if (settingsOpen || updatesOpen && !updateChanged) return;
  if (installVersion && installVersion !== next.updates.availableVersion) {
    installVersion = null;
    updateNotice = '새 버전 정보가 변경되었습니다. 버전을 확인하고 다시 설치를 선택해 주세요.';
  }
  render();
}
document.addEventListener('keydown', event => {
  if (event.key === 'Escape') {
    if (settingsOpen) { event.preventDefault(); dismissSettings(); }
    else if (updatesOpen) { event.preventDefault(); dismissUpdates(); }
  }
  if (event.key === 'Tab' && (settingsOpen || updatesOpen)) {
    const elements = [...document.querySelectorAll<HTMLElement>('dialog button:not([disabled]), dialog input:not([disabled]), dialog select:not([disabled]), dialog summary, dialog textarea:not([disabled])')].filter(element => element.getClientRects().length > 0);
    const first = elements[0], last = elements.at(-1);
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
  }
});

render();
async function connectDesktop() {
  if (!native) return;
  // Subscribe before reading to avoid dropping a refresh that completes during startup.
  await listen<Snapshot>('quota-updated', event => receiveSnapshot(event.payload));
  try { receiveSnapshot(await invoke<Snapshot>('get_snapshot')); }
  catch { notice = '사용량을 읽지 못했습니다. 앱을 다시 실행해 주세요.'; render(); }
}
void connectDesktop();
