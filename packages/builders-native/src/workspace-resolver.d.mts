export type WorkspaceKind = 'angular' | 'nx' | 'runtime';

export interface ResolveWorkspaceOptions {
  cwd?: string;
  configPath?: string;
  projectName?: string;
}

export interface ResolvedWorkspaceProject {
  workspaceKind: WorkspaceKind;
  workspaceRoot: string;
  configFile: string;
  projectName: string;
  projectRoot: string;
  sourceRoot: string;
  outputPath: string;
  version: string;
  buildOptions: Record<string, unknown>;
  nativeOptions: Record<string, unknown>;
}

export function resolveWorkspaceProject(
  options?: ResolveWorkspaceOptions,
): ResolvedWorkspaceProject;
