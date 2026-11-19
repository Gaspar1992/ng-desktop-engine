import { spawn } from 'node:child_process';
import { watchBundle } from './build-bundle.mjs';
import { resolveEngineBinaryPath } from './resolve-engine-binary.mjs';
import { resolveWorkspaceProject } from './workspace-resolver.mjs';

const workspace = resolveWorkspaceProject();
const watcher = watchBundle({
  cwd: workspace.workspaceRoot,
  configPath: workspace.configFile,
  projectName: workspace.projectName,
});

const resolvedEngine = resolveEngineBinaryPath({
  cwd: process.cwd(),
  workspaceRoot: workspace.workspaceRoot,
});

const engineArgs = [
  ...resolvedEngine.args,
  `--config=${workspace.configFile}`,
  `--project=${workspace.projectName}`,
];

process.stdout.write(
  `[NgDesktop Dev Server] Launching '${workspace.projectName}' via ${resolvedEngine.source} (${resolvedEngine.command})...\n`,
);
const engineProc = spawn(resolvedEngine.command, engineArgs, {
  cwd: workspace.workspaceRoot,
  stdio: 'inherit',
  env: process.env,
});

function shutdown(code = 0) {
  watcher.close();
  if (!engineProc.killed) {
    engineProc.kill('SIGTERM');
  }
  process.exit(code);
}

engineProc.on('exit', (code) => {
  watcher.close();
  process.exit(code ?? 0);
});

process.on('SIGINT', () => shutdown(0));
process.on('SIGTERM', () => shutdown(0));
