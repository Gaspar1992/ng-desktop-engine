import { existsSync, readFileSync, statSync } from 'node:fs';
import net from 'node:net';

/**
 * Resolves the connection target for either a Unix domain socket path or a
 * loopback `tcp://127.0.0.1:<port>` pointer file used on Windows.
 * Returns `{ error }` if a `tcp://` pointer file has an invalid host or port.
 */
function resolveHmrToken(socketPath, explicitToken) {
  if (typeof explicitToken === 'string' && explicitToken.trim()) {
    return explicitToken.trim();
  }
  const envToken = process.env.NG_DESKTOP_HMR_TOKEN?.trim();
  if (envToken) {
    return envToken;
  }
  const tokenPath = `${socketPath}.token`;
  try {
    if (existsSync(tokenPath)) {
      const token = readFileSync(tokenPath, 'utf8').trim();
      if (token) {
        return token;
      }
    }
  } catch {
    // Ignore unreadable token files; server enforces auth if required.
  }
  return undefined;
}

function resolveHmrTarget(socketPath) {
  try {
    if (statSync(socketPath).isFile()) {
      const pointer = readFileSync(socketPath, 'utf8').trim();
      if (pointer.startsWith('tcp://')) {
        const parsed = new URL(pointer);
        const port = Number(parsed.port);
        if (!parsed.hostname || !Number.isInteger(port) || port < 1 || port > 65535) {
          return { error: `Invalid TCP HMR pointer: ${pointer}` };
        }
        return { target: { host: parsed.hostname, port } };
      }
    }
  } catch {
    return { error: 'Invalid HMR endpoint pointer file' };
  }
  return { target: socketPath };
}

/**
 * Sends an HMR IPC message to the running native Rust engine over a local Unix domain socket
 * or a loopback TCP pointer file (Windows).
 * Gracefully resolves with `{ connected: false }` if the engine is not currently running.
 */
export function sendHmrMessage(socketPath, message, { timeoutMs = 1500 } = {}) {
  return new Promise((resolve) => {
    if (!existsSync(socketPath)) {
      resolve({ connected: false, error: 'Socket file not found' });
      return;
    }

    const resolvedEndpoint = resolveHmrTarget(socketPath);
    if (resolvedEndpoint.error) {
      resolve({ connected: false, error: resolvedEndpoint.error });
      return;
    }

    let client;
    try {
      client = net.createConnection(resolvedEndpoint.target);
    } catch (err) {
      resolve({ connected: false, error: err instanceof Error ? err.message : String(err) });
      return;
    }
    let resolved = false;
    let responseData = '';

    const timer = setTimeout(() => {
      if (!resolved) {
        resolved = true;
        client.destroy();
        resolve({ connected: true, timeout: true });
      }
    }, timeoutMs);

    const resolvedToken = resolveHmrToken(socketPath, message?.token);
    const wireMessage = resolvedToken ? { ...message, token: resolvedToken } : message;

    client.on('connect', () => {
      client.write(JSON.stringify(wireMessage) + '\n');
    });

    client.on('data', (chunk) => {
      responseData += chunk.toString();
      if (responseData.includes('\n')) {
        const line = responseData.split('\n')[0].trim();
        clearTimeout(timer);
        if (!resolved) {
          resolved = true;
          client.end();
          try {
            resolve({ connected: true, response: JSON.parse(line) });
          } catch {
            resolve({ connected: true, rawResponse: line });
          }
        }
      }
    });

    client.on('error', (err) => {
      clearTimeout(timer);
      if (!resolved) {
        resolved = true;
        resolve({ connected: false, error: err.message });
      }
    });
  });
}

/**
 * Pushes a newly compiled `.ngb` bundle to the live running desktop window.
 */
export async function pushHmrReload(socketPath, bundleScript) {
  return sendHmrMessage(socketPath, { type: 'reload', script: bundleScript });
}

/**
 * Pushes a newly compiled `server.bundle.js` to hot-reload the isolated backend worker.
 */
export async function pushHmrServerReload(socketPath, serverBundleCode) {
  return sendHmrMessage(socketPath, { type: 'reload_server', bundle: serverBundleCode });
}

/**
 * Evaluates arbitrary JavaScript inside the active runtime of the running desktop window.
 */
export async function pushHmrEval(socketPath, jsCode) {
  return sendHmrMessage(socketPath, { type: 'eval', code: jsCode });
}

/**
 * Pings the running desktop engine to check IPC connectivity.
 */
export async function pingHmr(socketPath) {
  return sendHmrMessage(socketPath, { type: 'ping' });
}
