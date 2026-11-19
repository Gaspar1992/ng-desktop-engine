export interface EngineBinaryPackageMeta {
  readonly pkgName: string;
  readonly dirName: string;
  readonly os: 'linux' | 'win32' | 'darwin';
  readonly cpu: 'x64' | 'arm64';
  readonly libc?: 'glibc';
  readonly binaryName: string;
}

export const ENGINE_BINARY_PACKAGES: Readonly<Record<string, EngineBinaryPackageMeta>>;

export type EngineBinarySource = 'env' | 'optional-package' | 'cargo-target' | 'cargo-fallback';

export interface ResolveEngineBinaryOptions {
  env?: Record<string, string | undefined>;
  platform?: NodeJS.Platform;
  arch?: string;
  cwd?: string;
  workspaceRoot?: string;
  resolvePkgPath?: (pkgName: string) => string | null | undefined;
  fileExists?: (path: string) => boolean;
}

export interface EngineBinaryResolution {
  mode: 'binary' | 'cargo';
  command: string;
  args: string[];
  source: EngineBinarySource;
  packageName: string | null;
  targetKey: string;
}

export function resolveEngineBinaryPath(
  options?: ResolveEngineBinaryOptions,
): EngineBinaryResolution;
