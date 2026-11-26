/**
 * Pure schematic tree helper usable by `ng add @ng-desktop/builders` or direct unit verification.
 */
export function applyNgAddToTree(tree, options = {}) {
  const targetPlatform = options.targetPlatform || 'linux-gtk4';
  const permissionsMode = options.permissionsMode || 'auto';

  if (tree.exists('/package.json')) {
    const pkgRaw = tree.read('/package.json')?.toString('utf8') ?? '{}';
    const pkg = JSON.parse(pkgRaw);
    pkg.dependencies ??= {};
    pkg.dependencies['@ng-desktop/platform'] ??= '^0.7.0';
    pkg.devDependencies ??= {};
    pkg.devDependencies['@ng-desktop/builders'] ??= '^0.7.0';
    tree.overwrite('/package.json', `${JSON.stringify(pkg, null, 2)}\n`);
  }

  const builderOptions = {
    targetPlatform,
    permissions: {
      mode: permissionsMode,
    },
  };

  if (tree.exists('/angular.json')) {
    const raw = JSON.parse(tree.read('/angular.json').toString('utf8'));
    const projectName = options.project || raw.defaultProject || Object.keys(raw.projects ?? {})[0];
    if (projectName && raw.projects?.[projectName]) {
      raw.projects[projectName].architect ??= {};
      raw.projects[projectName].architect.native ??= {
        builder: '@ng-desktop/builders:application',
        options: builderOptions,
      };
      tree.overwrite('/angular.json', `${JSON.stringify(raw, null, 2)}\n`);
    }
    return tree;
  }

  const nxProjectPath = options.project ? `/apps/${options.project}/project.json` : '/project.json';
  const fallbackPath = tree.exists(nxProjectPath) ? nxProjectPath : '/project.json';
  if (tree.exists(fallbackPath)) {
    const projectJson = JSON.parse(tree.read(fallbackPath).toString('utf8'));
    projectJson.targets ??= {};
    projectJson.targets.native ??= {
      executor: '@ng-desktop/builders:application',
      options: builderOptions,
    };
    tree.overwrite(fallbackPath, `${JSON.stringify(projectJson, null, 2)}\n`);
  }

  return tree;
}

export default function ngAddSchematic(options = {}) {
  return (tree) => applyNgAddToTree(tree, options);
}
