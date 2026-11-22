import { HmrDeltaSummary } from './build-bundle.mjs';

export const ARCHITECT_BUILDER_SYMBOL: unique symbol;
export const ARCHITECT_VERSION_SYMBOL: unique symbol;

export interface ArchitectBuilderContextLike {
  workspaceRoot?: string;
  target?: { project?: string; target?: string };
  logger?: { info?: (msg: string) => void; error?: (msg: string) => void };
  addTeardown?: (teardown: () => void | Promise<void>) => void;
}

export interface NativeApplicationBuilderOptions {
  cwd?: string;
  configPath?: string;
  projectName?: string;
  version?: string;
  rebuildIcons?: boolean;
  os?: string;
  arch?: string;
  cpu?: string;
  targetPlatform?: string;
  jsEngine?: string;
  outputPath?: string;
  main?: string;
  permissions?: { mode?: 'auto' | 'strict'; allow?: string[]; deny?: string[] };
  packageFormats?: string[];
}

export interface NativeApplicationBuilderOutput {
  success: boolean;
  projectName: string;
  version: string;
  bundleFile: string;
  entryBundleFile?: string;
  serverBundleFile?: string;
  permissionsFile: string;
  runtimeConfigFile: string;
  hmrDelta: HmrDeltaSummary;
}

export interface ArchitectBuilderObject<TOpt, TOut> {
  handler: unknown;
  run: (options?: TOpt, context?: ArchitectBuilderContextLike) => Promise<TOut>;
  [key: symbol]: unknown;
}

export function createArchitectBuilder<TOpt, TOut>(
  handlerFn: (options?: TOpt, context?: ArchitectBuilderContextLike) => Promise<TOut>,
): ArchitectBuilderObject<TOpt, TOut>;

export function runNativeApplicationBuilder(
  options?: NativeApplicationBuilderOptions,
  context?: ArchitectBuilderContextLike,
): Promise<NativeApplicationBuilderOutput>;

export const applicationBuilder: ArchitectBuilderObject<
  NativeApplicationBuilderOptions,
  NativeApplicationBuilderOutput
>;

export default applicationBuilder;
