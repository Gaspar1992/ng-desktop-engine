import { FSWatcher } from 'node:fs';
import { PermissionAnalysisManifest } from './permission-analyzer.mjs';
import { StyleRuleIr } from './style-compiler.mjs';
import { TemplateAstNode } from './template-compiler.mjs';

export interface CompileBundleOptions {
  rebuildIcons?: boolean;
  cwd?: string;
  configPath?: string;
  projectName?: string;
  version?: string;
}

export interface CompileBundleResult {
  projectName: string;
  version: string;
  sourceRoot: string;
  bundleFile: string;
  permissionsFile: string;
  runtimeConfigFile: string;
  permissions: PermissionAnalysisManifest;
  socketPath: string;
}

export function emitNgbBundle(
  styleRules: StyleRuleIr[],
  numericSignals: Record<string, number>,
  methods: Record<string, { kind: 'set' | 'add'; signalName: string; value: number }>,
  astRoots: TemplateAstNode[],
  iconMeta: { appId: string; relPath: string } | null,
  sourceLabel: string,
): string;

export function compileBundle(options?: CompileBundleOptions): CompileBundleResult;

export function watchBundle(options?: CompileBundleOptions): FSWatcher;
