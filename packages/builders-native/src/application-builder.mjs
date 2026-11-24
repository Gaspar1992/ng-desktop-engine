import { createRequire } from 'node:module';
import { compileBundle } from './build-bundle.mjs';

export const ARCHITECT_BUILDER_SYMBOL = Symbol.for('@angular-devkit/architect:builder');
export const ARCHITECT_VERSION_SYMBOL = Symbol.for('@angular-devkit/architect:builder-version');

export function createArchitectBuilder(handlerFn) {
  let builderObj = null;
  try {
    const req = createRequire(import.meta.url);
    const architect = req('@angular-devkit/architect');
    if (typeof architect?.createBuilder === 'function') {
      builderObj = architect.createBuilder(handlerFn);
    }
  } catch {
    // Fallback below if @angular-devkit/architect is not installed in minimal environments
  }
  if (!builderObj) {
    builderObj = {
      handler: handlerFn,
    };
  }
  builderObj[ARCHITECT_BUILDER_SYMBOL] = true;
  builderObj[ARCHITECT_VERSION_SYMBOL] = builderObj[ARCHITECT_VERSION_SYMBOL] ?? '0.4.1';
  builderObj.run = handlerFn;
  return builderObj;
}

export async function runNativeApplicationBuilder(options = {}, context = {}) {
  const workspaceRoot = context?.workspaceRoot || options?.cwd || process.cwd();
  const projectName = context?.target?.project || options?.projectName;
  const compiled = compileBundle({
    cwd: workspaceRoot,
    configPath: options?.configPath,
    projectName,
    version: options?.version,
    rebuildIcons: options?.rebuildIcons ?? true,
  });

  context?.logger?.info?.(
    `[NgDesktop Architect] Built ${compiled.projectName} v${compiled.version} -> ${compiled.bundleFile}`,
  );

  return {
    success: true,
    projectName: compiled.projectName,
    version: compiled.version,
    bundleFile: compiled.bundleFile,
    ...(compiled.entryBundleFile ? { entryBundleFile: compiled.entryBundleFile } : {}),
    ...(compiled.serverBundleFile ? { serverBundleFile: compiled.serverBundleFile } : {}),
    permissionsFile: compiled.permissionsFile,
    runtimeConfigFile: compiled.runtimeConfigFile,
    hmrDelta: compiled.hmrDelta,
  };
}

export const applicationBuilder = createArchitectBuilder(runNativeApplicationBuilder);
export default applicationBuilder;
