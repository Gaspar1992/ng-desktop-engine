/// Standard `globalThis.__NG_NATIVE__` + Web/Host globals (`console`, `queueMicrotask`, `setTimeout`, `fetch`)
/// injected into every JS engine context.
/// `@ng-desktop/platform` (`NativeRenderer2`), `@ng-desktop/forms`, `@ng-desktop/http`, and
/// `@ng-desktop/desktop` call this object to queue frame-batched scene/desktop commands,
/// configure OS-native HTTP transports, and register live JS event listener closures.
pub const NG_NATIVE_BOOTSTRAP_JS: &str = r#"
globalThis.__NG_NATIVE__ = (() => {
  let queue = [];
  let logs = [];
  const resolveAppId = () => {
    const raw = globalThis.__NG_NATIVE_APP_ID__;
    return typeof raw === 'string' && raw.trim() ? raw.trim() : 'ng-desktop-app';
  };
  let timers = new Map();
  let nextTimerId = 1;
  let listeners = new Map();
  let nextListenerId = 1;
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

  globalThis.console = {
    log: (...args) => logs.push(['LOG', args.join(' ')]),
    info: (...args) => logs.push(['INFO', args.join(' ')]),
    warn: (...args) => logs.push(['WARN', args.join(' ')]),
    error: (...args) => logs.push(['ERROR', args.join(' ')]),
  };

  if (typeof globalThis.queueMicrotask !== 'function') {
    globalThis.queueMicrotask = (cb) => Promise.resolve().then(cb);
  }

  globalThis.setTimeout = (cb, _ms = 0) => {
    const id = nextTimerId++;
    timers.set(id, cb);
    return id;
  };

  globalThis.clearTimeout = (id) => {
    timers.delete(id);
  };

  const bridge = {
    registerStyle(cls, bg = 'none', fg = 'none', pad = 0, space = 0) {
      queue.push(`style ${cls} ${bg} ${fg} ${pad} ${space}`);
    },
    declareSignal(name, initial = 0) {
      queue.push(`signal ${name} ${initial}`);
    },
    createElement(id, tag) {
      queue.push(`create ${id} ${tag}`);
      return id;
    },
    createComment(id, text = 'anchor') {
      queue.push(`comment ${id} ${text}`);
      return id;
    },
    setText(id, text) {
      queue.push(`text ${id} ${text}`);
    },
    setAttribute(id, key, value) {
      queue.push(`attr ${id} ${key} ${value}`);
    },
    setWindowIcon(id, appId, icoPath) {
      queue.push(`icon ${id} ${appId} ${icoPath}`);
    },
    appendChild(parent, child) {
      queue.push(`append ${parent} ${child}`);
    },
    insertBefore(parent, child, refChild) {
      if (refChild) {
        queue.push(`insert_before ${parent} ${child} ${refChild}`);
      } else {
        queue.push(`insert_before ${parent} ${child}`);
      }
    },
    removeChild(parent, child) {
      queue.push(`remove_child ${parent} ${child}`);
    },
    addClass(id, className) {
      queue.push(`add_class ${id} ${className}`);
    },
    bindText(id, signalName, template) {
      queue.push(`bind_text ${id} ${signalName} ${template}`);
    },
    bindAttr(id, attrKey, signalName, template = '{}') {
      queue.push(`bind_attr ${id} ${attrKey} ${signalName} ${template}`);
    },
    bindVisible(id, signalName, op = '!=', rhs = 0) {
      queue.push(`bind_visible ${id} ${signalName} ${op} ${rhs}`);
    },
    onAdd(id, eventName, signalName, delta = 1) {
      queue.push(`on_add ${id} ${eventName} ${signalName} ${delta}`);
    },
    onSet(id, eventName, signalName, value = 0) {
      queue.push(`on_set ${id} ${eventName} ${signalName} ${value}`);
    },
    listen(id, eventName, callback) {
      const lid = nextListenerId++;
      listeners.set(lid, { id, eventName, callback });
      return () => listeners.delete(lid);
    },
    __dispatchEvent(id, eventName, eventPayload = {}) {
      for (const entry of listeners.values()) {
        if (entry.id === id && entry.eventName === eventName) {
          entry.callback(eventPayload);
        }
      }
    },
    __flushTimers() {
      const pending = Array.from(timers.values());
      timers.clear();
      for (const cb of pending) {
        cb();
      }
    },
    destroyNode(id) {
      queue.push(`destroy ${id}`);
    },
    getSystemTheme() {
      return systemTheme;
    },
    setSystemTheme(theme) {
      systemTheme = theme === 'light' ? 'light' : 'dark';
      queue.push(`theme ${systemTheme}`);
    },
    getClipboard() {
      return clipboardText;
    },
    setClipboard(text) {
      clipboardText = String(text ?? '');
      queue.push(`clipboard ${clipboardText}`);
    },
    sendNotification(title, body, icon = resolveAppId()) {
      queue.push(`comment notify_${nextListenerId++} [${icon}] ${title}: ${body}`);
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
    sqliteExecute(sql) {
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
      const m = String(sql ?? '').trim().replace(/;+\s*$/, '').match(
        /^SELECT\s+(.+?)\s+FROM\s+([a-zA-Z_]\w*)(?:\s+ORDER\s+BY\s+([a-zA-Z_]\w*)(?:\s+(ASC|DESC))?)?$/i,
      );
      if (!m) return [];
      const tbl = sqliteTables.get(m[2]);
      if (!tbl) return [];
      return tbl.rows.map((r) => ({ ...r }));
    },
    getNetworkStatus() {
      return { ...networkStatus };
    },
    setNetworkStatus(online, metered = false) {
      networkStatus = {
        online: Boolean(online),
        metered: Boolean(metered),
        connectivity: !online ? 'offline' : metered ? 'limited' : 'full',
      };
    },
    getSystemLocale() {
      return systemLocale;
    },
    getSystemTimezone() {
      return systemTimezone;
    },
    resolveImageAsset(src, width) {
      const trimmed = String(src ?? '').trim();
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
      systemErrors.push(String(message ?? ''));
    },
    getSystemErrors() {
      return [...systemErrors];
    },
    computeFuzzyFilter(items = [], query = '', limit = 50) {
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
      const clean = String(path ?? '').trim();
      return fsFiles.has(clean) ? fsFiles.get(clean) : null;
    },
    fsWriteText(path, content) {
      const clean = String(path ?? '').trim();
      if (!clean) return false;
      fsFiles.set(clean, String(content ?? ''));
      if (fsWatched.has(clean)) fsChanged.add(clean);
      return true;
    },
    fsWatchFile(path) {
      const clean = String(path ?? '').trim();
      if (!clean) return false;
      fsWatched.add(clean);
      return true;
    },
    fsPollChangedFiles() {
      const out = Array.from(fsChanged);
      fsChanged.clear();
      return out;
    },
    __drainQueue() {
      const out = queue.join('\n');
      queue = [];
      return out;
    }
  };

  if (typeof globalThis.fetch !== 'function') {
    globalThis.fetch = (input, init = {}) => {
      const url = typeof input === 'string' ? input : String(input?.url ?? '');
      const res = bridge.httpRequest(init.method || 'GET', url, init.body, {
        headers: init.headers || {},
      });
      return Promise.resolve({
        ok: res.status >= 200 && res.status < 300,
        status: res.status,
        statusText: res.statusText,
        headers: res.headers || {},
        text: () => Promise.resolve(res.body),
        json: () => Promise.resolve(JSON.parse(res.body || 'null')),
      });
    };
  }

  return bridge;
})();
"#;
