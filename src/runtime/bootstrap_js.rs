/// Standard `globalThis.__NG_NATIVE__` + Web/Host globals (`console`, `queueMicrotask`, `setTimeout`, `fetch`)
/// injected into every JS engine context.
/// `@ng-desktop/platform` (`NativeRenderer2`), `@ng-desktop/forms`, `@ng-desktop/http`, and
/// `@ng-desktop/desktop` call this object to queue frame-batched scene/desktop commands,
/// configure OS-native HTTP transports, and register live JS event listener closures.
pub const NG_NATIVE_BOOTSTRAP_JS: &str = r#"
globalThis.__NG_NATIVE__ = (() => {
  let queue = [];
  let batchDepth = 0;
  let batchBuffer = [];
  const emit = (cmd) => {
    if (batchDepth > 0) batchBuffer.push(cmd);
    else queue.push(cmd);
  };
  let logs = [];
  const resolveAppId = () => {
    const raw = globalThis.__NG_NATIVE_APP_ID__;
    return typeof raw === 'string' && raw.trim() ? raw.trim() : 'ng-desktop-app';
  };
  let timers = new Map();
  let nextTimerId = 1;
  let rafQueue = new Map();
  let nextRafId = 1;
  const startTime = Date.now();
  const listenersByNode = new Map();
  let nextListenerId = 1;
  let hostCallbacks = new Map();
  let nextCallbackId = 1;
  let systemTheme = 'dark';
  let clipboardText = '';
  let networkStatus = { online: true, metered: false, connectivity: 'full' };
  let systemLocale = 'en-US';
  let systemTimezone = 'UTC';
  let systemErrors = [];
  let sqliteTables = new Map();
  let fsFiles = new Map();
  let fsWatched = new Set();
  let fsChanged = new Set();
  let windowGeometry = { width: 1020, height: 720, x: null, y: null, maximized: false };
  let appMenuItems = [];
  let trayState = { visible: false, icon: '', tooltip: '', menuItems: [] };
  let shortcuts = [];
  let httpEndpoints = new Map();
  let httpConfig = {
    platform: 'auto',
    useFetchTransport: true,
    interceptorsEnabled: true,
    xsrfEnabled: true,
    xsrfCookieName: 'XSRF-TOKEN',
    xsrfHeaderName: 'X-XSRF-TOKEN',
    xsrfTokenValue: '',
    jsonpEnabled: false,
    requestsViaParent: false,
    maxParallelConnections: 24,
    cacheEnabled: true,
    timeoutMs: 15000,
  };

  const escapeNewlines = (value) => String(value ?? '').replace(/\r?\n/g, '\u001e');
  const hasHostService = () => typeof globalThis.__NG_NATIVE_HOST_SERVICE__ === 'function';
  const callHostService = (payload) => {
    if (!hasHostService()) return null;
    try {
      const raw = globalThis.__NG_NATIVE_HOST_SERVICE__(JSON.stringify(payload));
      return raw ? JSON.parse(raw) : null;
    } catch (_err) {
      return null;
    }
  };
  const registerHostCallback = (resolve, reject) => {
    const id = nextCallbackId++;
    hostCallbacks.set(id, { resolve, reject });
    return id;
  };

  const pushLog = (level, msg) => {
    if (logs.length >= 500) logs.shift();
    logs.push([level, msg]);
  };
  const pushSystemError = (msg) => {
    if (systemErrors.length >= 200) systemErrors.shift();
    systemErrors.push(msg);
  };

  globalThis.console = {
    log: (...args) => pushLog('LOG', args.join(' ')),
    info: (...args) => pushLog('INFO', args.join(' ')),
    warn: (...args) => pushLog('WARN', args.join(' ')),
    debug: (...args) => pushLog('DEBUG', args.join(' ')),
    error: (...args) => {
      const msg = args.join(' ');
      pushLog('ERROR', msg);
      pushSystemError(msg);
      callHostService({ action: 'logError', message: msg });
    },
  };

  if (typeof globalThis.queueMicrotask !== 'function') {
    globalThis.queueMicrotask = (cb) => Promise.resolve().then(cb);
  }

  if (typeof globalThis.performance === 'undefined') {
    globalThis.performance = {
      now: () => Date.now() - startTime,
      timeOrigin: startTime,
    };
  } else if (typeof globalThis.performance.now !== 'function') {
    globalThis.performance.now = () => Date.now() - startTime;
  }

  globalThis.setTimeout = (cb, ms = 0, ...args) => {
    const id = nextTimerId++;
    const delay = Math.max(0, Number(ms) || 0);
    timers.set(id, { id, cb, args, dueAt: Date.now() + delay, interval: 0 });
    return id;
  };

  globalThis.clearTimeout = (id) => {
    timers.delete(id);
  };

  globalThis.setInterval = (cb, ms = 0, ...args) => {
    const id = nextTimerId++;
    const delay = Math.max(1, Number(ms) || 0);
    timers.set(id, { id, cb, args, dueAt: Date.now() + delay, interval: delay });
    return id;
  };

  globalThis.clearInterval = (id) => {
    timers.delete(id);
  };

  globalThis.requestAnimationFrame = (cb) => {
    const id = nextRafId++;
    rafQueue.set(id, cb);
    return id;
  };

  globalThis.cancelAnimationFrame = (id) => {
    rafQueue.delete(id);
  };

  const bridge = {
    registerStyle(cls, bg = 'none', fg = 'none', pad = 0, space = 0, extras = '') {
      const extraTrimmed = String(extras ?? '').trim();
      if (extraTrimmed) {
        emit(`style ${cls} ${bg} ${fg} ${pad} ${space} ${extraTrimmed}`);
      } else {
        emit(`style ${cls} ${bg} ${fg} ${pad} ${space}`);
      }
    },
    declareSignal(name, initial = 0) {
      emit(`signal ${name} ${initial}`);
    },
    createElement(id, tag) {
      emit(`create ${id} ${tag}`);
      return id;
    },
    createComment(id, text = 'anchor') {
      emit(`comment ${id} ${text}`);
      return id;
    },
    setText(id, text) {
      emit(`text ${id} ${escapeNewlines(text)}`);
    },
    setAttribute(id, key, value) {
      emit(`attr ${id} ${key} ${escapeNewlines(value)}`);
    },
    setWindowIcon(id, appId, icoPath) {
      emit(`icon ${id} ${appId} ${icoPath}`);
    },
    appendChild(parent, child) {
      emit(`append ${parent} ${child}`);
    },
    insertBefore(parent, child, refChild) {
      if (refChild) {
        emit(`insert_before ${parent} ${child} ${refChild}`);
      } else {
        emit(`insert_before ${parent} ${child}`);
      }
    },
    removeChild(parent, child) {
      emit(`remove_child ${parent} ${child}`);
    },
    addClass(id, className) {
      emit(`add_class ${id} ${className}`);
    },
    removeClass(id, className) {
      emit(`remove_class ${id} ${className}`);
    },
    bindText(id, signalName, template) {
      emit(`bind_text ${id} ${signalName} ${template}`);
    },
    bindAttr(id, attrKey, signalName, template = '{}') {
      emit(`bind_attr ${id} ${attrKey} ${signalName} ${template}`);
    },
    bindVisible(id, signalName, op = '!=', rhs = 0) {
      emit(`bind_visible ${id} ${signalName} ${op} ${rhs}`);
    },
    onAdd(id, eventName, signalName, delta = 1) {
      emit(`on_add ${id} ${eventName} ${signalName} ${delta}`);
    },
    onSet(id, eventName, signalName, value = 0) {
      emit(`on_set ${id} ${eventName} ${signalName} ${value}`);
    },
    listen(id, eventName, callback) {
      const lid = nextListenerId++;
      let byEvent = listenersByNode.get(id);
      if (!byEvent) {
        byEvent = new Map();
        listenersByNode.set(id, byEvent);
      }
      let callbacks = byEvent.get(eventName);
      if (!callbacks) {
        callbacks = new Map();
        byEvent.set(eventName, callbacks);
      }
      callbacks.set(lid, callback);
      return () => {
        callbacks.delete(lid);
        if (callbacks.size === 0) {
          byEvent.delete(eventName);
          if (byEvent.size === 0) {
            listenersByNode.delete(id);
          }
        }
      };
    },
    __dispatchEvent(id, eventName, eventPayload = {}) {
      const callbacks = listenersByNode.get(id)?.get(eventName);
      if (!callbacks || callbacks.size === 0) return;
      for (const callback of Array.from(callbacks.values())) {
        callback(eventPayload);
      }
    },
    __dispatchAndDrain(id, eventName, eventPayload = {}) {
      bridge.__dispatchEvent(id, eventName, eventPayload);
      bridge.__flushTimers();
      return bridge.__drainQueue();
    },
    __resolveHostCallback(id, payload) {
      const pending = hostCallbacks.get(id);
      if (!pending) return;
      hostCallbacks.delete(id);
      pending.resolve(payload);
    },
    __tickAndDrain(nowMs = Date.now(), completions = []) {
      if (Array.isArray(completions)) {
        for (const pair of completions) {
          if (Array.isArray(pair) && pair.length >= 2) {
            bridge.__resolveHostCallback(pair[0], pair[1]);
          }
        }
      }
      if (timers.size > 0 || rafQueue.size > 0) {
        bridge.__tickTimers(nowMs);
      }
      return bridge.__drainQueue();
    },
    beginBatch() {
      batchDepth += 1;
    },
    endBatch() {
      if (batchDepth > 0) {
        batchDepth -= 1;
      }
      if (batchDepth === 0 && batchBuffer.length > 0) {
        queue.push(...batchBuffer);
        batchBuffer.length = 0;
      }
    },
    __tickTimers(nowMs = Date.now()) {
      const now = typeof nowMs === 'number' && Number.isFinite(nowMs) ? nowMs : Date.now();
      const due = [];
      for (const entry of timers.values()) {
        if (entry.dueAt <= now) {
          due.push(entry);
          if (entry.interval > 0) {
            entry.dueAt = now + entry.interval;
          } else {
            timers.delete(entry.id);
          }
        }
      }
      for (const entry of due) {
        entry.cb(...(entry.args || []));
      }
      if (rafQueue.size > 0) {
        const frames = Array.from(rafQueue.values());
        rafQueue.clear();
        for (const frameCb of frames) {
          frameCb(now - startTime);
        }
      }
    },
    __flushTimers() {
      for (let pass = 0; pass < 16; pass++) {
        const oneShots = Array.from(timers.values()).filter((e) => e.interval === 0);
        const frames = Array.from(rafQueue.values());
        if (oneShots.length === 0 && frames.length === 0) break;
        for (const entry of oneShots) {
          timers.delete(entry.id);
          entry.cb(...(entry.args || []));
        }
        rafQueue.clear();
        const nowRel = Date.now() - startTime;
        for (const frameCb of frames) {
          frameCb(nowRel);
        }
      }
    },
    destroyNode(id) {
      listenersByNode.delete(id);
      emit(`destroy ${id}`);
    },
    getSystemTheme() {
      return systemTheme;
    },
    setSystemTheme(theme) {
      systemTheme = theme === 'light' ? 'light' : 'dark';
      emit(`theme ${systemTheme}`);
    },
    getClipboard() {
      return clipboardText;
    },
    setClipboard(text) {
      clipboardText = String(text ?? '');
      emit(`clipboard ${clipboardText}`);
    },
    sendNotification(title, body, icon = resolveAppId()) {
      emit(`comment notify_${nextListenerId++} [${icon}] ${title}: ${body}`);
    },
    openFileDialog(_title = 'Open File', defaultPath = '/home/user/document.txt') {
      return defaultPath;
    },
    saveFileDialog(_title = 'Save File', suggestedName = '/home/user/untitled.txt') {
      return suggestedName;
    },
    configureHttp(config = {}) {
      httpConfig = { ...httpConfig, ...config };
      if (typeof globalThis.__NG_NATIVE_HOST_HTTP_CONFIG__ === 'function') {
        try {
          const updated = JSON.parse(
            globalThis.__NG_NATIVE_HOST_HTTP_CONFIG__(
              JSON.stringify({ action: 'config', ...httpConfig }),
            ),
          );
          httpConfig = { ...httpConfig, ...updated };
        } catch (_err) {
          // Keep local JS copy if host parser reports an issue
        }
      }
      return { ...httpConfig };
    },
    getHttpConfig() {
      return { ...httpConfig };
    },
    registerHttpEndpoint(method, url, status, body) {
      const cleanMethod = String(method).trim().toUpperCase();
      const cleanUrl = String(url).trim();
      const key = `${cleanMethod} ${cleanUrl}`;
      httpEndpoints.set(key, {
        status: Number(status),
        statusText: Number(status) < 400 ? 'OK' : 'Error',
        body: String(body ?? ''),
        headers: {},
        transport: 'mock-route',
        callKind: 'json',
        cached: false,
      });
      if (typeof globalThis.__NG_NATIVE_HOST_HTTP_CONFIG__ === 'function') {
        globalThis.__NG_NATIVE_HOST_HTTP_CONFIG__(
          JSON.stringify({
            action: 'registerEndpoint',
            method: cleanMethod,
            url: cleanUrl,
            status: Number(status),
            body: String(body ?? ''),
          }),
        );
      }
    },
    httpRequest(method, url, body, options = {}) {
      const cleanMethod = String(method).trim().toUpperCase();
      const cleanUrl = String(url).trim();
      if (typeof globalThis.__NG_NATIVE_HOST_HTTP__ === 'function') {
        const envelope = {
          method: cleanMethod,
          url: cleanUrl,
          headers: options?.headers || {},
          responseType: options?.responseType || 'json',
          withCredentials: Boolean(options?.withCredentials),
        };
        if (options?.callKind) {
          envelope.callKind = options.callKind;
        }
        if (body !== undefined && body !== null) {
          envelope.body = String(body);
        }
        return JSON.parse(globalThis.__NG_NATIVE_HOST_HTTP__(JSON.stringify(envelope)));
      }
      const key = `${cleanMethod} ${cleanUrl}`;
      return httpEndpoints.get(key) || {
        status: 404,
        statusText: 'Not Found',
        body: JSON.stringify({ error: 'Endpoint not registered' }),
        headers: {},
        transport: 'js-fallback',
        callKind: options?.callKind || 'json',
        cached: false,
      };
    },
    httpBatchRequest(requests = []) {
      if (typeof globalThis.__NG_NATIVE_HOST_HTTP__ === 'function') {
        const mapped = requests.map((req) => ({
          method: String(req.method || 'GET').trim().toUpperCase(),
          url: String(req.url || '').trim(),
          headers: req.options?.headers || {},
          responseType: req.options?.responseType || 'json',
          callKind: req.options?.callKind || 'batch',
          withCredentials: Boolean(req.options?.withCredentials),
          ...(req.body !== undefined && req.body !== null ? { body: String(req.body) } : {}),
        }));
        return JSON.parse(globalThis.__NG_NATIVE_HOST_HTTP__(JSON.stringify(mapped)));
      }
      return requests.map((req) =>
        bridge.httpRequest(req.method || 'GET', req.url, req.body, req.options),
      );
    },
    httpRequestAsync(method, url, body, options = {}) {
      const cleanMethod = String(method || 'GET').trim().toUpperCase();
      const cleanUrl = String(url || '').trim();
      if (hasHostService()) {
        return new Promise((resolve, reject) => {
          const callbackId = registerHostCallback(resolve, reject);
          const request = {
            method: cleanMethod,
            url: cleanUrl,
            headers: options?.headers || {},
            responseType: options?.responseType || 'json',
            withCredentials: Boolean(options?.withCredentials),
          };
          if (options?.callKind) {
            request.callKind = options.callKind;
          }
          if (body !== undefined && body !== null) {
            request.body = String(body);
          }
          const ack = callHostService({ action: 'httpAsync', callbackId, request });
          if (!ack || ack.ok !== true) {
            hostCallbacks.delete(callbackId);
            reject(new Error((ack && ack.error) || 'httpRequestAsync failed'));
          }
        });
      }
      return Promise.resolve(bridge.httpRequest(cleanMethod, cleanUrl, body, options));
    },
    httpBatchRequestAsync(requests = []) {
      if (hasHostService()) {
        return new Promise((resolve, reject) => {
          const callbackId = registerHostCallback(resolve, reject);
          const mapped = (Array.isArray(requests) ? requests : []).map((req) => ({
            method: String(req.method || 'GET').trim().toUpperCase(),
            url: String(req.url || '').trim(),
            headers: req.options?.headers || {},
            responseType: req.options?.responseType || 'json',
            callKind: req.options?.callKind || 'batch',
            withCredentials: Boolean(req.options?.withCredentials),
            ...(req.body !== undefined && req.body !== null ? { body: String(req.body) } : {}),
          }));
          const ack = callHostService({ action: 'httpBatchAsync', callbackId, requests: mapped });
          if (!ack || ack.ok !== true) {
            hostCallbacks.delete(callbackId);
            reject(new Error((ack && ack.error) || 'httpBatchRequestAsync failed'));
          }
        });
      }
      return Promise.resolve(bridge.httpBatchRequest(requests));
    },
    downloadImageAsync(url, width) {
      const cleanUrl = String(url ?? '').trim();
      if (hasHostService()) {
        return new Promise((resolve) => {
          const callbackId = registerHostCallback(resolve, () =>
            resolve({ ok: false, path: bridge.resolveImageAsset(cleanUrl, width) }),
          );
          const payload = { action: 'downloadImageAsync', callbackId, url: cleanUrl };
          if (width !== undefined && width !== null) {
            payload.width = Number(width);
          }
          const ack = callHostService(payload);
          if (ack && ack.cached === true) {
            hostCallbacks.delete(callbackId);
            resolve({
              ok: true,
              cached: true,
              path: ack.path || bridge.resolveImageAsset(cleanUrl, width),
            });
            return;
          }
          if (!ack || ack.queued !== true) {
            hostCallbacks.delete(callbackId);
            resolve({
              ok: Boolean(ack && ack.ok),
              cached: false,
              path: (ack && ack.path) || bridge.resolveImageAsset(cleanUrl, width),
              error: ack && ack.error,
            });
          }
        });
      }
      return Promise.resolve({
        ok: true,
        cached: false,
        path: bridge.resolveImageAsset(cleanUrl, width),
      });
    },
    sqliteExecute(sql) {
      if (hasHostService()) {
        const res = callHostService({ action: 'sqliteExecute', sql: String(sql ?? '') });
        return Boolean(res && res.ok);
      }
      const statements = String(sql ?? '').split(';').map((s) => s.trim()).filter(Boolean);
      for (const stmt of statements) {
        const cm = stmt.match(/^CREATE\s+TABLE(?:\s+IF\s+NOT\s+EXISTS)?\s+([a-zA-Z_]\w*)\s*\(([\s\S]*)\)$/i);
        if (cm) {
          if (!sqliteTables.has(cm[1])) {
            const cols = cm[2].split(',').map((d) => d.trim().split(/\s+/)[0]).filter(Boolean);
            sqliteTables.set(cm[1], { columns: cols, rows: [] });
          }
          continue;
        }
        const im = stmt.match(/^INSERT(?:\s+OR\s+(REPLACE|IGNORE))?\s+INTO\s+([a-zA-Z_]\w*)(?:\s*\(([^)]*)\))?\s+VALUES\s*\(([\s\S]*)\)$/i);
        if (im) {
          const tbl = sqliteTables.get(im[2]) || { columns: [], rows: [] };
          sqliteTables.set(im[2], tbl);
          const cols = im[3] ? im[3].split(',').map((c) => c.trim()) : tbl.columns;
          const vals = im[4].split(',').map((v) => v.trim().replace(/^['"]|['"]$/g, ''));
          const row = {};
          for (let i = 0; i < cols.length; i++) row[cols[i]] = vals[i] ?? '';
          const pk = cols.includes('id') ? 'id' : cols[0];
          const idx = pk ? tbl.rows.findIndex((r) => r[pk] === row[pk]) : -1;
          if (idx !== -1) {
            if ((im[1] || '').toUpperCase() !== 'IGNORE') tbl.rows[idx] = row;
          } else {
            tbl.rows.push(row);
          }
          continue;
        }
        const dm = stmt.match(/^DELETE\s+FROM\s+([a-zA-Z_]\w*)(?:\s+WHERE\s+([a-zA-Z_]\w*)\s*=\s*(.+))?$/i);
        if (dm) {
          const tbl = sqliteTables.get(dm[1]);
          if (tbl) {
            tbl.rows = dm[2]
              ? tbl.rows.filter((r) => r[dm[2]] !== dm[3].trim().replace(/^['"]|['"]$/g, ''))
              : [];
          }
        }
      }
      return true;
    },
    sqliteQuery(sql) {
      if (hasHostService()) {
        const res = callHostService({ action: 'sqliteQuery', sql: String(sql ?? '') });
        return res && res.ok && Array.isArray(res.rows) ? res.rows : [];
      }
      const m = String(sql ?? '').trim().replace(/;+\s*$/, '').match(
        /^SELECT\s+(.+?)\s+FROM\s+([a-zA-Z_]\w*)(?:\s+ORDER\s+BY\s+([a-zA-Z_]\w*)(?:\s+(ASC|DESC))?)?$/i,
      );
      if (!m) return [];
      const tbl = sqliteTables.get(m[2]);
      if (!tbl) return [];
      return tbl.rows.map((r) => ({ ...r }));
    },
    getNetworkStatus() {
      if (hasHostService()) {
        const res = callHostService({ action: 'systemEnv' });
        if (res && res.ok) {
          return {
            online: Boolean(res.online),
            metered: Boolean(res.metered),
            connectivity: res.connectivity || 'full',
          };
        }
      }
      return { ...networkStatus };
    },
    setNetworkStatus(online, metered = false) {
      networkStatus = {
        online: Boolean(online),
        metered: Boolean(metered),
        connectivity: !online ? 'offline' : metered ? 'limited' : 'full',
      };
      callHostService({
        action: 'systemEnv',
        online: Boolean(online),
        metered: Boolean(metered),
      });
    },
    getSystemLocale() {
      if (hasHostService()) {
        const res = callHostService({ action: 'systemEnv' });
        if (res && res.ok && typeof res.locale === 'string' && res.locale) {
          return res.locale;
        }
      }
      return systemLocale;
    },
    getSystemTimezone() {
      if (hasHostService()) {
        const res = callHostService({ action: 'systemEnv' });
        if (res && res.ok && typeof res.timezone === 'string' && res.timezone) {
          return res.timezone;
        }
      }
      return systemTimezone;
    },
    resolveImageAsset(src, width) {
      const trimmed = String(src ?? '').trim();
      if (hasHostService()) {
        const payload = { action: 'systemEnv', src: trimmed };
        if (width !== undefined && width !== null) {
          payload.width = Number(width);
        }
        const res = callHostService(payload);
        if (res && res.ok && typeof res.resolvedImage === 'string') {
          return res.resolvedImage;
        }
      }
      if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
        const clean = trimmed.split('?')[0].toLowerCase();
        const m = clean.match(/\.(png|jpg|jpeg|webp|svg|ico)$/);
        const ext = m ? m[1] : 'img';
        const suffix = width ? `?w=${width}` : '';
        return `/home/user/.cache/${resolveAppId()}/images/img_${bridge.computeHash(trimmed)}.${ext}${suffix}`;
      }
      return trimmed;
    },
    logSystemError(message) {
      const text = String(message ?? '');
      pushSystemError(text);
      callHostService({ action: 'logError', message: text });
    },
    getSystemErrors() {
      return [...systemErrors];
    },
    computeFuzzyFilter(items = [], query = '', limit = 50) {
      if (hasHostService()) {
        const res = callHostService({
          action: 'computeFuzzyFilter',
          items: Array.from(items, (x) => String(x ?? '')),
          query: String(query ?? ''),
          limit: Number(limit ?? 50),
        });
        return res && res.ok && Array.isArray(res.matches) ? res.matches : [];
      }
      const needle = String(query ?? '').trim().toLowerCase();
      const out = [];
      for (let index = 0; index < items.length; index++) {
        const value = String(items[index] ?? '');
        const cand = value.toLowerCase();
        let score = null;
        if (!needle) score = 1;
        else if (cand === needle) score = 1000;
        else if (cand.startsWith(needle)) score = 600 + Math.max(0, 80 - value.length);
        else if (cand.includes(needle)) score = 350 + Math.max(0, 60 - cand.indexOf(needle) * 2);
        if (score !== null) out.push({ index, score, value });
      }
      out.sort((a, b) => (b.score !== a.score ? b.score - a.score : a.index - b.index));
      return out.slice(0, limit);
    },
    computeHash(input = '') {
      const str = String(input ?? '');
      if (hasHostService()) {
        const res = callHostService({ action: 'computeHash', input: str });
        if (res && res.ok && typeof res.hash === 'string') {
          return res.hash;
        }
      }
      let hi = 0x84222325;
      let lo = 0xcbf29ce4;
      for (let i = 0; i < str.length; i++) {
        lo = (lo ^ str.charCodeAt(i)) >>> 0;
        hi = (Math.imul(hi, 0x01000193) ^ (lo >>> 16)) >>> 0;
        lo = Math.imul(lo, 0x01000193) >>> 0;
      }
      return hi.toString(16).padStart(8, '0') + lo.toString(16).padStart(8, '0');
    },
    fsResolvePath(kind = 'config', relPath = '') {
      if (hasHostService()) {
        const res = callHostService({
          action: 'fsResolvePath',
          kind: String(kind ?? 'config'),
          relPath: String(relPath ?? ''),
        });
        if (!res || !res.ok) {
          throw new Error((res && res.error) || 'Invalid XDG path: ' + relPath);
        }
        return res.path;
      }
      const clean = String(relPath ?? '').trim().replace(/^[/\\]+/, '');
      if (!clean || clean.split(/[/\\]/).includes('..')) {
        throw new Error('Invalid XDG path: ' + relPath);
      }
      const appId = resolveAppId();
      const base =
        kind === 'data'
          ? `/home/user/.local/share/${appId}`
          : kind === 'cache'
            ? `/home/user/.cache/${appId}`
            : `/home/user/.config/${appId}`;
      return base + '/' + clean;
    },
    fsReadText(path) {
      if (hasHostService()) {
        const res = callHostService({ action: 'fsReadText', path: String(path ?? '') });
        return res && res.ok ? (res.content ?? null) : null;
      }
      const clean = String(path ?? '').trim();
      return fsFiles.has(clean) ? fsFiles.get(clean) : null;
    },
    fsWriteText(path, content) {
      if (hasHostService()) {
        const res = callHostService({
          action: 'fsWriteText',
          path: String(path ?? ''),
          content: String(content ?? ''),
        });
        return Boolean(res && res.ok);
      }
      const clean = String(path ?? '').trim();
      if (!clean) return false;
      fsFiles.set(clean, String(content ?? ''));
      if (fsWatched.has(clean)) fsChanged.add(clean);
      return true;
    },
    fsWatchFile(path) {
      if (hasHostService()) {
        const res = callHostService({ action: 'fsWatchFile', path: String(path ?? '') });
        return Boolean(res && res.ok);
      }
      const clean = String(path ?? '').trim();
      if (!clean) return false;
      fsWatched.add(clean);
      return true;
    },
    fsPollChangedFiles() {
      if (hasHostService()) {
        const res = callHostService({ action: 'fsPollChangedFiles' });
        return res && res.ok && Array.isArray(res.paths) ? res.paths : [];
      }
      const out = Array.from(fsChanged);
      fsChanged.clear();
      return out;
    },
    getWindowGeometry() {
      if (hasHostService()) {
        const res = callHostService({ action: 'windowGetGeometry' });
        if (res && res.ok) {
          windowGeometry = {
            width: Number(res.width || 1020),
            height: Number(res.height || 720),
            x: typeof res.x === 'number' ? res.x : null,
            y: typeof res.y === 'number' ? res.y : null,
            maximized: Boolean(res.maximized),
          };
          return { ...windowGeometry };
        }
      }
      return { ...windowGeometry };
    },
    setWindowGeometry(geom = {}) {
      const next = {
        width: typeof geom.width === 'number' && geom.width >= 160 ? Math.trunc(geom.width) : windowGeometry.width,
        height: typeof geom.height === 'number' && geom.height >= 120 ? Math.trunc(geom.height) : windowGeometry.height,
        x: typeof geom.x === 'number' ? Math.trunc(geom.x) : (geom.x === null ? null : windowGeometry.x),
        y: typeof geom.y === 'number' ? Math.trunc(geom.y) : (geom.y === null ? null : windowGeometry.y),
        maximized: typeof geom.maximized === 'boolean' ? geom.maximized : windowGeometry.maximized,
      };
      if (hasHostService()) {
        const res = callHostService({ action: 'windowSetGeometry', ...next });
        if (res && res.ok) {
          windowGeometry = {
            width: Number(res.width || next.width),
            height: Number(res.height || next.height),
            x: typeof res.x === 'number' ? res.x : null,
            y: typeof res.y === 'number' ? res.y : null,
            maximized: Boolean(res.maximized),
          };
          return { ...windowGeometry };
        }
      }
      windowGeometry = next;
      return { ...windowGeometry };
    },
    getAppMenu() {
      if (hasHostService()) {
        const res = callHostService({ action: 'menuGetItems' });
        if (res && res.ok && Array.isArray(res.items)) {
          appMenuItems = res.items;
        }
      }
      return appMenuItems.map((item) => ({ ...item }));
    },
    setAppMenu(items = []) {
      const normalized = Array.isArray(items)
        ? items.map((item) => ({
            id: String(item?.id ?? ''),
            label: String(item?.label ?? item?.id ?? ''),
            accelerator: item?.accelerator ? String(item.accelerator) : null,
            enabled: item?.enabled !== false,
          }))
        : [];
      if (hasHostService()) {
        const res = callHostService({ action: 'menuSetItems', items: normalized });
        if (res && res.ok && Array.isArray(res.items)) {
          appMenuItems = res.items;
          return appMenuItems.map((item) => ({ ...item }));
        }
      }
      appMenuItems = normalized;
      return appMenuItems.map((item) => ({ ...item }));
    },
    getTrayState() {
      if (hasHostService()) {
        const res = callHostService({ action: 'trayGetState' });
        if (res && res.ok) {
          trayState = {
            visible: Boolean(res.visible),
            icon: String(res.icon ?? ''),
            tooltip: String(res.tooltip ?? ''),
            menuItems: Array.isArray(res.menuItems) ? res.menuItems : [],
          };
        }
      }
      return { ...trayState, menuItems: [...trayState.menuItems] };
    },
    setTrayState(state = {}) {
      const next = {
        visible: typeof state.visible === 'boolean' ? state.visible : trayState.visible,
        icon: typeof state.icon === 'string' ? state.icon : trayState.icon,
        tooltip: typeof state.tooltip === 'string' ? state.tooltip : trayState.tooltip,
        menuItems: Array.isArray(state.menuItems) ? state.menuItems : trayState.menuItems,
      };
      if (hasHostService()) {
        const res = callHostService({ action: 'traySetState', ...next });
        if (res && res.ok) {
          trayState = {
            visible: Boolean(res.visible),
            icon: String(res.icon ?? ''),
            tooltip: String(res.tooltip ?? ''),
            menuItems: Array.isArray(res.menuItems) ? res.menuItems : [],
          };
          return { ...trayState, menuItems: [...trayState.menuItems] };
        }
      }
      trayState = next;
      return { ...trayState, menuItems: [...trayState.menuItems] };
    },
    registerShortcut(accelerator, actionId) {
      const cleanAcc = String(accelerator ?? '').trim();
      const cleanAction = String(actionId ?? '').trim();
      if (!cleanAcc || !cleanAction) return shortcuts.map((s) => ({ ...s }));
      if (hasHostService()) {
        const res = callHostService({ action: 'shortcutRegister', accelerator: cleanAcc, actionId: cleanAction });
        if (res && res.ok && Array.isArray(res.shortcuts)) {
          shortcuts = res.shortcuts;
          return shortcuts.map((s) => ({ ...s }));
        }
      }
      const existing = shortcuts.find((s) => s.accelerator === cleanAcc);
      if (existing) existing.actionId = cleanAction;
      else shortcuts.push({ accelerator: cleanAcc, actionId: cleanAction });
      return shortcuts.map((s) => ({ ...s }));
    },
    unregisterShortcut(accelerator) {
      const cleanAcc = String(accelerator ?? '').trim();
      if (hasHostService()) {
        const res = callHostService({ action: 'shortcutUnregister', accelerator: cleanAcc });
        if (res && res.ok && Array.isArray(res.shortcuts)) {
          shortcuts = res.shortcuts;
          return shortcuts.map((s) => ({ ...s }));
        }
      }
      shortcuts = shortcuts.filter((s) => s.accelerator !== cleanAcc);
      return shortcuts.map((s) => ({ ...s }));
    },
    getRegisteredShortcuts() {
      if (hasHostService()) {
        const res = callHostService({ action: 'shortcutList' });
        if (res && res.ok && Array.isArray(res.shortcuts)) {
          shortcuts = res.shortcuts;
        }
      }
      return shortcuts.map((s) => ({ ...s }));
    },
    __drainQueue() {
      if (queue.length === 0) return '';
      const out = queue.join('\n');
      queue = [];
      return out;
    }
  };

  if (typeof globalThis.fetch !== 'function') {
    globalThis.fetch = (input, init = {}) => {
      const url = typeof input === 'string' ? input : String(input?.url ?? '');
      return bridge
        .httpRequestAsync(init.method || 'GET', url, init.body, {
          headers: init.headers || {},
        })
        .then((res) => ({
          ok: res.status >= 200 && res.status < 300,
          status: res.status,
          statusText: res.statusText,
          headers: res.headers || {},
          text: () => Promise.resolve(res.body),
          json: () => Promise.resolve(JSON.parse(res.body || 'null')),
          arrayBuffer: () =>
            Promise.resolve(new TextEncoder().encode(String(res.body ?? '')).buffer),
        }));
    };
  }

  if (typeof globalThis.window === 'undefined') {
    globalThis.window = globalThis;
  }
  if (typeof globalThis.self === 'undefined') {
    globalThis.self = globalThis;
  }
  if (typeof globalThis.location === 'undefined') {
    globalThis.location = {
      href: 'http://localhost/',
      origin: 'http://localhost',
      protocol: 'http:',
      host: 'localhost',
      hostname: 'localhost',
      port: '',
      pathname: '/',
      search: '',
      hash: '',
      toString() {
        return this.href;
      },
    };
  }
  if (typeof globalThis.history === 'undefined') {
    globalThis.history = {
      state: null,
      length: 1,
      pushState() {},
      replaceState() {},
      back() {},
      forward() {},
      go() {},
    };
  }
  if (typeof globalThis.navigator === 'undefined') {
    globalThis.navigator = {
      userAgent: 'NgDesktop/0.3 (JavaScriptCore)',
      language: 'en-US',
      languages: ['en-US'],
      platform: 'Linux x86_64',
      onLine: true,
    };
  }
  if (typeof globalThis.Event === 'undefined') {
    globalThis.Event = class Event {
      constructor(type, init = {}) {
        this.type = String(type);
        this.bubbles = Boolean(init.bubbles);
        this.cancelable = Boolean(init.cancelable);
        this.composed = Boolean(init.composed);
        this.defaultPrevented = false;
        this.target = null;
        this.currentTarget = null;
      }
      preventDefault() {
        if (this.cancelable) this.defaultPrevented = true;
      }
      stopPropagation() {}
      stopImmediatePropagation() {}
    };
  }
  if (typeof globalThis.CustomEvent === 'undefined') {
    globalThis.CustomEvent = class CustomEvent extends globalThis.Event {
      constructor(type, init = {}) {
        super(type, init);
        this.detail = init.detail !== undefined ? init.detail : null;
      }
    };
  }
  if (typeof globalThis.Node === 'undefined') {
    class DomNode {
      constructor(nodeType, nodeName) {
        this.nodeType = nodeType;
        this.nodeName = nodeName;
        this.parentNode = null;
        this.childNodes = [];
        this._listeners = new Map();
      }
      get firstChild() {
        return this.childNodes[0] || null;
      }
      get lastChild() {
        return this.childNodes.length ? this.childNodes[this.childNodes.length - 1] : null;
      }
      get nextSibling() {
        if (!this.parentNode) return null;
        const idx = this.parentNode.childNodes.indexOf(this);
        return idx !== -1 && idx + 1 < this.parentNode.childNodes.length
          ? this.parentNode.childNodes[idx + 1]
          : null;
      }
      get previousSibling() {
        if (!this.parentNode) return null;
        const idx = this.parentNode.childNodes.indexOf(this);
        return idx > 0 ? this.parentNode.childNodes[idx - 1] : null;
      }
      appendChild(child) {
        if (child.parentNode) child.parentNode.removeChild(child);
        child.parentNode = this;
        this.childNodes.push(child);
        return child;
      }
      insertBefore(newChild, refChild) {
        if (!refChild) return this.appendChild(newChild);
        if (newChild.parentNode) newChild.parentNode.removeChild(newChild);
        const idx = this.childNodes.indexOf(refChild);
        newChild.parentNode = this;
        if (idx === -1) this.childNodes.push(newChild);
        else this.childNodes.splice(idx, 0, newChild);
        return newChild;
      }
      removeChild(child) {
        const idx = this.childNodes.indexOf(child);
        if (idx !== -1) this.childNodes.splice(idx, 1);
        child.parentNode = null;
        return child;
      }
      remove() {
        if (this.parentNode) this.parentNode.removeChild(this);
      }
      addEventListener(type, listener) {
        if (!this._listeners.has(type)) this._listeners.set(type, new Set());
        this._listeners.get(type).add(listener);
      }
      removeEventListener(type, listener) {
        const set = this._listeners.get(type);
        if (set) set.delete(listener);
      }
      dispatchEvent(evt) {
        const set = this._listeners.get(evt?.type);
        if (set) {
          evt.target = evt.target || this;
          evt.currentTarget = this;
          for (const fn of set) fn.call(this, evt);
        }
        return !(evt && evt.defaultPrevented);
      }
    }
    DomNode.ELEMENT_NODE = 1;
    DomNode.TEXT_NODE = 3;
    DomNode.COMMENT_NODE = 8;
    DomNode.DOCUMENT_NODE = 9;
    DomNode.DOCUMENT_FRAGMENT_NODE = 11;
    globalThis.Node = DomNode;
  }
  if (typeof globalThis.Element === 'undefined') {
    globalThis.Element = class Element extends globalThis.Node {
      constructor(tagName) {
        const upper = String(tagName ?? 'div').toUpperCase();
        super(1, upper);
        this.tagName = upper;
        this.attributes = new Map();
        this.classList = {
          add: (...names) => {
            const cls = new Set(String(this.getAttribute('class') || '').split(/\s+/).filter(Boolean));
            for (const n of names) if (n) cls.add(n);
            this.setAttribute('class', Array.from(cls).join(' '));
          },
          remove: (...names) => {
            const cls = new Set(String(this.getAttribute('class') || '').split(/\s+/).filter(Boolean));
            for (const n of names) cls.delete(n);
            this.setAttribute('class', Array.from(cls).join(' '));
          },
          contains: (n) =>
            String(this.getAttribute('class') || '').split(/\s+/).filter(Boolean).includes(n),
          toggle: (n, force) => {
            const has = this.classList.contains(n);
            const next = force !== undefined ? Boolean(force) : !has;
            if (next) this.classList.add(n);
            else this.classList.remove(n);
            return next;
          },
        };
      }
      get children() {
        return this.childNodes.filter((n) => n.nodeType === 1);
      }
      setAttribute(name, value) {
        this.attributes.set(String(name), String(value ?? ''));
      }
      setAttributeNS(_ns, name, value) {
        this.setAttribute(name, value);
      }
      getAttribute(name) {
        return this.attributes.has(String(name)) ? this.attributes.get(String(name)) : null;
      }
      hasAttribute(name) {
        return this.attributes.has(String(name));
      }
      removeAttribute(name) {
        this.attributes.delete(String(name));
      }
      removeAttributeNS(_ns, name) {
        this.removeAttribute(name);
      }
      getElementsByTagName(tag) {
        const target = String(tag).toUpperCase();
        const out = [];
        const visit = (node) => {
          for (const child of node.childNodes) {
            if (child.nodeType === 1 && (target === '*' || child.tagName === target)) out.push(child);
            visit(child);
          }
        };
        visit(this);
        return out;
      }
      querySelectorAll(selector) {
        const sel = String(selector ?? '').trim();
        if (!sel) return [];
        const out = [];
        const matches = (el) => {
          if (sel.startsWith('#')) return el.getAttribute('id') === sel.slice(1);
          if (sel.startsWith('.')) return el.classList.contains(sel.slice(1));
          return el.tagName === sel.toUpperCase();
        };
        const visit = (node) => {
          for (const child of node.childNodes) {
            if (child.nodeType === 1) {
              if (matches(child)) out.push(child);
              visit(child);
            }
          }
        };
        visit(this);
        return out;
      }
      querySelector(selector) {
        return this.querySelectorAll(selector)[0] || null;
      }
      cloneNode(deep = false) {
        const copy = new this.constructor(this.tagName);
        for (const [k, v] of this.attributes.entries()) copy.setAttribute(k, v);
        copy.textContent = this.textContent;
        if (deep) {
          for (const child of this.childNodes) copy.appendChild(child.cloneNode(true));
        }
        return copy;
      }
    };
  }
  if (typeof globalThis.HTMLElement === 'undefined') {
    globalThis.HTMLElement = class HTMLElement extends globalThis.Element {
      constructor(tagName) {
        super(tagName);
        this.style = {};
        this.dataset = {};
        this.textContent = '';
        this.innerHTML = '';
        this._content = null;
      }
      get content() {
        if (!this._content) {
          const frag = new globalThis.Node(11, '#document-fragment');
          frag.querySelectorAll = () => [];
          frag.querySelector = () => null;
          this._content = frag;
        }
        return this._content;
      }
    };
  }
  if (typeof globalThis.Document === 'undefined') {
    globalThis.Document = class Document extends globalThis.Node {
      constructor() {
        super(9, '#document');
        this.baseURI = 'http://localhost/';
        this.cookie = '';
        this.title = '';
        this.defaultView = globalThis;
        this.location = globalThis.location;
        this.documentElement = new globalThis.HTMLElement('html');
        this.head = new globalThis.HTMLElement('head');
        this.body = new globalThis.HTMLElement('body');
        this.documentElement.appendChild(this.head);
        this.documentElement.appendChild(this.body);
        this.appendChild(this.documentElement);
        this.implementation = {
          createHTMLDocument(docTitle = '') {
            const doc = new globalThis.Document();
            doc.title = String(docTitle ?? '');
            return doc;
          },
        };
      }
      createElement(tag) {
        return new globalThis.HTMLElement(tag);
      }
      createElementNS(_ns, tag) {
        return new globalThis.HTMLElement(tag);
      }
      createTextNode(text) {
        const node = new globalThis.Node(3, '#text');
        node.nodeValue = String(text ?? '');
        node.textContent = node.nodeValue;
        return node;
      }
      createComment(text) {
        const node = new globalThis.Node(8, '#comment');
        node.nodeValue = String(text ?? '');
        node.textContent = node.nodeValue;
        return node;
      }
      createDocumentFragment() {
        const frag = new globalThis.Node(11, '#document-fragment');
        frag.querySelectorAll = globalThis.Element.prototype.querySelectorAll;
        frag.querySelector = globalThis.Element.prototype.querySelector;
        return frag;
      }
      getElementsByTagName(tag) {
        return this.documentElement.getElementsByTagName(tag);
      }
      querySelectorAll(selector) {
        const s = String(selector ?? '').trim().toLowerCase();
        if (s === 'head') return [this.head];
        if (s === 'body') return [this.body];
        if (s === 'html') return [this.documentElement];
        return this.documentElement.querySelectorAll(selector);
      }
      querySelector(selector) {
        return this.querySelectorAll(selector)[0] || null;
      }
      getElementById(id) {
        return this.querySelector('#' + id);
      }
    };
  }
  if (typeof globalThis.document === 'undefined') {
    globalThis.document = new globalThis.Document();
  }
  if (typeof globalThis.addEventListener !== 'function') {
    globalThis.addEventListener = (type, fn) => globalThis.document.addEventListener(type, fn);
    globalThis.removeEventListener = (type, fn) => globalThis.document.removeEventListener(type, fn);
    globalThis.dispatchEvent = (evt) => globalThis.document.dispatchEvent(evt);
  }
  if (typeof globalThis.TextEncoder === 'undefined') {
    globalThis.TextEncoder = class TextEncoder {
      encode(str = '') {
        const out = [];
        for (let i = 0; i < str.length; i++) {
          let code = str.charCodeAt(i);
          if (code < 0x80) out.push(code);
          else if (code < 0x800) {
            out.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f));
          } else {
            out.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
          }
        }
        return new Uint8Array(out);
      }
    };
  }
  if (typeof globalThis.TextDecoder === 'undefined') {
    globalThis.TextDecoder = class TextDecoder {
      decode(bytes = new Uint8Array()) {
        const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
        let out = '';
        for (let i = 0; i < arr.length; i++) out += String.fromCharCode(arr[i]);
        return out;
      }
    };
  }
  if (typeof globalThis.URLSearchParams === 'undefined') {
    globalThis.URLSearchParams = class URLSearchParams {
      constructor(init = '') {
        this._entries = [];
        const clean = String(init ?? '').replace(/^\?/, '');
        if (clean) {
          for (const part of clean.split('&')) {
            if (!part) continue;
            const eq = part.indexOf('=');
            const k = eq === -1 ? part : part.slice(0, eq);
            const v = eq === -1 ? '' : part.slice(eq + 1);
            this._entries.push([decodeURIComponent(k), decodeURIComponent(v)]);
          }
        }
      }
      get(key) {
        const found = this._entries.find(([k]) => k === key);
        return found ? found[1] : null;
      }
      set(key, value) {
        this._entries = this._entries.filter(([k]) => k !== key);
        this._entries.push([String(key), String(value)]);
      }
      append(key, value) {
        this._entries.push([String(key), String(value)]);
      }
      has(key) {
        return this._entries.some(([k]) => k === key);
      }
      toString() {
        return this._entries
          .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
          .join('&');
      }
    };
  }
  if (typeof globalThis.URL === 'undefined') {
    globalThis.URL = class URL {
      constructor(input, base = 'http://localhost/') {
        const raw = String(input ?? '');
        this.href = raw.includes('://') ? raw : String(base).replace(/\/$/, '') + '/' + raw.replace(/^\//, '');
        const m = this.href.match(/^([a-z]+:)\/\/([^/?#]+)(\/[^?#]*)?(\?[^#]*)?(#.*)?$/i);
        this.protocol = m ? m[1] : 'http:';
        this.host = m ? m[2] : 'localhost';
        this.hostname = this.host.split(':')[0];
        this.pathname = (m && m[3]) || '/';
        this.search = (m && m[4]) || '';
        this.hash = (m && m[5]) || '';
        this.origin = `${this.protocol}//${this.host}`;
        this.searchParams = new globalThis.URLSearchParams(this.search);
      }
      toString() {
        return this.href;
      }
    };
  }
  if (typeof globalThis.AbortController === 'undefined') {
    globalThis.AbortSignal = class AbortSignal {
      constructor() {
        this.aborted = false;
        this.reason = undefined;
      }
      addEventListener() {}
      removeEventListener() {}
    };
    globalThis.AbortController = class AbortController {
      constructor() {
        this.signal = new globalThis.AbortSignal();
      }
      abort(reason) {
        this.signal.aborted = true;
        this.signal.reason = reason;
      }
    };
  }
  if (typeof globalThis.Headers === 'undefined') {
    globalThis.Headers = class Headers {
      constructor(init = {}) {
        this._map = new Map();
        if (init && typeof init === 'object') {
          for (const [k, v] of Object.entries(init)) {
            this._map.set(k.toLowerCase(), String(v));
          }
        }
      }
      get(k) {
        return this._map.get(String(k).toLowerCase()) ?? null;
      }
      set(k, v) {
        this._map.set(String(k).toLowerCase(), String(v));
      }
      has(k) {
        return this._map.has(String(k).toLowerCase());
      }
      delete(k) {
        this._map.delete(String(k).toLowerCase());
      }
      forEach(cb) {
        this._map.forEach((v, k) => cb(v, k, this));
      }
      entries() {
        return this._map.entries();
      }
      [Symbol.iterator]() {
        return this._map.entries();
      }
    };
  }
  if (typeof globalThis.Blob === 'undefined') {
    globalThis.Blob = class Blob {
      constructor(parts = [], options = {}) {
        this._parts = parts;
        this.type = options.type || '';
        this.size = parts.reduce((acc, p) => acc + String(p ?? '').length, 0);
      }
      text() {
        return Promise.resolve(this._parts.map((p) => String(p ?? '')).join(''));
      }
    };
  }
  for (const obsName of ['IntersectionObserver', 'MutationObserver', 'ResizeObserver']) {
    if (typeof globalThis[obsName] === 'undefined') {
      globalThis[obsName] = class {
        observe() {}
        unobserve() {}
        disconnect() {}
        takeRecords() {
          return [];
        }
      };
    }
  }

  return bridge;
})();
"#;
