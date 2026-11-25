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

export interface HmrDeltaSummary {
  ngbChanged: boolean;
  appBundleChanged: boolean;
  serverBundleChanged: boolean;
}

export interface CompileBundleResult {
  projectName: string;
  version: string;
  sourceRoot: string;
  bundleFile: string;
  entryBundleFile?: string;
  serverBundleFile?: string;
  permissionsFile: string;
  runtimeConfigFile: string;
  permissions: PermissionAnalysisManifest;
  socketPath: string;
  hmrDelta: HmrDeltaSummary;
}

/** Emits textual `.ngb` instructions from compiled style rules, signals, methods, and widget AST. */
export function emitNgbBundle(
  styleRules: StyleRuleIr[],
  numericSignals: Record<string, number>,
  methods: Record<string, { kind: 'set' | 'add'; signalName: string; value: number }>,
  astRoots: TemplateAstNode[],
  iconMeta: { appId: string; relPath: string } | null,
  sourceLabel: string,
): string;

/** Compiles an Angular 22 workspace project into `main.ngb`, `app.bundle.js`, `server.bundle.js`, and manifests. */
export function compileBundle(options?: CompileBundleOptions): CompileBundleResult;

/** Watches application sources and pushes incremental HMR reloads over Unix socket / loopback TCP IPC. */
export function watchBundle(options?: CompileBundleOptions): FSWatcher;
