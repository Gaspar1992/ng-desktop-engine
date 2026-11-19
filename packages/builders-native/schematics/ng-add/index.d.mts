export interface SchematicTreeLike {
  exists(path: string): boolean;
  read(path: string): Buffer | string | null;
  overwrite(path: string, content: string): void;
}

export interface NgAddSchematicOptions {
  project?: string;
  targetPlatform?: 'linux-gtk4' | 'windows-winui' | 'macos-appkit';
  permissionsMode?: 'auto' | 'strict';
}

export function applyNgAddToTree<T extends SchematicTreeLike>(
  tree: T,
  options?: NgAddSchematicOptions,
): T;

export default function ngAddSchematic(
  options?: NgAddSchematicOptions,
): <T extends SchematicTreeLike>(tree: T) => T;
