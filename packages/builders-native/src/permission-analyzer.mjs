import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { parseSync } from 'oxc-parser';

function createEmptyCapabilities() {
  return {
    network: false,
    sqlite: false,
    filesystem: {
      xdg: false,
      dialogs: false,
      drop: false,
    },
    clipboard: false,
    notifications: false,
    computeThreads: false,
    fullSystemAccess: false,
  };
}

function normalizeToken(token) {
  return String(token ?? '')
    .trim()
    .toLowerCase()
    .replaceAll('_', '-');
}

function applyCapabilityToken(capabilities, token, enabled) {
  switch (normalizeToken(token)) {
    case 'network':
    case 'http':
      capabilities.network = enabled;
      break;
    case 'sqlite':
    case 'database':
      capabilities.sqlite = enabled;
      break;
    case 'filesystem':
    case 'fs':
      capabilities.filesystem.xdg = enabled;
      capabilities.filesystem.dialogs = enabled;
      capabilities.filesystem.drop = enabled;
      break;
    case 'filesystem:xdg':
    case 'filesystem-xdg':
    case 'filesystemxdg':
    case 'xdg':
      capabilities.filesystem.xdg = enabled;
      break;
    case 'filesystem:dialogs':
    case 'filesystem-dialogs':
    case 'filesystemdialogs':
    case 'dialogs':
      capabilities.filesystem.dialogs = enabled;
      break;
    case 'filesystem:drop':
    case 'filesystem-drop':
    case 'filesystemdrop':
    case 'drop':
      capabilities.filesystem.drop = enabled;
      break;
    case 'clipboard':
      capabilities.clipboard = enabled;
      break;
    case 'notifications':
    case 'notification':
      capabilities.notifications = enabled;
      break;
    case 'compute-threads':
    case 'computethreads':
    case 'compute':
    case 'threads':
      capabilities.computeThreads = enabled;
      break;
    case 'full-system-access':
    case 'fullsystemaccess':
      capabilities.fullSystemAccess = enabled;
      break;
    default:
      break;
  }
}

function isCapabilityEnabled(capabilities, cap) {
  switch (cap) {
    case 'network':
      return capabilities.network;
    case 'sqlite':
      return capabilities.sqlite;
    case 'filesystem:xdg':
      return capabilities.filesystem.xdg;
    case 'filesystem:dialogs':
      return capabilities.filesystem.dialogs;
    case 'filesystem:drop':
      return capabilities.filesystem.drop;
    case 'clipboard':
      return capabilities.clipboard;
    case 'notifications':
      return capabilities.notifications;
    case 'compute-threads':
      return capabilities.computeThreads;
    default:
      return false;
  }
}

const IMPORT_SPECIFIER_CAPS = new Map([
  ['HttpClient', 'network'],
  ['provideNativeHttp', 'network'],
  ['provideHttpClient', 'network'],
  ['NativeSqliteService', 'sqlite'],
  ['NativeFsService', 'filesystem:xdg'],
  ['NativeComputeService', 'compute-threads'],
]);

const CALL_NAME_CAPS = new Map([
  ['fetch', 'network'],
  ['httpRequest', 'network'],
  ['httpBatch', 'network'],
  ['downloadFiles', 'network'],
  ['httpResource', 'network'],
  ['configureHttp', 'network'],
  ['sqliteExec', 'sqlite'],
  ['sqliteQuery', 'sqlite'],
  ['openSqlite', 'sqlite'],
  ['resolveXdgPath', 'filesystem:xdg'],
  ['fsResolvePath', 'filesystem:xdg'],
  ['fsReadText', 'filesystem:xdg'],
  ['fsWriteText', 'filesystem:xdg'],
  ['fsWatchFile', 'filesystem:xdg'],
  ['openFileDialog', 'filesystem:dialogs'],
  ['saveFileDialog', 'filesystem:dialogs'],
  ['getClipboard', 'clipboard'],
  ['setClipboard', 'clipboard'],
  ['readClipboard', 'clipboard'],
  ['writeClipboard', 'clipboard'],
  ['sendNotification', 'notifications'],
  ['computeFilter', 'compute-threads'],
  ['computeHash', 'compute-threads'],
]);

function visitAst(node, visitor) {
  if (!node || typeof node !== 'object') return;
  if (Array.isArray(node)) {
    for (const item of node) {
      visitAst(item, visitor);
    }
    return;
  }
  if (typeof node.type === 'string') {
    visitor(node);
  }
  for (const key of Object.keys(node)) {
    if (key === 'loc' || key === 'span' || key === 'start' || key === 'end') continue;
    const val = node[key];
    if (val && typeof val === 'object') {
      visitAst(val, visitor);
    }
  }
}

function inspectTypeScriptAst(filename, code, addEvidence) {
  const parsed = parseSync(filename, code);
  visitAst(parsed.program, (node) => {
    if (node.type === 'ImportDeclaration') {
      const modName = node.source?.value;
      if (
        modName === '@ng-desktop/http' ||
        modName === '@ng-native/http' ||
        modName === '@angular/common/http'
      ) {
        addEvidence('network', filename, `import from '${modName}'`);
      }
      for (const spec of node.specifiers || []) {
        const imported = spec.imported?.name || spec.local?.name;
        const cap = IMPORT_SPECIFIER_CAPS.get(imported);
        if (cap) {
          addEvidence(cap, filename, `import '${imported}'`);
        }
      }
      return;
    }

    if (node.type === 'CallExpression') {
      if (node.callee?.type === 'Identifier') {
        const cap = CALL_NAME_CAPS.get(node.callee.name);
        if (cap) {
          addEvidence(cap, filename, `call '${node.callee.name}()'`);
        }
      } else if (
        node.callee?.type === 'MemberExpression' &&
        node.callee.property?.type === 'Identifier'
      ) {
        const method = node.callee.property.name;
        const cap = method === 'notify' ? 'notifications' : CALL_NAME_CAPS.get(method);
        if (cap) {
          addEvidence(cap, filename, `call '.${method}()'`);
        }
      }
      return;
    }

    if (
      node.type === 'NewExpression' &&
      node.callee?.type === 'Identifier' &&
      node.callee.name === 'WebSocket'
    ) {
      addEvidence('network', filename, `new WebSocket()`);
      return;
    }

    if (node.type === 'Literal' && typeof node.value === 'string') {
      if (node.value === 'file-drop' || node.value === 'ng-drop-target') {
        addEvidence('filesystem:drop', filename, `literal '${node.value}'`);
      }
    }
  });
}

function collectProjectSourceFiles(rootDir, currentDir = rootDir, out = []) {
  if (!existsSync(currentDir)) return out;
  for (const entry of readdirSync(currentDir)) {
    const fullPath = resolve(currentDir, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      collectProjectSourceFiles(rootDir, fullPath, out);
    } else if (
      (entry.endsWith('.ts') && !entry.endsWith('.spec.ts') && !entry.endsWith('.d.ts')) ||
      entry.endsWith('.html')
    ) {
      out.push({
        file: relative(rootDir, fullPath),
        content: readFileSync(fullPath, 'utf8'),
      });
    }
  }
  return out;
}

/**
 * Statically analyzes source files using `oxc-parser` AST traversal and template attribute checks,
 * applying optional `permissions` policy (`mode: 'auto' | 'strict'`, `allow`, `deny`).
 */
export function analyzeSourcePermissions(files, permissionsConfig = {}) {
  const mode = permissionsConfig?.mode === 'strict' ? 'strict' : 'auto';
  const allowList = Array.isArray(permissionsConfig?.allow) ? permissionsConfig.allow : [];
  const denyList = Array.isArray(permissionsConfig?.deny) ? permissionsConfig.deny : [];

  const evidence = [];
  const seenKeys = new Set();
  const addEvidence = (capability, file, reason) => {
    const key = `${capability}::${file}::${reason}`;
    if (seenKeys.has(key)) return;
    seenKeys.add(key);
    evidence.push({ capability, file, reason });
  };

  for (const item of files) {
    if (item.file.endsWith('.ts') || item.file.endsWith('.mts') || item.file.endsWith('.js')) {
      inspectTypeScriptAst(item.file, item.content, addEvidence);
    } else if (item.file.endsWith('.html')) {
      if (
        item.content.includes('(file-drop)') ||
        item.content.includes('dropTarget') ||
        item.content.includes('ng-drop-target')
      ) {
        addEvidence('filesystem:drop', item.file, 'template drop-target binding');
      }
    }
  }

  const capabilities = createEmptyCapabilities();
  if (mode === 'auto') {
    for (const item of evidence) {
      applyCapabilityToken(capabilities, item.capability, true);
    }
  }

  for (const token of allowList) {
    applyCapabilityToken(capabilities, token, true);
  }
  for (const token of denyList) {
    applyCapabilityToken(capabilities, token, false);
  }

  const warnings = [];
  if (mode === 'strict') {
    for (const item of evidence) {
      if (!isCapabilityEnabled(capabilities, item.capability) && !capabilities.fullSystemAccess) {
        warnings.push(
          `Detected '${item.capability}' usage in '${item.file}' (${item.reason}), but strict mode omits it unless listed in allow.`,
        );
      }
    }
  }

  return {
    schemaVersion: 1,
    mode,
    capabilities,
    evidence,
    warnings,
  };
}

/**
 * Recursively collects application TypeScript/HTML sources under `sourceRoot` and computes the
 * build-time least-privilege capability manifest.
 */
export function analyzeProjectPermissions(sourceRoot, permissionsConfig = {}) {
  const files = collectProjectSourceFiles(sourceRoot);
  return analyzeSourcePermissions(files, permissionsConfig);
}
