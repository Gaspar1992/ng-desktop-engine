import { Observable } from 'rxjs';
import { createArchitectBuilder, runNativeApplicationBuilder } from './application-builder.mjs';
import { watchBundle } from './build-bundle.mjs';

export function runNativeDevServerBuilder(options = {}, context = {}) {
  const watching = Boolean(options?.watch);
  if (!watching) {
    return runNativeApplicationBuilder(options, context).then((initial) => ({
      ...initial,
      watching: false,
    }));
  }

  return new Observable((subscriber) => {
    const workspaceRoot = context?.workspaceRoot || options?.cwd || process.cwd();
    let watcher = null;
    try {
      watcher = watchBundle({
        cwd: workspaceRoot,
        configPath: options?.configPath,
        projectName: options?.projectName,
        version: options?.version,
      });
      context?.addTeardown?.(() => watcher?.close());
      subscriber.next({
        success: true,
        watching: true,
      });
    } catch (err) {
      subscriber.error(err);
    }
    return () => {
      watcher?.close?.();
    };
  });
}

export const devServerBuilder = createArchitectBuilder(runNativeDevServerBuilder);
export default devServerBuilder;
