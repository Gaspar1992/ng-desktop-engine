import { ENGINE_BINARY_PACKAGES, resolveEngineBinaryPath } from '@ng-native/builders';

describe('Phase 12 — Prebuilt Engine Binary Resolution (resolveEngineBinaryPath)', () => {
  it('prioritizes explicit NG_DESKTOP_ENGINE_BIN / NG_NATIVE_ENGINE_BIN environment overrides', () => {
    const res = resolveEngineBinaryPath({
      env: { NG_DESKTOP_ENGINE_BIN: '/opt/custom/ng-native-engine' },
      platform: 'linux',
      arch: 'x64',
    });

    expect(res).toEqual({
      mode: 'binary',
      command: '/opt/custom/ng-native-engine',
      args: [],
      source: 'env',
      packageName: '@ng-native/engine-linux-x64-gnu',
      targetKey: 'linux-x64',
    });
  });

  it('resolves prebuilt optional npm packages across Linux, Windows, and macOS', () => {
    const winRes = resolveEngineBinaryPath({
      env: {},
      platform: 'win32',
      arch: 'x64',
      resolvePkgPath: (pkg) =>
        pkg === '@ng-native/engine-win32-x64-msvc'
          ? '/app/node_modules/@ng-native/engine-win32-x64-msvc/package.json'
          : null,
      fileExists: (p) =>
        p.endsWith('node_modules/@ng-native/engine-win32-x64-msvc/bin/ng-native-engine.exe'),
    });

    expect(winRes.mode).toBe('binary');
    expect(winRes.source).toBe('optional-package');
    expect(winRes.packageName).toBe('@ng-native/engine-win32-x64-msvc');
    expect(winRes.command).toContain('ng-native-engine.exe');

    expect(Object.keys(ENGINE_BINARY_PACKAGES)).toEqual([
      'linux-x64',
      'linux-arm64',
      'win32-x64',
      'darwin-arm64',
      'darwin-x64',
    ]);
  });

  it('uses local Cargo target binaries when present and falls back to cargo run otherwise', () => {
    const localRelease = resolveEngineBinaryPath({
      env: {},
      platform: 'darwin',
      arch: 'arm64',
      workspaceRoot: '/repo',
      resolvePkgPath: () => null,
      fileExists: (p) => p === '/repo/target/release/ng-native-engine',
    });
    expect(localRelease.mode).toBe('binary');
    expect(localRelease.source).toBe('cargo-target');
    expect(localRelease.command).toBe('/repo/target/release/ng-native-engine');

    const fallback = resolveEngineBinaryPath({
      env: {},
      platform: 'linux',
      arch: 'arm64',
      workspaceRoot: '/empty-repo',
      resolvePkgPath: () => null,
      fileExists: () => false,
    });
    expect(fallback.mode).toBe('cargo');
    expect(fallback.command).toBe('cargo');
    expect(fallback.args).toEqual(['run', '--']);
    expect(fallback.source).toBe('cargo-fallback');
    expect(fallback.packageName).toBe('@ng-native/engine-linux-arm64-gnu');
  });
});
