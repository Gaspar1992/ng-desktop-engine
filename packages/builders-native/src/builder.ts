export type NativeOsKind = 'auto' | 'linux' | 'windows' | 'macos';
export type NativeArchKind = 'auto' | 'x86_64' | 'aarch64' | 'universal';
export type NativeCpuProfile = 'auto' | 'native' | 'baseline' | 'avx2' | 'neon';
export type NativePlatformTarget = 'linux-gtk4' | 'windows-winui' | 'macos-appkit';
export type NativeJsEngineKind = 'jsc' | 'quickjs';
export type NativePackageFormat = 'deb' | 'appimage' | 'flatpak' | 'msi' | 'dmg';
export type LinuxPackageFormat = 'deb' | 'appimage' | 'flatpak';
export type NativePermissionsMode = 'auto' | 'strict';

export interface NativePermissionsOptions {
  mode?: NativePermissionsMode;
  allow?: string[];
  deny?: string[];
}

export interface NativeBuilderOptions {
  os?: NativeOsKind;
  arch?: NativeArchKind;
  cpu?: NativeCpuProfile;
  targetPlatform?: NativePlatformTarget;
  jsEngine?: NativeJsEngineKind;
  outputPath?: string;
  version?: string;
  permissions?: NativePermissionsOptions;
  packageFormats?: NativePackageFormat[];
}

export interface ResolvedNativeBuilderConfig {
  os: 'linux' | 'windows' | 'macos';
  arch: 'x86_64' | 'aarch64' | 'universal';
  cpu: NativeCpuProfile;
  targetPlatform: NativePlatformTarget;
  jsEngine: NativeJsEngineKind;
  outputPath: string;
  rustTargetTriple: string;
  packageFormats: NativePackageFormat[];
}

export interface NativeBuilderResult {
  success: boolean;
  projectName: string;
  os: 'linux' | 'windows' | 'macos';
  arch: 'x86_64' | 'aarch64' | 'universal';
  cpu: NativeCpuProfile;
  rustTargetTriple: string;
  targetPlatform: NativePlatformTarget;
  jsEngine: NativeJsEngineKind;
  bundlePath: string;
  permissionsManifestPath: string;
  runtimeConfigPath: string;
  packageArtifacts: string[];
}

function detectHostOs(): 'linux' | 'windows' | 'macos' {
  if (typeof process !== 'undefined') {
    if (process.platform === 'win32') return 'windows';
    if (process.platform === 'darwin') return 'macos';
  }
  return 'linux';
}

function detectHostArch(): 'x86_64' | 'aarch64' {
  if (typeof process !== 'undefined' && process.arch === 'arm64') {
    return 'aarch64';
  }
  return 'x86_64';
}

function resolveTargetTriple(
  os: 'linux' | 'windows' | 'macos',
  arch: 'x86_64' | 'aarch64' | 'universal',
): string {
  if (os === 'windows') {
    return arch === 'aarch64' ? 'aarch64-pc-windows-msvc' : 'x86_64-pc-windows-msvc';
  }
  if (os === 'macos') {
    if (arch === 'universal') return 'universal-apple-darwin';
    return arch === 'x86_64' ? 'x86_64-apple-darwin' : 'aarch64-apple-darwin';
  }
  return arch === 'aarch64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
}

/**
 * Resolves minimal `angular.json` options (`os`, `arch`, `cpu`) into full target platform,
 * embedded JS engine, Rust target triple, and output directory defaults.
 */
export function resolveNativeBuilderConfig(
  projectName: string,
  options: NativeBuilderOptions = {},
): ResolvedNativeBuilderConfig {
  let os: 'linux' | 'windows' | 'macos';
  if (options.os && options.os !== 'auto') {
    os = options.os;
  } else if (options.targetPlatform === 'windows-winui') {
    os = 'windows';
  } else if (options.targetPlatform === 'macos-appkit') {
    os = 'macos';
  } else if (options.targetPlatform === 'linux-gtk4') {
    os = 'linux';
  } else {
    os = detectHostOs();
  }

  const targetPlatform: NativePlatformTarget =
    options.targetPlatform ??
    (os === 'windows' ? 'windows-winui' : os === 'macos' ? 'macos-appkit' : 'linux-gtk4');

  const jsEngine: NativeJsEngineKind = options.jsEngine ?? (os === 'windows' ? 'quickjs' : 'jsc');

  const arch: 'x86_64' | 'aarch64' | 'universal' =
    options.arch && options.arch !== 'auto' ? options.arch : detectHostArch();

  const cpu: NativeCpuProfile = options.cpu ?? 'auto';
  const outputPath = (options.outputPath ?? `dist/${projectName}`).replace(/\/+$/, '');
  const defaultFormats: NativePackageFormat[] =
    os === 'linux' ? ['deb', 'appimage', 'flatpak'] : os === 'windows' ? ['msi'] : ['dmg'];

  return {
    os,
    arch,
    cpu,
    targetPlatform,
    jsEngine,
    outputPath,
    rustTargetTriple: resolveTargetTriple(os, arch),
    packageFormats: options.packageFormats ?? defaultFormats,
  };
}

/**
 * Generates an `angular.json` `architect.native` target configuration block
 * for `@ng-native/builders:application`.
 */
export function createNativeTargetConfig(options: NativeBuilderOptions): {
  builder: '@ng-native/builders:application';
  options: NativeBuilderOptions;
} {
  return {
    builder: '@ng-native/builders:application',
    options: {
      os: options.os ?? 'linux',
      arch: options.arch ?? 'x86_64',
      ...(options.cpu ? { cpu: options.cpu } : {}),
      ...(options.targetPlatform ? { targetPlatform: options.targetPlatform } : {}),
      ...(options.jsEngine ? { jsEngine: options.jsEngine } : {}),
      ...(options.outputPath ? { outputPath: options.outputPath } : {}),
      ...(options.packageFormats ? { packageFormats: options.packageFormats } : {}),
    },
  };
}

/**
 * Executes the Angular 22 Native Builder pipeline, resolving minimal OS/CPU config,
 * `.ngb` bundle path, and target distribution artifacts.
 */
export function executeNativeBuilder(
  projectName: string,
  options: NativeBuilderOptions,
): NativeBuilderResult {
  const resolved = resolveNativeBuilderConfig(projectName, options);
  const packageArtifacts: string[] = [];

  for (const fmt of resolved.packageFormats) {
    if (fmt === 'deb') {
      packageArtifacts.push(`${resolved.outputPath}/packages/deb/DEBIAN/control`);
      packageArtifacts.push(
        `${resolved.outputPath}/packages/deb/usr/share/applications/${projectName}.desktop`,
      );
    } else if (fmt === 'appimage') {
      packageArtifacts.push(`${resolved.outputPath}/packages/appimage/AppRun`);
    } else if (fmt === 'flatpak') {
      packageArtifacts.push(`${resolved.outputPath}/packages/flatpak/${projectName}.yml`);
    } else if (fmt === 'msi') {
      packageArtifacts.push(`${resolved.outputPath}/packages/windows/${projectName}.msi`);
    } else if (fmt === 'dmg') {
      packageArtifacts.push(`${resolved.outputPath}/packages/macos/${projectName}.dmg`);
    }
  }

  return {
    success: true,
    projectName,
    os: resolved.os,
    arch: resolved.arch,
    cpu: resolved.cpu,
    rustTargetTriple: resolved.rustTargetTriple,
    targetPlatform: resolved.targetPlatform,
    jsEngine: resolved.jsEngine,
    bundlePath: `${resolved.outputPath}/main.ngb`,
    permissionsManifestPath: `${resolved.outputPath}/permissions.manifest.json`,
    runtimeConfigPath: `${resolved.outputPath}/ng-desktop.runtime.json`,
    packageArtifacts,
  };
}
