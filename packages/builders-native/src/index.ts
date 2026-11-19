export {
  extractComponentMetadata,
  type ComponentDecoratorMeta,
  type ExtractedComponentMetadata,
  type MethodMutationMeta,
} from './ast-metadata-extractor.mjs';
export {
  compileBundle,
  emitNgbBundle,
  watchBundle,
  type CompileBundleOptions,
  type CompileBundleResult,
} from './build-bundle.mjs';
export {
  analyzeProjectPermissions,
  analyzeSourcePermissions,
  type NativeCapabilitiesObject,
  type PermissionAnalysisManifest,
  type PermissionEvidence,
  type PermissionPolicyMode,
  type PermissionsPolicyConfig,
} from './permission-analyzer.mjs';
export {
  resolveWorkspaceProject,
  type ResolvedWorkspaceProject,
  type ResolveWorkspaceOptions,
  type WorkspaceKind,
} from './workspace-resolver.mjs';
export {
  ENGINE_BINARY_PACKAGES,
  resolveEngineBinaryPath,
  type EngineBinaryPackageMeta,
  type EngineBinaryResolution,
  type EngineBinarySource,
  type ResolveEngineBinaryOptions,
} from './resolve-engine-binary.mjs';
export {
  createNativeTargetConfig,
  executeNativeBuilder,
  resolveNativeBuilderConfig,
  type LinuxPackageFormat,
  type NativeArchKind,
  type NativeBuilderOptions,
  type NativeBuilderResult,
  type NativeCpuProfile,
  type NativeJsEngineKind,
  type NativeOsKind,
  type NativePackageFormat,
  type NativePermissionsMode,
  type NativePermissionsOptions,
  type NativePlatformTarget,
  type ResolvedNativeBuilderConfig,
} from './builder';
export {
  pingHmr,
  pushHmrEval,
  pushHmrReload,
  sendHmrMessage,
  type HmrClientResult,
} from './hmr-client.mjs';
export { buildMultiResolutionIcons } from './icon-compiler.mjs';
export { emitStyleInstruction, parseStylesCss, type StyleRuleIr } from './style-compiler.mjs';
export {
  applyPipeTransform,
  parseTemplateHtml,
  type ComponentDefinitionIr,
  type ReactiveBindingIr,
  type TemplateAstNode,
  type TemplateIdState,
} from './template-compiler.mjs';
