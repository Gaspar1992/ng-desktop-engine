export interface DiscoveredServerRoute {
  method: string;
  pattern: string;
  filePath: string;
}

export interface CompileServerBundleOptions {
  workspaceRoot: string;
  sourceRootAbs: string;
  outDir: string;
  projectName: string;
}

export interface EsbuildBuildOptions {
  workspaceRoot: string;
  resolveDir?: string;
  entrySource: string;
  loader?: 'ts' | 'js';
  alias?: Record<string, string>;
  define?: Record<string, string>;
  minifySyntax?: boolean;
  banner?: { js?: string };
}

export function discoverServerRoutes(sourceRootAbs: string): DiscoveredServerRoute[];

export function runEsbuildBuild(options: EsbuildBuildOptions): string;

export function compileServerBundle(options: CompileServerBundleOptions): string | undefined;
