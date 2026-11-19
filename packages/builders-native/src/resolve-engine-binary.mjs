import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';

export const ENGINE_BINARY_PACKAGES = {
  'linux-x64': {
    pkgName: '@ng-desktop/engine-linux-x64-gnu',
    dirName: 'engine-linux-x64-gnu',
    os: 'linux',
    cpu: 'x64',
    libc: 'glibc',
    binaryName: 'ng-native-engine',
  },
  'linux-arm64': {
    pkgName: '@ng-desktop/engine-linux-arm64-gnu',
    dirName: 'engine-linux-arm64-gnu',
    os: 'linux',
    cpu: 'arm64',
    libc: 'glibc',
    binaryName: 'ng-native-engine',
  },
  'win32-x64': {
    pkgName: '@ng-desktop/engine-win32-x64-msvc',
    dirName: 'engine-win32-x64-msvc',
    os: 'win32',
    cpu: 'x64',
    binaryName: 'ng-native-engine.exe',
  },
  'darwin-arm64': {
    pkgName: '@ng-desktop/engine-darwin-arm64',
    dirName: 'engine-darwin-arm64',
    os: 'darwin',
    cpu: 'arm64',
    binaryName: 'ng-native-engine',
  },
  'darwin-x64': {
    pkgName: '@ng-desktop/engine-darwin-x64',
    dirName: 'engine-darwin-x64',
    os: 'darwin',
    cpu: 'x64',
    binaryName: 'ng-native-engine',
  },
};

function defaultResolvePkgJsonPath(pkgName, cwd) {
  try {
    const req = createRequire(resolve(cwd, 'package.json'));
    return req.resolve(`${pkgName}/package.json`);
  } catch {
    return null;
  }
}

/**
 * Resolves the prebuilt `ng-native-engine` host executable in priority order:
 *   1. Explicit `NG_DESKTOP_ENGINE_BIN` / `NG_NATIVE_ENGINE_BIN` environment override
 *   2. Installed platform-specific `@ng-desktop/engine-*` npm `optionalDependencies` package
 *   3. Local Cargo build artifact (`target/release` then `target/debug`)
 *   4. Fallback to `cargo run --` for source-checkout development
 */
export function resolveEngineBinaryPath({
  env = process.env,
  platform = process.platform,
  arch = process.arch,
  cwd = process.cwd(),
  workspaceRoot,
  resolvePkgPath,
  fileExists = existsSync,
} = {}) {
  const targetKey = `${platform}-${arch}`;
  const entry = ENGINE_BINARY_PACKAGES[targetKey] ?? null;
  const fallbackBinName = platform === 'win32' ? 'ng-native-engine.exe' : 'ng-native-engine';
  const binName = entry?.binaryName ?? fallbackBinName;

  const envOverride =
    env?.['NG_DESKTOP_ENGINE_BIN']?.trim() || env?.['NG_NATIVE_ENGINE_BIN']?.trim();
  if (envOverride) {
    return {
      mode: 'binary',
      command: envOverride,
      args: [],
      source: 'env',
      packageName: entry?.pkgName ?? null,
      targetKey,
    };
  }

  if (entry) {
    const pkgJsonPath = resolvePkgPath
      ? resolvePkgPath(entry.pkgName)
      : defaultResolvePkgJsonPath(entry.pkgName, cwd);
    if (pkgJsonPath) {
      const candidateBin = resolve(dirname(pkgJsonPath), 'bin', entry.binaryName);
      if (fileExists(candidateBin)) {
        return {
          mode: 'binary',
          command: candidateBin,
          args: [],
          source: 'optional-package',
          packageName: entry.pkgName,
          targetKey,
        };
      }
    }
  }

  const searchRoot = workspaceRoot ?? cwd;
  for (const profile of ['release', 'debug']) {
    const localBin = resolve(searchRoot, 'target', profile, binName);
    if (fileExists(localBin)) {
      return {
        mode: 'binary',
        command: localBin,
        args: [],
        source: 'cargo-target',
        packageName: entry?.pkgName ?? null,
        targetKey,
      };
    }
  }

  return {
    mode: 'cargo',
    command: 'cargo',
    args: ['run', '--'],
    source: 'cargo-fallback',
    packageName: entry?.pkgName ?? null,
    targetKey,
  };
}
