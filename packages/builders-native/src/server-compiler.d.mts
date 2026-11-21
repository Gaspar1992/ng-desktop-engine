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

export function discoverServerRoutes(sourceRootAbs: string): DiscoveredServerRoute[];

export function compileServerBundle(options: CompileServerBundleOptions): string | undefined;
