export type PermissionPolicyMode = 'auto' | 'strict';

export interface PermissionsPolicyConfig {
  mode?: PermissionPolicyMode;
  allow?: string[];
  deny?: string[];
}

export interface PermissionEvidence {
  capability: string;
  file: string;
  reason: string;
}

export interface NativeCapabilitiesObject {
  network: boolean;
  sqlite: boolean;
  filesystem: {
    xdg: boolean;
    dialogs: boolean;
    drop: boolean;
  };
  clipboard: boolean;
  notifications: boolean;
  computeThreads: boolean;
  fullSystemAccess: boolean;
}

export interface PermissionAnalysisManifest {
  schemaVersion: 1;
  mode: PermissionPolicyMode;
  capabilities: NativeCapabilitiesObject;
  evidence: PermissionEvidence[];
  warnings: string[];
}

export function analyzeSourcePermissions(
  files: ReadonlyArray<{ file: string; content: string }>,
  permissionsConfig?: PermissionsPolicyConfig,
): PermissionAnalysisManifest;

export function analyzeProjectPermissions(
  sourceRoot: string,
  permissionsConfig?: PermissionsPolicyConfig,
): PermissionAnalysisManifest;
