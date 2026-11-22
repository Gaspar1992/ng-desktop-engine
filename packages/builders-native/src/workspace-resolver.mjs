import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

function readJsonSafe(filePath) {
  return JSON.parse(readFileSync(filePath, 'utf8'));
}

function readPackageVersion(workspaceRoot, projectRoot) {
  const candidates = [];
  if (projectRoot && projectRoot !== '.') {
    candidates.push(resolve(workspaceRoot, projectRoot, 'package.json'));
  }
  candidates.push(resolve(workspaceRoot, 'package.json'));
  for (const pkgPath of candidates) {
    if (existsSync(pkgPath)) {
      try {
        const pkg = readJsonSafe(pkgPath);
        if (typeof pkg.version === 'string' && pkg.version.trim()) {
          return pkg.version.trim();
        }
      } catch {
        // Ignore malformed package.json and continue fallback chain
      }
    }
  }
  return '0.4.0';
}

function findNxProjectJsonFiles(workspaceRoot) {
  const found = [];
  const rootProjectJson = resolve(workspaceRoot, 'project.json');
  if (existsSync(rootProjectJson)) {
    found.push(rootProjectJson);
  }
  for (const container of ['apps', 'projects', 'packages', 'examples']) {
    const containerDir = resolve(workspaceRoot, container);
    if (!existsSync(containerDir)) continue;
    for (const entry of readdirSync(containerDir)) {
      const candidateDir = resolve(containerDir, entry);
      if (!statSync(candidateDir).isDirectory()) continue;
      const projectJson = resolve(candidateDir, 'project.json');
      if (existsSync(projectJson)) {
        found.push(projectJson);
      }
    }
  }
  return found;
}

function resolveFromAngularJson(workspaceRoot, configFile, angularJson, requestedProject) {
  const projects = angularJson.projects ?? {};
  const projectKeys = Object.keys(projects);
  if (projectKeys.length === 0) {
    throw new Error(`No projects defined in '${configFile}'`);
  }
  const projectName =
    requestedProject && projects[requestedProject]
      ? requestedProject
      : angularJson.defaultProject && projects[angularJson.defaultProject]
        ? angularJson.defaultProject
        : projectKeys[0];

  const projectCfg = projects[projectName];
  if (!projectCfg) {
    throw new Error(`Project '${projectName}' not found in '${configFile}'`);
  }
  const projectRoot = projectCfg.root ?? '.';
  const sourceRoot = projectCfg.sourceRoot ?? (projectRoot === '.' ? 'src' : `${projectRoot}/src`);
  const buildOptions = projectCfg.architect?.build?.options ?? {};
  const nativeOptions = projectCfg.architect?.native?.options ?? {};
  const outputPath = nativeOptions.outputPath || buildOptions.outputPath || `dist/${projectName}`;
  const version = nativeOptions.version || readPackageVersion(workspaceRoot, projectRoot);

  return {
    workspaceKind: 'angular',
    workspaceRoot,
    configFile,
    projectName,
    projectRoot,
    sourceRoot,
    outputPath,
    version,
    buildOptions,
    nativeOptions,
  };
}

function resolveFromNxProjectJson(workspaceRoot, configFile, projectJson) {
  const projectName = projectJson.name;
  if (!projectName || typeof projectName !== 'string') {
    throw new Error(`Missing 'name' in Nx project config '${configFile}'`);
  }
  const relDir = relative(workspaceRoot, dirname(configFile));
  const projectRoot = relDir === '' ? '.' : relDir;
  const sourceRoot = projectJson.sourceRoot ?? (projectRoot === '.' ? 'src' : `${projectRoot}/src`);
  const buildOptions = projectJson.targets?.build?.options ?? {};
  const nativeOptions = projectJson.targets?.native?.options ?? {};
  const outputPath =
    nativeOptions.outputPath ||
    buildOptions.outputPath ||
    (projectRoot === '.' ? `dist/${projectName}` : `dist/${projectRoot}`);
  const version = nativeOptions.version || readPackageVersion(workspaceRoot, projectRoot);

  return {
    workspaceKind: 'nx',
    workspaceRoot,
    configFile,
    projectName,
    projectRoot,
    sourceRoot,
    outputPath,
    version,
    buildOptions,
    nativeOptions,
  };
}

function resolveFromRuntimeManifest(workspaceRoot, configFile, runtimeJson) {
  const projectName = runtimeJson.projectName;
  if (!projectName || typeof projectName !== 'string') {
    throw new Error(`Missing 'projectName' in runtime manifest '${configFile}'`);
  }
  const outputPath = runtimeJson.outputPath || `dist/${projectName}`;
  const version = runtimeJson.version || readPackageVersion(workspaceRoot, '.');
  return {
    workspaceKind: 'runtime',
    workspaceRoot,
    configFile,
    projectName,
    projectRoot: runtimeJson.projectRoot || '.',
    sourceRoot: runtimeJson.sourceRoot || 'src',
    outputPath,
    version,
    buildOptions: {},
    nativeOptions: {
      targetPlatform: runtimeJson.targetPlatform,
      outputPath,
      version,
      permissions: runtimeJson.permissions,
    },
  };
}

/**
 * Resolves workspace configuration across Angular CLI (`angular.json`), Nx (`project.json` / `nx.json`),
 * and serialized builder runtime manifests (`ng-desktop.runtime.json`) without hardcoded literals.
 */
export function resolveWorkspaceProject({ cwd = process.cwd(), configPath, projectName } = {}) {
  if (configPath) {
    const absConfig = resolve(cwd, configPath);
    if (!existsSync(absConfig)) {
      throw new Error(`Workspace config file not found: '${absConfig}'`);
    }
    const raw = readJsonSafe(absConfig);
    if (raw.projects && typeof raw.projects === 'object') {
      return resolveFromAngularJson(dirname(absConfig), absConfig, raw, projectName);
    }
    if (raw.projectName && !raw.targets) {
      return resolveFromRuntimeManifest(dirname(absConfig), absConfig, raw);
    }
    return resolveFromNxProjectJson(dirname(absConfig), absConfig, raw);
  }

  const candidateRoots = [cwd, resolve(__dirname, '../../..')];
  for (const root of candidateRoots) {
    const angularFile = resolve(root, 'angular.json');
    if (existsSync(angularFile)) {
      return resolveFromAngularJson(root, angularFile, readJsonSafe(angularFile), projectName);
    }

    const directProjectJson = resolve(root, 'project.json');
    if (existsSync(directProjectJson)) {
      const parsed = readJsonSafe(directProjectJson);
      if (!projectName || parsed.name === projectName) {
        return resolveFromNxProjectJson(root, directProjectJson, parsed);
      }
    }

    if (existsSync(resolve(root, 'nx.json'))) {
      const nxProjects = findNxProjectJsonFiles(root);
      const parsedList = nxProjects.map((file) => ({ file, json: readJsonSafe(file) }));
      const selected =
        (projectName && parsedList.find((item) => item.json.name === projectName)) ||
        parsedList.find((item) => item.json.targets?.native) ||
        parsedList.find((item) => item.json.projectType === 'application') ||
        parsedList[0];
      if (selected) {
        return resolveFromNxProjectJson(root, selected.file, selected.json);
      }
    }
  }

  throw new Error(
    `No Angular ('angular.json') or Nx ('project.json' / 'nx.json') workspace found from '${cwd}'`,
  );
}
