/// Standard `globalThis.__NG_NATIVE__` + Web/Host globals (`console`, `queueMicrotask`, `setTimeout`, `fetch`)
/// injected into every JS engine context.
/// `@ng-native/platform` (`NativeRenderer2`), `@ng-native/forms`, `@ng-native/http`, and
/// `@ng-native/desktop` call this object to queue frame-batched scene/desktop commands,
/// configure OS-native HTTP transports, and register live JS event listener closures.
pub const NG_NATIVE_BOOTSTRAP_JS: &str = r"
globalThis.__NG_NATIVE__ = (() => {
  let queue = [];
  let logs = [];
  let timers = new Map();
  let nextTimerId = 1;
  let listeners = new Map();
  let nextListenerId = 1;
  let systemTheme = 'dark';
  let clipboardText = '';
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
    sendNotification(title, body, icon = 'ng-linux-counter') {
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
";
