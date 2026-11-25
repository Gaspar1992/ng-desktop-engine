import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  watch,
  writeFileSync,
} from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { extractComponentMetadata } from './ast-metadata-extractor.mjs';
import { pushHmrReload, pushHmrServerReload } from './hmr-client.mjs';
import { buildMultiResolutionIcons } from './icon-compiler.mjs';
import { analyzeProjectPermissions } from './permission-analyzer.mjs';
import { compileServerBundle, discoverServerRoutes, runEsbuildBuild } from './server-compiler.mjs';
import {
  buildStyleRuleKey,
  emitStyleInstruction,
  extractCssVariables,
  formatStyleRuleExtensions,
  parseStylesCss,
} from './style-compiler.mjs';
import { parseTemplateHtml } from './template-compiler.mjs';
import { resolveWorkspaceProject } from './workspace-resolver.mjs';

export {
  compileServerBundle,
  discoverServerRoutes,
  extractComponentMetadata,
  parseStylesCss,
  parseTemplateHtml,
  pushHmrReload,
  pushHmrServerReload,
};

const lastEmittedByOutDir = new Map();

function extractCliFlagValue(argv, flag) {
  const prefix = `${flag}=`;
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg.startsWith(prefix)) {
      return arg.slice(prefix.length).trim() || undefined;
    }
    if (arg === flag && i + 1 < argv.length && !argv[i + 1].startsWith('--')) {
      return argv[i + 1].trim() || undefined;
    }
  }
  return undefined;
}

function mapNativeEventName(evtName) {
  if (evtName === 'click') return 'clicked';
  if (evtName === 'change') return 'toggled';
  if (evtName === 'valueChange') return 'value-changed';
  return evtName;
}

function collectComponentTsFiles(dirPath, rootAppDir, out = []) {
  if (!existsSync(dirPath)) return out;
  for (const entry of readdirSync(dirPath)) {
    const full = resolve(dirPath, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      collectComponentTsFiles(full, rootAppDir, out);
    } else if (entry.endsWith('.ts') && !entry.endsWith('.spec.ts') && entry !== 'app.ts') {
      out.push(relative(rootAppDir, full));
    }
  }
  return out;
}

/**
 * 4. Bytecode Emitter (SRP): Translates Styles IR + Signals + Icons + Template AST into `.ngb` instructions.
 */
export function emitNgbBundle(
  styleRules,
  numericSignals,
  methods,
  astRoots,
  iconMeta,
  sourceLabel,
) {
  const lines = [`// Auto-compiled by Angular Native Template Compiler from ${sourceLabel}`];

  for (const [sigName, initialValue] of Object.entries(numericSignals)) {
    lines.push(`signal ${sigName} ${initialValue}`);
  }

  for (const rule of styleRules) {
    lines.push(emitStyleInstruction(rule));
  }

  const appendInstructions = [];
  const bindingInstructions = [];

  function visitNode(node) {
    if (node.isComment) {
      lines.push(`comment ${node.id} ${node.text}`);
      return;
    }

    lines.push(`create ${node.id} ${node.tag}`);

    if (node.title) {
      lines.push(`text ${node.id} ${node.title}`);
    } else if (node.text) {
      lines.push(`text ${node.id} ${node.text}`);
    }

    if ((node.tag === 'ng-window' || node.tag === 'adw-window') && iconMeta) {
      lines.push(`icon ${node.id} ${iconMeta.appId} ${iconMeta.relPath}`);
    }

    for (const [attrKey, attrVal] of Object.entries(node.attributes || {})) {
      lines.push(`attr ${node.id} ${attrKey} ${attrVal}`);
    }

    for (const cls of node.classes) {
      lines.push(`add_class ${node.id} ${cls}`);
    }

    if (node.reactiveBinding) {
      bindingInstructions.push(
        `bind_text ${node.id} ${node.reactiveBinding.signalName} ${node.reactiveBinding.template}`,
      );
    }

    for (const attrBind of node.reactiveAttrs || []) {
      bindingInstructions.push(
        `bind_attr ${node.id} ${attrBind.attrKey} ${attrBind.signalName} ${attrBind.template}`,
      );
    }

    if (node.visibilityBinding) {
      bindingInstructions.push(
        `bind_visible ${node.id} ${node.visibilityBinding.signalName} ${node.visibilityBinding.op} ${node.visibilityBinding.rhs}`,
      );
    }

    for (const [evtName, methodName] of Object.entries(node.events)) {
      const nativeEvt = mapNativeEventName(evtName);
      const methodMeta = methods[methodName];
      if (methodMeta) {
        const opcode = methodMeta.kind === 'set' ? 'on_set' : 'on_add';
        bindingInstructions.push(
          `${opcode} ${node.id} ${nativeEvt} ${methodMeta.signalName} ${methodMeta.value}`,
        );
      }
    }

    for (const child of node.children) {
      visitNode(child);
      appendInstructions.push(`append ${node.id} ${child.id}`);
    }
  }

  for (const root of astRoots) {
    visitNode(root);
  }

  lines.push(...appendInstructions);
  lines.push(...bindingInstructions);
  return lines.join('\n') + '\n';
}

function emitAppJsBundleIfPresent(outDir, projectName, allStyleRules, iconMeta) {
  const browserDir = resolve(outDir, 'browser');
  if (!existsSync(browserDir)) {
    return undefined;
  }
  const entries = readdirSync(browserDir);
  const mainFile =
    entries.find((f) => f === 'main.js') ?? entries.find((f) => /^main-.*\.js$/.test(f));
  if (!mainFile) {
    return undefined;
  }
  const polyfillsFile =
    entries.find((f) => f === 'polyfills.js') ?? entries.find((f) => /^polyfills-.*\.js$/.test(f));

  const preamble = [
    `globalThis.__NG_NATIVE_APP_ID__ = ${JSON.stringify(projectName)};`,
    iconMeta
      ? `globalThis.__NG_DESKTOP_ICON__ = ${JSON.stringify(iconMeta)};`
      : 'delete globalThis.__NG_DESKTOP_ICON__;',
  ];
  for (const rule of allStyleRules) {
    const extras = formatStyleRuleExtensions(rule);
    preamble.push(
      `globalThis.__NG_NATIVE__?.registerStyle(${JSON.stringify(rule.className)}, ${JSON.stringify(rule.background)}, ${JSON.stringify(rule.color)}, ${Number(rule.padding) || 0}, ${Number(rule.spacing) || 0}, ${JSON.stringify(extras)});`,
    );
  }

  const entryLines = [];
  if (polyfillsFile) {
    entryLines.push(`import ${JSON.stringify(`./${polyfillsFile}`)};`);
  }
  entryLines.push(`import ${JSON.stringify(`./${mainFile}`)};`);

  const bundled = runEsbuildBuild({
    workspaceRoot: outDir,
    resolveDir: browserDir,
    entrySource: entryLines.join('\n'),
    loader: 'js',
    define: { __NG_DESKTOP_PROD_BUNDLE__: 'true' },
    minifySyntax: true,
    banner: { js: preamble.join('\n') },
  });

  const entryBundleFile = resolve(outDir, 'app.bundle.js');
  writeFileSync(entryBundleFile, bundled, 'utf8');
  return entryBundleFile;
}

function collectComponentCssSources(compMeta, baseDir) {
  const sources = [];
  const stylePaths = [];
  if (compMeta.styleUrl) {
    stylePaths.push(compMeta.styleUrl);
  }
  if (Array.isArray(compMeta.styleUrls)) {
    stylePaths.push(...compMeta.styleUrls);
  }
  for (const relCss of stylePaths) {
    const absCss = resolve(baseDir, relCss);
    if (existsSync(absCss)) {
      sources.push({ css: readFileSync(absCss, 'utf8'), from: relCss });
    }
  }
  if (Array.isArray(compMeta.styles)) {
    for (const inlineCss of compMeta.styles) {
      sources.push({ css: inlineCss, from: `${compMeta.selector}.inline.css` });
    }
  }
  return sources;
}

function collectComponentStyleRules(compMeta, baseDir, sharedVars = null) {
  const rules = [];
  for (const { css, from } of collectComponentCssSources(compMeta, baseDir)) {
    rules.push(...parseStylesCss(css, from, sharedVars));
  }
  return rules;
}

function dedupeStyleRules(rules) {
  const byKey = new Map();
  for (const rule of rules) {
    byKey.set(buildStyleRuleKey(rule), rule);
  }
  return [...byKey.values()];
}

export function compileBundle({
  rebuildIcons = true,
  cwd,
  configPath,
  projectName: requestedProject,
  version: versionOverride,
} = {}) {
  const workspace = resolveWorkspaceProject({
    cwd,
    configPath,
    projectName: requestedProject,
  });
  const rootDir = workspace.workspaceRoot;
  const projectName = workspace.projectName;
  const nativeOptions = workspace.nativeOptions ?? {};
  const resolvedOutputPath = workspace.outputPath;
  const resolvedVersion = versionOverride || workspace.version;

  const sourceRootAbs = resolve(rootDir, workspace.sourceRoot);
  const appDir = resolve(sourceRootAbs, 'app');
  const tsSource = readFileSync(resolve(appDir, 'app.ts'), 'utf8');
  const { state, numericSignals, methods, component } = extractComponentMetadata(
    tsSource,
    'app.ts',
  );
  const htmlSource =
    typeof component.template === 'string'
      ? component.template
      : component.templateUrl && existsSync(resolve(appDir, component.templateUrl))
        ? readFileSync(resolve(appDir, component.templateUrl), 'utf8')
        : '';
  const indexHtmlPath = resolve(sourceRootAbs, 'index.html');
  const indexHtml = existsSync(indexHtmlPath) ? readFileSync(indexHtmlPath, 'utf8') : '';

  const iconHrefMatch = indexHtml.match(/<link[^>]*rel="icon"[^>]*href="([^"]+)"/);
  const iconFileName = iconHrefMatch ? iconHrefMatch[1] : 'favicon.ico';
  const publicPrefix = workspace.projectRoot === '.' ? 'public' : `${workspace.projectRoot}/public`;
  const iconRelPath = `${publicPrefix}/${iconFileName}`;
  const iconAbsPath = resolve(rootDir, iconRelPath);

  const componentRegistry = {};
  const sharedVars = {
    baseVars: new Map(),
    lightVars: new Map(),
    darkVars: new Map(),
  };
  const globalStylesPath = resolve(sourceRootAbs, 'styles.css');
  if (existsSync(globalStylesPath)) {
    extractCssVariables(readFileSync(globalStylesPath, 'utf8'), 'styles.css', sharedVars);
  }
  for (const { css, from } of collectComponentCssSources(component, appDir)) {
    extractCssVariables(css, from, sharedVars);
  }

  const collectedRules = [];
  if (existsSync(globalStylesPath)) {
    collectedRules.push(
      ...parseStylesCss(readFileSync(globalStylesPath, 'utf8'), 'styles.css', sharedVars),
    );
  }
  collectedRules.push(...collectComponentStyleRules(component, appDir, sharedVars));

  const candidateFiles = collectComponentTsFiles(appDir, appDir);
  for (const file of candidateFiles) {
    const filePath = resolve(appDir, file);
    if (existsSync(filePath)) {
      const ts = readFileSync(filePath, 'utf8');
      if (!ts.includes('@Component')) continue;
      const meta = extractComponentMetadata(ts, file);
      if (!meta.component.hasComponentDecorator) continue;
      const compDir = dirname(filePath);
      const templateHtml =
        typeof meta.component.template === 'string'
          ? meta.component.template
          : meta.component.templateUrl && existsSync(resolve(compDir, meta.component.templateUrl))
            ? readFileSync(resolve(compDir, meta.component.templateUrl), 'utf8')
            : '';
      collectedRules.push(...collectComponentStyleRules(meta.component, compDir, sharedVars));
      componentRegistry[meta.component.selector] = {
        templateHtml,
        templateUrl: meta.component.templateUrl || `${meta.component.selector}.html`,
      };
    }
  }
  const allStyleRules = dedupeStyleRules(collectedRules);

  const templateAst = parseTemplateHtml(
    htmlSource,
    state,
    numericSignals,
    'app.html',
    componentRegistry,
  );

  const outDir = resolve(rootDir, resolvedOutputPath);
  mkdirSync(outDir, { recursive: true });

  let iconMeta = null;
  if (existsSync(iconAbsPath)) {
    iconMeta = { appId: projectName, relPath: iconRelPath };
    if (rebuildIcons) {
      buildMultiResolutionIcons(iconAbsPath, projectName, outDir, state.windowTitle);
    }
  }

  const bundle = emitNgbBundle(
    allStyleRules,
    numericSignals,
    methods,
    templateAst,
    iconMeta,
    `${workspace.sourceRoot}/app/app.{ts,html,css}`,
  );

  const bundleFile = resolve(outDir, 'main.ngb');
  writeFileSync(bundleFile, bundle, 'utf8');

  const entryBundleFile = emitAppJsBundleIfPresent(outDir, projectName, allStyleRules, iconMeta);
  const serverBundleFile = compileServerBundle({
    workspaceRoot: rootDir,
    sourceRootAbs,
    outDir,
    projectName,
  });

  const permissions = analyzeProjectPermissions(sourceRootAbs, nativeOptions.permissions);
  const permissionsFile = resolve(outDir, 'permissions.manifest.json');
  writeFileSync(permissionsFile, JSON.stringify(permissions, null, 2) + '\n', 'utf8');

  const runtimeConfig = {
    schemaVersion: 1,
    workspaceKind: workspace.workspaceKind,
    projectName,
    version: resolvedVersion,
    targetPlatform: nativeOptions.targetPlatform || 'linux-gtk4',
    outputPath: resolvedOutputPath,
    iconSearchDir: `${resolvedOutputPath}/icons`,
    ...(entryBundleFile ? { entryBundle: 'app.bundle.js' } : {}),
    ...(serverBundleFile ? { serverBundle: 'server.bundle.js' } : {}),
    permissions,
  };
  const runtimeConfigFile = resolve(outDir, 'ng-desktop.runtime.json');
  writeFileSync(runtimeConfigFile, JSON.stringify(runtimeConfig, null, 2) + '\n', 'utf8');

  console.log(
    `[Angular Native Compiler] Compiled ${projectName} v${resolvedVersion} (${workspace.workspaceKind}) -> ${bundleFile}`,
  );
  if (entryBundleFile) {
    console.log(`[Angular Native Compiler] Bundled Ivy JS runtime entry -> ${entryBundleFile}`);
  }
  if (serverBundleFile) {
    console.log(`[Angular Native Compiler] Bundled Analog server routes -> ${serverBundleFile}`);
  }

  const prevEmitted = lastEmittedByOutDir.get(outDir);
  const appBundleContent = entryBundleFile ? readFileSync(entryBundleFile, 'utf8') : null;
  const serverBundleContent = serverBundleFile ? readFileSync(serverBundleFile, 'utf8') : null;
  const hmrDelta = {
    ngbChanged: !prevEmitted || prevEmitted.ngb !== bundle,
    appBundleChanged: Boolean(
      appBundleContent && (!prevEmitted || prevEmitted.appBundle !== appBundleContent),
    ),
    serverBundleChanged: Boolean(
      serverBundleContent && (!prevEmitted || prevEmitted.serverBundle !== serverBundleContent),
    ),
  };
  lastEmittedByOutDir.set(outDir, {
    ngb: bundle,
    appBundle: appBundleContent,
    serverBundle: serverBundleContent,
  });

  const socketPath = resolve(outDir, 'hmr.sock');
  if (existsSync(socketPath)) {
    if (hmrDelta.ngbChanged) {
      pushHmrReload(socketPath, bundle).then((res) => {
        if (res.connected && res.response?.status === 'ok') {
          console.log(`[Angular Native HMR] Pushed live bundle update over IPC -> ${socketPath}`);
        }
      });
    }
    if (hmrDelta.serverBundleChanged && serverBundleContent) {
      pushHmrServerReload(socketPath, serverBundleContent).then((res) => {
        if (res.connected && res.response?.status === 'ok') {
          console.log(
            `[Angular Native HMR] Pushed live server route reload over IPC -> ${socketPath}`,
          );
        }
      });
    }
  }

  return {
    projectName,
    version: resolvedVersion,
    sourceRoot: sourceRootAbs,
    bundleFile,
    ...(entryBundleFile ? { entryBundleFile } : {}),
    ...(serverBundleFile ? { serverBundleFile } : {}),
    permissionsFile,
    runtimeConfigFile,
    permissions,
    socketPath,
    hmrDelta,
  };
}

export function watchBundle(options = {}) {
  const { sourceRoot } = compileBundle({ ...options, rebuildIcons: true });
  console.log(`[Angular Native Watcher] Watching '${sourceRoot}' for changes...`);

  let debounceTimer = null;
  return watch(sourceRoot, { recursive: true }, (_eventType, filename) => {
    if (!filename || !/\.(ts|html|css)$/.test(filename)) return;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      try {
        compileBundle({ ...options, rebuildIcons: filename.endsWith('index.html') });
      } catch (err) {
        console.error('[Angular Native Watcher] Compilation error:', err);
      }
    }, 60);
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const cliOptions = {
    configPath: extractCliFlagValue(process.argv, '--config'),
    projectName: extractCliFlagValue(process.argv, '--project'),
    version: extractCliFlagValue(process.argv, '--version'),
  };
  if (process.argv.includes('--watch')) {
    watchBundle(cliOptions);
  } else {
    compileBundle({ ...cliOptions, rebuildIcons: true });
  }
}
