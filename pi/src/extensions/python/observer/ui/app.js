/** Read-only HTTP observation. No Python expressions or terminal input are sent from this page. */
export {};
/** @typedef {import('../types.ts').ObserverStatus} ObserverStatus */
/** @typedef {import('../types.ts').InspectRequest} InspectRequest */
/** @typedef {import('../types.ts').InspectResult} InspectResult */
/** @typedef {import('../types.ts').InspectData} InspectData */
/** @typedef {import('../types.ts').ValuePath} ValuePath */
/** @typedef {import('../types.ts').ValueSummary} ValueSummary */
/** @typedef {import('../types.ts').TerminalInfo} TerminalInfo */
/** @typedef {import('../types.ts').CellColor} CellColor */
/** @typedef {Extract<InspectData, {view: 'variables'}>} VariablesData */
/** @typedef {Extract<InspectData, {view: 'value'}>} ValueData */
/** @typedef {Extract<InspectData, {view: 'terminal', mode: 'screen'}>} ScreenData */
/** @typedef {Extract<InspectData, {view: 'terminal', mode: 'raw'}>} RawData */
/** @typedef {'variables' | 'execution' | 'terminal'} View */
/** @typedef {{name: string, path: ValuePath, offset: number}} ValueSelection */
/** @typedef {'unauthorized' | 'invalid_request' | 'unavailable' | 'internal_error' | 'disconnected'} ApiErrorCode */

const REFRESH_MS = 500;
const RAW_LIMIT_BYTES = 64 * 1024;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** @param {string} id */
function element(id) {
  return /** @type {HTMLElement} */ (document.getElementById(id));
}

const ui = {
  pause: /** @type {HTMLButtonElement} */ (element('pause-refresh')),
  search: /** @type {HTMLInputElement} */ (element('variable-search')),
  definitions: /** @type {HTMLInputElement} */ (element('show-definitions')),
  followOutput: /** @type {HTMLInputElement} */ (element('follow-output')),
  variableRows: element('variable-rows'),
  children: element('value-children'),
  terminalList: element('terminal-list'),
  breadcrumbs: element('value-breadcrumbs'),
  screenGrid: element('screen-grid'),
  screenScroll: element('terminal-screen'),
  rawOutput: element('terminal-raw'),
  output: element('execution-output'),
};

/** @type {View} */
let view = 'variables';
let paused = false;
let token = readToken();
/** @type {ObserverStatus | null} */
let status = null;
/** @type {ApiErrorCode | null} */
let serviceError = null;
let connected = false;
let statusReceivedAt = 0;
let executionSampledAt = 0;
let workspaceEpoch = 0;
let viewRevision = 0;
let readMessage = '';
let valueMessage = '';
let terminalMessage = '';
let inspectStartedAt = 0;
let inspecting = false;
let readingStatus = false;
/** @type {AbortController | null} */
let statusController = null;
/** @type {AbortController | null} */
let inspectController = null;
/** @type {number | undefined} */
let pollTimer;

const variables = {
  search: '', definitions: false, offset: 0,
  /** @type {VariablesData | null} */
  data: null,
  /** @type {ValueSelection | null} */
  selection: null,
  /** @type {ValueData | null} */
  value: null,
  listSampledAt: 0, valueSampledAt: 0,
};
const terminals = {
  /** @type {TerminalInfo[] | null} */
  list: null,
  /** @type {string | null} */
  selectedId: null,
  /** @type {'screen' | 'raw'} */
  mode: 'screen',
  /** @type {ScreenData | null} */
  screen: null,
  sampledAt: 0,
  raw: newRawBuffer(),
};

function newRawBuffer() {
  return {
    text: '',
    /** @type {number | undefined} */
    since: undefined,
    lostBytes: 0, droppedBytes: 0, trimmedBytes: 0,
    initialTail: false,
  };
}

const STATE_LABELS = {
  not_started: '尚未启动', starting: '正在启动', running: '正在执行',
  idle: '空闲', stopping: '正在退出', exited: '已退出',
};
/** @type {Record<string, string>} */
const OUTCOME_LABELS = {
  completed: '已完成', python_error: 'Python 报错', interrupted: '已中断',
  timed_out: '已超时', process_exited: 'Python 进程已退出',
  startup_error: 'Python 启动失败', output_error: '输出读取失败',
};
/** @type {Record<ApiErrorCode, string>} */
const API_ERROR_LABELS = {
  unauthorized: '访问凭证失效',
  invalid_request: '请求无效',
  unavailable: '服务不可用',
  internal_error: '读取失败',
  disconnected: '连接已断开',
};

function readToken() {
  return new URLSearchParams(location.hash.slice(1)).get('token') ?? '';
}

class ApiError extends Error {
  /** @param {ApiErrorCode} code */
  constructor(code) {
    super(code);
    this.code = code;
  }
}

/**
 * The fragment secret stays in memory and is used only for same-origin API authorization.
 * @template T
 * @param {string} path
 * @param {AbortSignal} signal
 * @param {InspectRequest} [body]
 * @returns {Promise<T>}
 */
async function requestApi(path, signal, body) {
  let response;
  try {
    response = await fetch(path, {
      method: body ? 'POST' : 'GET',
      headers: {
        Authorization: `Bearer ${token}`,
        ...(body ? { 'Content-Type': 'application/json' } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
      cache: 'no-store', credentials: 'omit', redirect: 'error', signal,
    });
  } catch (error) {
    if (signal.aborted) throw error;
    throw new ApiError('disconnected');
  }
  if (!response.ok) {
    const errorBody = /** @type {{error?: string}} */ (await response.json().catch(() => ({})));
    const code = errorBody.error;
    if (code === 'unauthorized' || code === 'invalid_request' || code === 'unavailable' || code === 'internal_error') {
      throw new ApiError(code);
    }
    throw new ApiError('internal_error');
  }
  return response.json();
}

/** @param {unknown} error */
function errorCode(error) {
  return error instanceof ApiError ? error.code : 'internal_error';
}

function pageActive() {
  return !document.hidden && pollTimer !== undefined;
}

function canInspect() {
  return pageActive() && !paused && connected && status !== null &&
    (status.state === 'idle' || status.state === 'running');
}

/**
 * Status has its own request lane, so a GIL-delayed inspect never blocks execution state.
 */
async function pollStatus() {
  if (!pageActive() || readingStatus || !token || serviceError === 'unauthorized') return;
  readingStatus = true;
  const controller = new AbortController();
  statusController = controller;
  try {
    const path = view === 'execution' && !paused ? '/api/status?output=1' : '/api/status';
    const next = /** @type {ObserverStatus} */ (await requestApi(path, controller.signal));
    if (controller.signal.aborted || !pageActive()) return;
    const previous = status;
    if (previous && (previous.workspaceId !== next.workspaceId || previous.sessionId !== next.sessionId)) {
      clearObservedData();
      inspectController?.abort();
    }
    if (previous && previous.state !== next.state && (next.state === 'exited' || next.state === 'not_started')) {
      clearObservedData();
      inspectController?.abort();
    }
    status = next;
    connected = true;
    serviceError = null;
    statusReceivedAt = Date.now();
    renderStatus();
    if (view === 'execution' && !paused) renderExecution();
    renderNotices();
  } catch (error) {
    if (controller.signal.aborted) return;
    connected = false;
    serviceError = errorCode(error);
    renderStatus();
    renderNotices();
  } finally {
    readingStatus = false;
    if (statusController === controller) statusController = null;
  }
}

/** Clear all observations at the worker boundary, including in-flight response eligibility. */
function clearObservedData() {
  workspaceEpoch += 1;
  viewRevision += 1;
  variables.data = null;
  variables.value = null;
  variables.selection = null;
  variables.offset = 0;
  variables.listSampledAt = 0;
  variables.valueSampledAt = 0;
  terminals.list = null;
  terminals.selectedId = null;
  clearTerminalData();
  clearExecutionView();
  valueMessage = '';
  terminalMessage = '';
  readMessage = '';
  setInspectionBusy(false);
  renderTerminals();
  renderVariables();
  renderValue();
  renderTerminal();
}

/** One inspect lane serves the sidebar and only the visible list / selected value or terminal. */
async function pollInspection() {
  if (!canInspect() || inspecting) return;
  const epoch = workspaceEpoch;
  const revision = viewRevision;
  const workspaceId = status?.workspaceId;
  const controller = new AbortController();
  inspectController = controller;
  inspecting = true;
  inspectStartedAt = Date.now();
  setInspectionBusy(true);
  /** @type {InspectRequest[]} */
  const requests = [{ view: 'terminals' }];
  if (view === 'variables') {
    requests.push({ view: 'variables', search: variables.search, definitions: variables.definitions, offset: variables.offset });
    if (variables.selection) {
      requests.push({ view: 'value', name: variables.selection.name, path: [...variables.selection.path], offset: variables.selection.offset });
    }
  } else if (view === 'terminal' && terminals.selectedId) {
    requests.push({ view: 'terminal', id: terminals.selectedId, mode: terminals.mode,
      ...(terminals.mode === 'raw' && terminals.raw.since !== undefined ? { since: terminals.raw.since } : {}),
    });
  }
  try {
    for (const request of requests) {
      if (!canInspect() || epoch !== workspaceEpoch || revision !== viewRevision) break;
      const result = /** @type {InspectResult} */ (await requestApi('/api/inspect', controller.signal, request));
      if (controller.signal.aborted || !canInspect() || epoch !== workspaceEpoch || revision !== viewRevision) break;
      // Status is authoritative: a delayed response must never restore a retired workspace.
      if (result.workspaceId !== workspaceId || result.workspaceId !== status?.workspaceId) {
        clearObservedData();
        readMessage = '工作区已重置';
        break;
      }
      if (result.status !== 'ok') {
        handleInspectState(result.status, request);
        break;
      }
      readMessage = '';
      acceptData(result.data, result.sampledAt);
    }
  } catch (error) {
    if (!controller.signal.aborted && epoch === workspaceEpoch && revision === viewRevision && !paused) {
      const code = errorCode(error);
      if (code === 'unauthorized' || code === 'disconnected') {
        connected = false;
        serviceError = code;
      } else {
        readMessage = API_ERROR_LABELS[code];
      }
    }
  } finally {
    inspecting = false;
    if (inspectController === controller) inspectController = null;
    setInspectionBusy(false);
    renderNotices();
    renderFreshness();
  }
}

/** @param {boolean} busy */
function setInspectionBusy(busy) {
  element('terminals-region').setAttribute('aria-busy', String(busy));
  element('variables-region').setAttribute('aria-busy', String(busy && view === 'variables'));
  element('value-region').setAttribute('aria-busy', String(busy && view === 'variables' && variables.selection !== null));
}

/** @param {Exclude<InspectResult['status'], 'ok'>} result @param {InspectRequest} request */
function handleInspectState(result, request) {
  switch (result) {
    case 'busy':
      readMessage = '等待读取…';
      break;
    case 'unavailable':
      readMessage = '数据不可用';
      break;
    case 'inspection_failed':
      readMessage = '读取失败';
      break;
    case 'invalid_request':
      readMessage = '请求无效';
      break;
    case 'not_found':
      if (request.view === 'value') {
        variables.value = null;
        variables.valueSampledAt = 0;
        valueMessage = '内容已不存在';
        renderValue();
      } else if (request.view === 'terminal') {
        terminalMessage = `${request.id} 已关闭`;
        terminals.selectedId = null;
        clearTerminalData();
        viewRevision += 1;
        renderTerminal();
        renderTerminals();
      } else {
        readMessage = '数据已不存在';
      }
      break;
  }
}

/** @param {InspectData} data @param {number} sampledAt */
function acceptData(data, sampledAt) {
  switch (data.view) {
    case 'variables':
      if (data.offset > 0 && data.offset >= data.total) {
        variables.offset = lastPageOffset(data);
        variables.data = null;
        variables.listSampledAt = 0;
        viewRevision += 1;
      } else {
        variables.data = data;
        variables.offset = data.offset;
        variables.listSampledAt = sampledAt;
      }
      renderVariables();
      break;
    case 'value':
      if (variables.selection && data.offset > 0 && data.offset >= data.total) {
        variables.selection.offset = lastPageOffset(data);
        variables.value = null;
        variables.valueSampledAt = 0;
        viewRevision += 1;
      } else {
        variables.value = data;
        if (variables.selection) variables.selection.offset = data.offset;
        variables.valueSampledAt = sampledAt;
      }
      valueMessage = '';
      renderValue();
      break;
    case 'terminals': {
      terminals.list = data.terminals.filter(terminal => !terminal.closed);
      if (terminals.selectedId && !terminals.list.some(terminal => terminal.id === terminals.selectedId)) {
        terminalMessage = `${terminals.selectedId} 已关闭`;
        terminals.selectedId = null;
        clearTerminalData();
        viewRevision += 1;
      }
      renderTerminals();
      if (view === 'terminal') renderTerminal();
      break;
    }
    case 'terminal':
      terminals.sampledAt = sampledAt;
      if (data.mode === 'screen') terminals.screen = data;
      else appendRaw(data);
      renderTerminal();
      break;
  }
}

/** Live containers can shrink while viewing a later page. @param {{total: number, pageSize: number}} data */
function lastPageOffset(data) {
  return Math.max(0, Math.floor((data.total - 1) / data.pageSize) * data.pageSize);
}

/** @param {RawData} data */
function appendRaw(data) {
  const raw = terminals.raw;
  if (raw.since === undefined) raw.initialTail = data.raw.start > 0 || data.raw.truncated;
  raw.text += data.raw.text;
  raw.since = data.raw.end;
  raw.lostBytes += data.raw.lostBytes;
  raw.droppedBytes = data.raw.droppedBytes;
  const bytes = encoder.encode(raw.text);
  if (bytes.length > RAW_LIMIT_BYTES) {
    let start = bytes.length - RAW_LIMIT_BYTES;
    // Keep a whole UTF-8 code point when cutting the retained tail.
    while (start < bytes.length && (bytes[start] & 0xc0) === 0x80) start += 1;
    raw.trimmedBytes += start;
    raw.text = decoder.decode(bytes.subarray(start));
  }
}

/** @param {string} id @param {string} text */
function setText(id, text) {
  const target = element(id);
  if (target.textContent !== text) target.textContent = text;
}

/** @param {string} id @param {string} text */
function notice(id, text) {
  setText(id, text);
  element(id).hidden = !text;
}

function workspaceEmptyText() {
  if (!token) return '缺少访问凭证';
  if (!status) return '连接中…';
  if (status.state === 'not_started') return 'Python 未启动';
  if (status.state === 'starting') return 'Python 启动中';
  if (status.state === 'stopping') return 'Python 退出中';
  if (status.state === 'exited') return 'Python 已退出';
  return '';
}

function renderStatus() {
  const stateText = connected && status ? STATE_LABELS[status.state] : token ? '服务未连接' : '缺少访问凭证';
  setText('python-version', status?.version ? `Python ${status.version}` : 'Python');
  setText('python-state', stateText);
  element('python-state').dataset.state = connected && status ? status.state : 'disconnected';
  setText('python-pid', status?.pid ? `PID ${status.pid}` : '');
  element('python-pid').hidden = !status?.pid;
  setText('workspace-cwd', status?.cwd ?? '—');
  setText('terminal-count', String(terminals.list?.length ?? status?.terminalCount ?? '—'));
  if (!variables.data) setText('variables-empty', workspaceEmptyText() || '加载中…');
  if (!terminals.list) setText('terminals-empty', workspaceEmptyText() || '加载中…');
  renderFreshness();
}

function renderNotices() {
  const serviceMessage = !token ? '缺少访问凭证' : serviceError ? API_ERROR_LABELS[serviceError] : '';
  notice('service-notice', serviceMessage);
  let message = paused ? '' : readMessage;
  if (!paused && inspecting && Date.now() - inspectStartedAt >= 1500 && !message) message = '等待数据…';
  notice('read-notice', serviceMessage ? '' : message);
  ui.pause.setAttribute('aria-pressed', String(paused));
}

/** @param {number} timestamp */
function age(timestamp) {
  const elapsed = Math.max(0, Date.now() - timestamp);
  return elapsed < 1000 ? '刚刚' : `${Math.floor(elapsed / 1000)} 秒前`;
}

function renderFreshness() {
  let sampledAt = 0;
  if (view === 'variables') {
    sampledAt = variables.listSampledAt;
    if (variables.selection) {
      const timestamps = [variables.listSampledAt, variables.valueSampledAt].filter(timestamp => timestamp > 0);
      if (timestamps.length) sampledAt = Math.min(...timestamps);
    }
  } else if (view === 'terminal') sampledAt = terminals.sampledAt;
  else sampledAt = executionSampledAt;
  setText('data-age', paused ? '已暂停' : sampledAt ? `${age(sampledAt)}更新` : '');
  if (view === 'execution' && !paused && connected) renderExecutionTime();
}

/** @param {TerminalInfo} terminal */
function terminalState(terminal) {
  if (terminal.closed) return '已关闭';
  if (terminal.status.kind === 'running') return '运行中';
  return `已退出 · code ${terminal.status.code}${terminal.status.signal ? ` · 信号 ${terminal.status.signal}` : ''}`;
}

/** @param {TerminalInfo} terminal */
function terminalCommand(terminal) {
  return terminal.command?.length ? escapeControls(terminal.command.join(' ')).replace(/\s+/gu, ' ') : '';
}

/**
 * Preserve focus on a still-present control when live data changes, without treating labels as selectors.
 * @param {HTMLElement} container
 * @param {Node[]} nodes
 */
function replaceContent(container, nodes) {
  const active = document.activeElement;
  const key = active instanceof HTMLElement && container.contains(active) ? active.dataset.focusKey : undefined;
  container.replaceChildren(...nodes);
  if (key) {
    const target = Array.from(container.querySelectorAll('button')).find(button => button.dataset.focusKey === key);
    target?.focus({ preventScroll: true });
  }
}

/** @param {string} text @param {() => void} action @param {string} [className] */
function button(text, action, className = '') {
  const control = document.createElement('button');
  control.type = 'button';
  control.textContent = text;
  control.className = className;
  control.addEventListener('click', action);
  return control;
}

/** @param {string} text @param {string} [className] */
function span(text, className = '') {
  const node = document.createElement('span');
  node.textContent = text;
  node.className = className;
  return node;
}

let terminalListSignature = '';
function renderTerminals() {
  element('terminals-empty').hidden = !!terminals.list?.length;
  setText('terminals-empty', terminals.list ? '暂无终端' : workspaceEmptyText() || '加载中…');
  setText('terminal-count', String(terminals.list?.length ?? status?.terminalCount ?? '—'));
  const signature = JSON.stringify([terminals.list, terminals.selectedId, view]);
  if (signature === terminalListSignature) return;
  terminalListSignature = signature;
  const rows = (terminals.list ?? []).map(terminal => {
    const row = document.createElement('li');
    const control = button('', () => selectTerminal(terminal.id), 'side-item');
    control.setAttribute('aria-label', `查看终端 ${terminal.id}`);
    control.title = terminalCommand(terminal);
    control.setAttribute('aria-pressed', String(view === 'terminal' && terminals.selectedId === terminal.id));
    control.dataset.focusKey = terminal.id;
    const state = span(terminalState(terminal), 'side-state');
    state.dataset.state = terminal.status.kind;
    control.append(span([terminal.id, terminalCommand(terminal)].filter(Boolean).join(' · '), 'side-title'), state,
      span(`PID ${terminal.pid} · ${terminal.cols} × ${terminal.rows}`, 'mono muted small'));
    if (terminal.hasError) control.append(span('终端读取异常', 'warning small'));
    row.append(control);
    return row;
  });
  replaceContent(ui.terminalList, rows);
}

let variableListSignature = '';
function renderVariables() {
  const data = variables.data;
  element('variables-empty').hidden = !!data?.variables.length;
  setText('variables-empty', data ? (variables.search ? '无匹配变量' : '暂无变量') : workspaceEmptyText() || '加载中…');
  renderPagination('variables', data, variables.offset);
  const signature = JSON.stringify([data, variables.selection?.name]);
  if (signature === variableListSignature) return;
  variableListSignature = signature;
  const rows = (data?.variables ?? []).map(variable => {
    const row = document.createElement('tr');
    const selected = variables.selection?.name === variable.name;
    row.dataset.selected = String(selected);
    const nameCell = document.createElement('th');
    nameCell.scope = 'row';
    const control = button(variable.name, () => selectVariable(variable.name));
    control.setAttribute('aria-label', `查看变量 ${variable.name}`);
    control.setAttribute('aria-expanded', String(selected));
    control.setAttribute('aria-controls', 'value-content');
    control.dataset.focusKey = variable.name;
    nameCell.append(control);
    const typeCell = document.createElement('td');
    typeCell.append(span(variable.type));
    typeCell.title = variable.type;
    const summaryCell = document.createElement('td');
    summaryCell.append(span(variable.summary));
    summaryCell.title = variable.summary;
    row.append(nameCell, typeCell, summaryCell);
    return row;
  });
  replaceContent(ui.variableRows, rows);
}

/** @param {'variables' | 'children'} prefix @param {{offset: number, total: number, pageSize: number} | null} data @param {number} offset */
function renderPagination(prefix, data, offset) {
  const previous = /** @type {HTMLButtonElement} */ (element(`${prefix}-prev`));
  const next = /** @type {HTMLButtonElement} */ (element(`${prefix}-next`));
  previous.disabled = !data || offset <= 0;
  next.disabled = !data || offset + data.pageSize >= data.total;
  const range = data ? data.total === 0 ? '0' : `${offset + 1}–${Math.min(offset + data.pageSize, data.total)} / ${data.total}` : '';
  setText(`${prefix}-range`, range);
}

let valueSignature = '';
function renderValue() {
  const selection = variables.selection;
  element('close-value').hidden = !selection;
  element('value-empty').hidden = !selection || !!variables.value;
  setText('value-empty', selection ? valueMessage || '加载中…' : '');
  element('value-content').hidden = !selection;
  if (!selection) {
    ui.breadcrumbs.replaceChildren();
    ui.children.replaceChildren();
    setText('value-summary', '');
    setText('value-type', '');
    element('value-terminal-link').replaceChildren();
    valueSignature = '';
    return;
  }
  const signature = JSON.stringify([selection, variables.value]);
  if (signature === valueSignature) return;
  valueSignature = signature;
  const breadcrumbs = [button(selection.name, () => navigateValue(0))];
  for (let index = 0; index < selection.path.length; index += 1) {
    const selector = selection.path[index];
    const label = selector.kind === 'index' ? `[${selector.index}]` : `当前项 #${selector.index + 1}`;
    breadcrumbs.push(button(label, () => navigateValue(index + 1)));
  }
  breadcrumbs.forEach((control, index) => {
    control.dataset.focusKey = String(index);
    if (index === selection.path.length) control.setAttribute('aria-current', 'location');
  });
  replaceContent(ui.breadcrumbs, breadcrumbs);
  const data = variables.value;
  const sizeUnit = data?.value.type === 'str' ? '字符' : data?.value.type === 'bytes' ? '字节' : '项';
  setText('value-type', data ? `${data.value.type}${data.value.size !== undefined ? ` · ${data.value.size} ${sizeUnit}` : ''}` : '');
  setText('value-summary', data?.value.summary ?? '');
  const containerSize = data?.value.size !== undefined && ['list', 'tuple', 'dict', 'set', 'frozenset'].includes(data.value.type);
  element('value-summary').hidden = !data?.value.summary || containerSize;
  renderValueTerminalLink(data?.value);
  element('children-pagination').hidden = !data || data.total === 0;
  renderPagination('children', data, selection.offset);
  const rows = (data?.children ?? []).map(child => {
    const row = document.createElement('li');
    row.className = 'child-item';
    if (child.expandable) {
      const control = button(child.label, () => enterChild(child.selector));
      control.setAttribute('aria-label', `查看子项 ${child.label}`);
      control.dataset.focusKey = `${child.selector.kind}:${child.selector.index}`;
      row.append(control);
    } else row.append(span(child.label, 'child-label'));
    const description = document.createElement('div');
    description.className = 'child-summary';
    description.append(span(child.type, 'purple'), span(child.summary));
    if (child.terminalId) description.append(button(`查看终端 ${child.terminalId}`, () => selectTerminal(child.terminalId ?? ''), 'jump-link'));
    row.append(description);
    return row;
  });
  replaceContent(ui.children, rows);
}

/** @param {ValueSummary | undefined} value */
function renderValueTerminalLink(value) {
  const container = element('value-terminal-link');
  if (value?.terminalId) {
    const id = value.terminalId;
    const control = button(`查看关联终端 ${id}`, () => selectTerminal(id), 'jump-link');
    control.dataset.focusKey = id;
    replaceContent(container, [control]);
  } else container.replaceChildren();
}

function clearExecutionView() {
  executionSampledAt = 0;
  element('execution-empty').hidden = false;
  element('execution-content').hidden = true;
  setText('execution-state', '尚无观测数据');
  setText('execution-number', '');
  setText('execution-duration', '');
  setText('execution-timeout', '');
  setText('execution-code', '');
  setText('execution-output', '');
  setText('execution-cwd', '');
  notice('execution-output-state', '');
}

function renderExecution() {
  if (paused) {
    setText('execution-empty', '');
    return;
  }
  executionSampledAt = statusReceivedAt;
  setText('execution-empty', '暂无调用');
  const execution = status?.execution;
  element('execution-empty').hidden = !!execution;
  element('execution-content').hidden = !execution;
  if (!execution) {
    setText('execution-state', '尚无执行记录');
    setText('execution-number', '');
    setText('execution-timeout', '');
    setText('execution-code', '');
    setText('execution-output', '');
    renderExecutionTime();
    return;
  }
  setText('execution-state', execution.outcome ? OUTCOME_LABELS[execution.outcome] ?? '调用已结束（未识别结果）' : '正在执行');
  setText('execution-number', `第 ${execution.number} 次调用`);
  setText('execution-timeout', `超时上限 ${execution.timeoutSeconds} 秒`);
  setText('execution-code', execution.code);
  setText('execution-cwd', execution.cwd);
  const changed = ui.output.textContent !== execution.output;
  setText('execution-output', execution.output);
  if (changed && ui.followOutput.checked) ui.output.scrollTop = ui.output.scrollHeight;
  notice('execution-output-state', execution.outputUnavailable ? '读取失败' : execution.outputTruncated ? '已截断' : '');
  renderExecutionTime();
}

function renderExecutionTime() {
  const execution = status?.execution;
  if (!execution) {
    setText('execution-duration', '');
    return;
  }
  const end = execution.finishedAt ?? Date.now();
  const seconds = Math.max(0, end - execution.startedAt) / 1000;
  setText('execution-duration', `${execution.finishedAt === undefined ? '已运行' : '耗时'} ${seconds.toFixed(1)} 秒`);
}

let terminalMetadataSignature = '';
function renderTerminal() {
  const terminal = terminals.list?.find(item => item.id === terminals.selectedId);
  element('terminal-empty').hidden = !!terminal;
  element('terminal-content').hidden = !terminal;
  if (!terminal) {
    notice('terminal-empty', terminalMessage || (terminals.selectedId ? '加载中…' : ''));
    ui.screenGrid.replaceChildren();
    ui.rawOutput.textContent = '';
    setText('screen-accessible-text', '');
    return;
  }
  setText('terminal-title', [terminal.id, terminalCommand(terminal)].filter(Boolean).join(' · '));
  element('terminal-title').title = terminalCommand(terminal);
  setText('terminal-state', terminalState(terminal));
  setText('terminal-cwd', terminal.cwd ?? '');
  element('terminal-cwd').hidden = !terminal.cwd;
  setText('terminal-meta', `PID ${terminal.pid} · ${terminal.cols} 列 × ${terminal.rows} 行`);
  const signature = JSON.stringify([terminal.id, terminal.variables]);
  if (signature !== terminalMetadataSignature) {
    terminalMetadataSignature = signature;
    const links = terminal.variables.map(name => button(`变量 ${name}`, () => selectVariable(name, true), 'jump-link'));
    element('terminal-variable-links').replaceChildren(...links);
  }
  element('mode-screen').setAttribute('aria-pressed', String(terminals.mode === 'screen'));
  element('mode-raw').setAttribute('aria-pressed', String(terminals.mode === 'raw'));
  ui.screenScroll.hidden = terminals.mode !== 'screen';
  ui.rawOutput.hidden = terminals.mode !== 'raw';
  element('raw-gaps').hidden = terminals.mode !== 'raw';
  if (terminals.mode === 'screen') renderScreen();
  else renderRaw();
}

const XTERM_BASE = [
  '#000000', '#800000', '#008000', '#808000', '#000080', '#800080', '#008080', '#c0c0c0',
  '#808080', '#ff0000', '#00ff00', '#ffff00', '#0000ff', '#ff00ff', '#00ffff', '#ffffff',
];

/** @param {CellColor} color @param {'foreground' | 'background'} defaultRole */
function cellColor(color, defaultRole) {
  if (color.kind === 'default') return defaultRole === 'foreground' ? 'var(--text)' : 'var(--base)';
  if (color.kind === 'rgb') return `rgb(${color.red}, ${color.green}, ${color.blue})`;
  if (color.index < 16) return XTERM_BASE[color.index];
  if (color.index >= 232) {
    const gray = 8 + (color.index - 232) * 10;
    return `rgb(${gray}, ${gray}, ${gray})`;
  }
  const index = color.index - 16;
  const levels = [0, 95, 135, 175, 215, 255];
  return `rgb(${levels[Math.floor(index / 36)]}, ${levels[Math.floor(index / 6) % 6]}, ${levels[index % 6]})`;
}

let screenSignature = '';
function renderScreen() {
  const screen = terminals.screen?.screen;
  if (!screen) {
    ui.screenGrid.replaceChildren();
    setText('screen-accessible-text', '');
    notice('terminal-data-state', '加载中…');
    return;
  }
  const rect = screen.rect.effective;
  const cropped = rect.width < screen.full_size.cols || rect.height < screen.full_size.rows || rect.x !== 0 || rect.y !== 0;
  const cursorX = screen.cursor.x - rect.x;
  const cursorY = screen.cursor.y - rect.y;
  notice('terminal-data-state', [
    screen.hasError ? '终端读取失败' : '',
    cropped ? `已裁剪 ${rect.width} × ${rect.height}` : '',
  ].filter(Boolean).join(' · '));
  const signature = JSON.stringify(screen);
  if (signature === screenSignature) return;
  screenSignature = signature;
  const fragment = document.createDocumentFragment();
  screen.cells.forEach(cells => {
    const row = document.createElement('div');
    row.className = 'screen-row';
    row.style.width = `${rect.width}ch`;
    cells.forEach((cell, column) => {
      if (cell.wide_continuation) return;
      const node = document.createElement('span');
      node.className = 'screen-cell';
      node.style.left = `${column}ch`;
      node.style.width = cell.wide ? '2ch' : '1ch';
      const foreground = cellColor(cell.foreground, 'foreground');
      const background = cellColor(cell.background, 'background');
      node.style.color = cell.inverse ? background : foreground;
      node.style.backgroundColor = cell.inverse ? foreground : background;
      const text = span(cell.text || ' ', 'screen-cell-text');
      if (cell.bold) text.style.fontWeight = '700';
      if (cell.dim) text.style.opacity = '0.65';
      if (cell.italic) text.style.fontStyle = 'italic';
      if (cell.underline) text.style.textDecoration = 'underline';
      node.append(text);
      row.append(node);
    });
    fragment.append(row);
  });
  if (screen.cursor.visible && cursorX >= 0 && cursorY >= 0 && cursorX < rect.width && cursorY < rect.height) {
    const cursor = document.createElement('span');
    cursor.className = 'screen-cursor';
    cursor.style.left = `${cursorX}ch`;
    cursor.style.top = `${cursorY * 1.55}em`;
    fragment.append(cursor);
  }
  ui.screenGrid.style.width = `${rect.width}ch`;
  ui.screenGrid.style.height = `${rect.height * 1.55}em`;
  ui.screenGrid.replaceChildren(fragment);
  setText('screen-accessible-text', screen.lines.join('\n'));
}

/** @param {string} text */
function escapeControls(text) {
  return text.replace(/\p{Cc}/gu, character => {
    if (character === '\n' || character === '\t') return character;
    if (character === '\r') return '\\r';
    return `\\x${character.codePointAt(0)?.toString(16).padStart(2, '0')}`;
  });
}

function renderRaw() {
  const raw = terminals.raw;
  const text = escapeControls(raw.text);
  if (ui.rawOutput.textContent !== text) ui.rawOutput.textContent = text;
  notice('terminal-data-state', raw.since === undefined ? '加载中…' : '');
  const gaps = [];
  if (raw.initialTail || raw.droppedBytes || raw.trimmedBytes) gaps.push('已截断');
  if (raw.lostBytes) gaps.push(`缺失 ${raw.lostBytes} 字节`);
  notice('raw-gaps', gaps.join(' · '));
}

/** @param {View} next */
function changeView(next) {
  if (view !== next) {
    viewRevision += 1;
    if (view === 'variables') {
      variables.data = null;
      variables.value = null;
      variables.listSampledAt = 0;
      variables.valueSampledAt = 0;
      renderVariables();
      renderValue();
    }
    if (view === 'terminal') clearTerminalData();
    if (view === 'execution') clearExecutionView();
    view = next;
    readMessage = '';
  }
  for (const tab of /** @type {View[]} */ (['variables', 'execution', 'terminal'])) {
    const selected = tab === view;
    const control = element(`tab-${tab}`);
    control.setAttribute('aria-selected', String(selected));
    control.tabIndex = selected ? 0 : -1;
    element(`${tab}-view`).hidden = !selected;
  }
  element('python-side').setAttribute('aria-pressed', String(view !== 'terminal'));
  renderTerminals();
  if (view === 'execution') renderExecution();
  if (view === 'terminal') renderTerminal();
  renderNotices();
  renderFreshness();
}

/** @param {string} name @param {boolean} [fromTerminal] */
function selectVariable(name, fromTerminal = false) {
  if (fromTerminal) {
    variables.search = '';
    variables.offset = 0;
    ui.search.value = '';
  }
  variables.selection = { name, path: [], offset: 0 };
  variables.value = null;
  variables.valueSampledAt = 0;
  valueMessage = '';
  viewRevision += 1;
  changeView('variables');
  renderVariables();
  renderValue();
  renderFreshness();
}

/** @param {ValuePath[number]} selector */
function enterChild(selector) {
  if (!variables.selection) return;
  variables.selection.path.push(selector);
  variables.selection.offset = 0;
  valueSelectionChanged();
}

/** @param {number} depth */
function navigateValue(depth) {
  if (!variables.selection) return;
  variables.selection.path = variables.selection.path.slice(0, depth);
  variables.selection.offset = 0;
  valueSelectionChanged();
}

function valueSelectionChanged() {
  variables.value = null;
  variables.valueSampledAt = 0;
  valueMessage = '';
  viewRevision += 1;
  renderValue();
  renderFreshness();
}

/** @param {string} id */
function selectTerminal(id) {
  terminals.selectedId = id;
  terminalMessage = '';
  viewRevision += 1;
  clearTerminalData();
  changeView('terminal');
  renderTerminal();
}

function clearTerminalData() {
  terminals.screen = null;
  terminals.raw = newRawBuffer();
  terminals.sampledAt = 0;
  screenSignature = '';
  ui.screenGrid.replaceChildren();
  ui.rawOutput.textContent = '';
  setText('screen-accessible-text', '');
}

/** @param {'screen' | 'raw'} mode */
function changeTerminalMode(mode) {
  if (mode === terminals.mode) return;
  terminals.mode = mode;
  viewRevision += 1;
  clearTerminalData();
  renderTerminal();
  renderFreshness();
}

/** @param {'variables' | 'children'} list @param {number} direction */
function changePage(list, direction) {
  const data = list === 'variables' ? variables.data : variables.value;
  if (!data) return;
  const offset = Math.max(0, data.offset + direction * data.pageSize);
  if (list === 'variables') {
    variables.offset = offset;
    variables.data = null;
    variables.listSampledAt = 0;
    renderVariables();
  } else if (variables.selection) {
    variables.selection.offset = offset;
    valueSelectionChanged();
  }
  viewRevision += 1;
  renderFreshness();
}

function changeVariableFilter() {
  variables.search = ui.search.value;
  variables.definitions = ui.definitions.checked;
  variables.offset = 0;
  variables.data = null;
  variables.listSampledAt = 0;
  viewRevision += 1;
  renderVariables();
  renderFreshness();
}

for (const tab of /** @type {View[]} */ (['variables', 'execution', 'terminal'])) {
  element(`tab-${tab}`).addEventListener('click', () => changeView(tab));
  element(`tab-${tab}`).addEventListener('keydown', event => {
    const order = /** @type {View[]} */ (['variables', 'execution', 'terminal']);
    let index = order.indexOf(tab);
    if (event.key === 'ArrowRight') index = (index + 1) % order.length;
    else if (event.key === 'ArrowLeft') index = (index + order.length - 1) % order.length;
    else if (event.key === 'Home') index = 0;
    else if (event.key === 'End') index = order.length - 1;
    else return;
    event.preventDefault();
    changeView(order[index]);
    element(`tab-${order[index]}`).focus();
  });
}
ui.pause.addEventListener('click', () => {
  paused = !paused;
  viewRevision += 1;
  if (view === 'execution') renderExecution();
  renderNotices();
  renderFreshness();
});
element('python-side').addEventListener('click', () => changeView('variables'));
ui.search.addEventListener('input', changeVariableFilter);
ui.definitions.addEventListener('change', changeVariableFilter);
element('close-value').addEventListener('click', () => {
  variables.selection = null;
  valueSelectionChanged();
  renderVariables();
});
element('mode-screen').addEventListener('click', () => changeTerminalMode('screen'));
element('mode-raw').addEventListener('click', () => changeTerminalMode('raw'));
for (const list of /** @type {Array<'variables' | 'children'>} */ (['variables', 'children'])) {
  element(`${list}-prev`).addEventListener('click', () => changePage(list, -1));
  element(`${list}-next`).addEventListener('click', () => changePage(list, 1));
}
ui.followOutput.addEventListener('change', () => {
  if (ui.followOutput.checked) ui.output.scrollTop = ui.output.scrollHeight;
});

function tick() {
  void pollStatus();
  void pollInspection();
  renderFreshness();
  renderNotices();
}

function startPolling() {
  if (document.hidden || pollTimer !== undefined) return;
  pollTimer = window.setInterval(tick, REFRESH_MS);
  tick();
}

function stopPolling() {
  if (pollTimer !== undefined) window.clearInterval(pollTimer);
  pollTimer = undefined;
  statusController?.abort();
  inspectController?.abort();
}

document.addEventListener('visibilitychange', () => {
  if (document.hidden) stopPolling();
  else startPolling();
});
window.addEventListener('pagehide', stopPolling);
window.addEventListener('pageshow', startPolling);
window.addEventListener('hashchange', () => {
  stopPolling();
  token = readToken();
  status = null;
  connected = false;
  serviceError = null;
  statusReceivedAt = 0;
  clearObservedData();
  renderExecution();
  renderStatus();
  renderNotices();
  startPolling();
});

renderStatus();
renderVariables();
renderValue();
renderTerminals();
renderTerminal();
renderNotices();
startPolling();
