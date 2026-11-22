import { createArchitectBuilder, runNativeApplicationBuilder } from './application-builder.mjs';
import { watchBundle } from './build-bundle.mjs';

export async function runNativeDevServerBuilder(options = {}, context = {}) {
  const initial = await runNativeApplicationBuilder(options, context);
  const watching = Boolean(options?.watch);
  if (watching) {
    const workspaceRoot = context?.workspaceRoot || options?.cwd || process.cwd();
    const watcher = watchBundle({
      cwd: workspaceRoot,
      configPath: options?.configPath,
      projectName: initial.projectName,
      version: options?.version,
    });
    context?.addTeardown?.(() => watcher.close());
  }
  return {
    ...initial,
    watching,
  };
}

export const devServerBuilder = createArchitectBuilder(runNativeDevServerBuilder);
export default devServerBuilder;
