import { FSWatcher } from 'node:fs';
import { StyleRuleIr } from './style-compiler.mjs';
import { TemplateAstNode } from './template-compiler.mjs';

export interface CompileBundleResult {
  projectName: string;
  sourceRoot: string;
  bundleFile: string;
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

export function compileBundle(options?: { rebuildIcons?: boolean }): CompileBundleResult;

export function watchBundle(): FSWatcher;
