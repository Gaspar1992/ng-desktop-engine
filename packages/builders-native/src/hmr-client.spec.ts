import { unlinkSync, writeFileSync } from 'node:fs';
import net from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pingHmr, pushHmrEval, pushHmrReload, sendHmrMessage } from './hmr-client.mjs';

describe('HMR IPC Client (Node.js)', () => {
  const testSocket = join(tmpdir(), `ng-native-hmr-vitest-${process.pid}.sock`);
  let server: net.Server;

  beforeAll(async () => {
    try {
      unlinkSync(testSocket);
    } catch {
      // Ignore if socket file doesn't exist
    }

    server = net.createServer((conn: net.Socket) => {
      conn.on('data', (data: Buffer) => {
        const line = data.toString().trim();
        const msg = JSON.parse(line);
        if (msg.type === 'ping') {
          conn.write(JSON.stringify({ status: 'ok', pong: true }) + '\n');
        } else if (msg.type === 'reload') {
          conn.write(
            JSON.stringify({ status: 'ok', reloaded: true, length: msg.script.length }) + '\n',
          );
        } else if (msg.type === 'eval') {
          conn.write(JSON.stringify({ status: 'ok', result: `eval:${msg.code}` }) + '\n');
        }
      });
    });

    if (process.platform === 'win32') {
      await new Promise<void>((resolve) => {
        server.listen(0, '127.0.0.1', () => {
          const addr = server.address() as net.AddressInfo;
          writeFileSync(testSocket, `tcp://127.0.0.1:${addr.port}\n`, 'utf8');
          resolve();
        });
      });
    } else {
      await new Promise<void>((resolve) => {
        server.listen(testSocket, () => resolve());
      });
    }
  });

  afterAll(async () => {
    await new Promise<void>((resolve) => {
      server.close(() => {
        try {
          unlinkSync(testSocket);
        } catch {
          // Ignore
        }
        resolve();
      });
    });
  });

  it('pings running server and parses pong response', async () => {
    const res = await pingHmr(testSocket);
    expect(res.connected).toBe(true);
    expect(res.response).toEqual({ status: 'ok', pong: true });
  });

  it('automatically attaches bearer token from <socketPath>.token sidecar file', async () => {
    const authSocket = join(tmpdir(), `ng-native-hmr-auth-${process.pid}.tcp`);
    const tokenFile = `${authSocket}.token`;
    const authServer = net.createServer((conn: net.Socket) => {
      conn.on('data', (data: Buffer) => {
        const msg = JSON.parse(data.toString().trim());
        if (msg.token === 'test-sidecar-secret') {
          conn.write(JSON.stringify({ status: 'ok', pong: true }) + '\n');
        } else {
          conn.write(JSON.stringify({ status: 'error', message: 'Unauthorized' }) + '\n');
        }
      });
    });

    await new Promise<void>((resolve) => authServer.listen(0, '127.0.0.1', () => resolve()));
    try {
      const addr = authServer.address() as net.AddressInfo;
      writeFileSync(authSocket, `tcp://127.0.0.1:${addr.port}\n`, 'utf8');
      writeFileSync(tokenFile, 'test-sidecar-secret\n', 'utf8');

      const res = await pingHmr(authSocket);
      expect(res.connected).toBe(true);
      expect(res.response).toEqual({ status: 'ok', pong: true });
    } finally {
      await new Promise<void>((resolve) => authServer.close(() => resolve()));
      for (const file of [authSocket, tokenFile]) {
        try {
          unlinkSync(file);
        } catch {
          // Ignore cleanup errors
        }
      }
    }
  });

  it('pushes bundle reload and handles script payload', async () => {
    const res = await pushHmrReload(testSocket, 'signal count 42\ntext win App');
    expect(res.connected).toBe(true);
    expect(res.response).toEqual({ status: 'ok', reloaded: true, length: 28 });
  });

  it('evaluates JS snippets over IPC', async () => {
    const res = await pushHmrEval(testSocket, '__NG_NATIVE__.getTheme()');
    expect(res.connected).toBe(true);
    expect(res.response).toEqual({ status: 'ok', result: 'eval:__NG_NATIVE__.getTheme()' });
  });

  it('gracefully handles missing socket when engine is stopped', async () => {
    const res = await sendHmrMessage(join(tmpdir(), `nonexistent-engine-${process.pid}.sock`), {
      type: 'ping',
    });
    expect(res.connected).toBe(false);
    expect(res.error).toBe('Socket file not found');
  });

  it('connects over loopback tcp:// pointer file for Windows HMR IPC', async () => {
    const pointerPath = join(tmpdir(), `ng-native-hmr-win-${process.pid}.tcp`);
    const tcpServer = net.createServer((conn: net.Socket) => {
      conn.on('data', (data: Buffer) => {
        const msg = JSON.parse(data.toString().trim());
        if (msg.type === 'ping') {
          conn.write(JSON.stringify({ status: 'ok', pong: true }) + '\n');
        } else if (msg.type === 'reload') {
          conn.write(
            JSON.stringify({ status: 'ok', reloaded: true, length: msg.script.length }) + '\n',
          );
        }
      });
    });

    await new Promise<void>((resolve) => {
      tcpServer.listen(0, '127.0.0.1', () => resolve());
    });

    try {
      const addr = tcpServer.address() as net.AddressInfo;
      writeFileSync(pointerPath, `tcp://127.0.0.1:${addr.port}\n`, 'utf8');

      const pong = await pingHmr(pointerPath);
      expect(pong.connected).toBe(true);
      expect(pong.response).toEqual({ status: 'ok', pong: true });

      const reload = await pushHmrReload(pointerPath, 'signal x 1');
      expect(reload.connected).toBe(true);
      expect(reload.response).toEqual({ status: 'ok', reloaded: true, length: 10 });

      writeFileSync(pointerPath, 'tcp://127.0.0.1\n', 'utf8');
      const invalidPort = await pingHmr(pointerPath);
      expect(invalidPort.connected).toBe(false);
      expect(invalidPort.error).toContain('Invalid TCP HMR pointer');
    } finally {
      await new Promise<void>((resolve) => tcpServer.close(() => resolve()));
      try {
        unlinkSync(pointerPath);
      } catch {
        // Ignore cleanup failures
      }
    }
  });
});
