import { execFileSync } from 'node:child_process';
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = resolve(__dirname, '..');
const DEFAULT_OUT_DIR = resolve(ROOT_DIR, 'dist/ng-desktop-docs/browser');
const REPO_BLOB_BASE = 'https://github.com/Gaspar1992/ng-desktop-engine/blob/main';

const PACKAGES = [
  { dir: 'platform-native', slug: 'platform' },
  { dir: 'forms-native', slug: 'forms' },
  { dir: 'router-native', slug: 'router' },
  { dir: 'http-native', slug: 'http' },
  { dir: 'desktop-native', slug: 'desktop' },
  { dir: 'server-native', slug: 'server' },
  { dir: 'builders-native', slug: 'builders' },
];

const KIND_ORDER = ['Function', 'Class', 'Interface', 'Type', 'Enum', 'Const'];

function parseCliArgs(argv) {
  let outDir = DEFAULT_OUT_DIR;
  let skipRust = false;
  let baseHref = '/ng-desktop-engine/';

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--skip-rust') {
      skipRust = true;
    } else if (arg.startsWith('--out-dir=')) {
      const val = arg.slice('--out-dir='.length).trim();
      if (val) outDir = resolve(ROOT_DIR, val);
    } else if (arg === '--out-dir' && i + 1 < argv.length) {
      const val = argv[i + 1].trim();
      if (val) outDir = resolve(ROOT_DIR, val);
      i += 1;
    } else if (arg.startsWith('--base-href=')) {
      const val = arg.slice('--base-href='.length).trim();
      if (val) baseHref = val;
    } else if (arg === '--base-href' && i + 1 < argv.length) {
      const val = argv[i + 1].trim();
      if (val) baseHref = val;
      i += 1;
    }
  }

  return { outDir, skipRust, baseHref };
}

function escapeHtml(raw) {
  return String(raw)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function truncateText(text, maxLength = 240) {
  const normalized = text.replace(/\s+/g, ' ').trim();
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength - 3).trimEnd()}...`;
}

function classifySymbolKind(symbol) {
  const flags = symbol.getFlags();
  if (flags & ts.SymbolFlags.Class) return 'Class';
  if (flags & ts.SymbolFlags.Interface) return 'Interface';
  if (flags & ts.SymbolFlags.Function) return 'Function';
  if (flags & ts.SymbolFlags.TypeAlias) return 'Type';
  if (
    flags & ts.SymbolFlags.Enum ||
    flags & ts.SymbolFlags.RegularEnum ||
    flags & ts.SymbolFlags.ConstEnum
  ) {
    return 'Enum';
  }
  const decl = symbol.valueDeclaration ?? symbol.declarations?.[0];
  if (decl && (ts.isFunctionDeclaration(decl) || ts.isFunctionExpression(decl))) {
    return 'Function';
  }
  return 'Const';
}

function resolveSymbolLocation(symbol) {
  const decl = symbol.valueDeclaration ?? symbol.declarations?.[0];
  if (!decl) {
    return { sourcePath: null, line: null, decl: null };
  }
  const sourceFile = decl.getSourceFile();
  const rel = relative(ROOT_DIR, sourceFile.fileName).replaceAll('\\', '/');
  if (!rel || rel.startsWith('..') || rel.includes('node_modules/')) {
    return { sourcePath: null, line: null, decl };
  }
  const pos = decl.getStart(sourceFile);
  const { line } = sourceFile.getLineAndCharacterOfPosition(pos);
  return { sourcePath: rel, line: line + 1, decl };
}

function extractSymbolSignature(checker, symbol, kind, decl) {
  const name = symbol.getName();
  if (!decl) {
    return `${kind.toLowerCase()} ${name}`;
  }

  try {
    const type = checker.getTypeOfSymbolAtLocation(symbol, decl);
    if (kind === 'Function') {
      const signatures = checker.getSignaturesOfType(type, ts.SignatureKind.Call);
      if (signatures.length > 0) {
        return truncateText(
          `function ${name}${checker.signatureToString(signatures[0], decl, ts.TypeFormatFlags.NoTruncation)}`,
          280,
        );
      }
      return `function ${name}(...)`;
    }
    if (kind === 'Class') {
      return `class ${name}`;
    }
    if (kind === 'Interface') {
      return `interface ${name}`;
    }
    if (kind === 'Enum') {
      return `enum ${name}`;
    }
    if (kind === 'Type') {
      const declaredType = checker.getDeclaredTypeOfSymbol(symbol);
      return truncateText(`type ${name} = ${checker.typeToString(declaredType, decl)}`, 240);
    }
    return truncateText(`const ${name}: ${checker.typeToString(type, decl)}`, 220);
  } catch {
    return `${kind.toLowerCase()} ${name}`;
  }
}

function isNonPublicMember(decl) {
  if (!decl) return false;
  const modFlags = ts.getCombinedModifierFlags(decl);
  return Boolean(modFlags & ts.ModifierFlags.Private || modFlags & ts.ModifierFlags.Protected);
}

function extractPublicMembers(checker, symbol, kind) {
  if ((kind !== 'Class' && kind !== 'Interface') || !symbol.members) {
    return [];
  }
  const members = [];
  for (const member of symbol.members.values()) {
    const name = member.getName();
    if (!name || name === '__constructor' || name.startsWith('__@') || name.startsWith('#')) {
      continue;
    }
    const decl = member.valueDeclaration ?? member.declarations?.[0];
    if (!decl || isNonPublicMember(decl)) {
      continue;
    }
    let signature = name;
    try {
      const type = checker.getTypeOfSymbolAtLocation(member, decl);
      const callSigs = checker.getSignaturesOfType(type, ts.SignatureKind.Call);
      if (
        callSigs.length > 0 &&
        (member.getFlags() & ts.SymbolFlags.Method ||
          ts.isMethodSignature(decl) ||
          ts.isMethodDeclaration(decl))
      ) {
        signature = truncateText(`${name}${checker.signatureToString(callSigs[0], decl)}`, 180);
      } else {
        const optionalMark = member.getFlags() & ts.SymbolFlags.Optional ? '?' : '';
        signature = truncateText(
          `${name}${optionalMark}: ${checker.typeToString(type, decl)}`,
          180,
        );
      }
    } catch {
      signature = name;
    }

    const description = ts.displayPartsToString(member.getDocumentationComment(checker)).trim();

    members.push({
      name,
      signature,
      description,
    });
  }
  return members;
}

function extractTsApiCatalog() {
  const pathMappings = {};
  const packageMetas = PACKAGES.map((pkg) => {
    const pkgRoot = resolve(ROOT_DIR, 'packages', pkg.dir);
    const pkgJson = JSON.parse(readFileSync(resolve(pkgRoot, 'package.json'), 'utf8'));
    const entryPath = resolve(pkgRoot, 'src/index.ts');
    pathMappings[pkgJson.name] = [entryPath];
    return {
      ...pkg,
      name: pkgJson.name,
      version: pkgJson.version,
      description: pkgJson.description ?? `${pkgJson.name} for NgDesktop Engine`,
      entryPath,
    };
  });

  const program = ts.createProgram(
    packageMetas.map((meta) => meta.entryPath),
    {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler,
      allowJs: true,
      skipLibCheck: true,
      experimentalDecorators: true,
      strict: true,
      baseUrl: ROOT_DIR,
      paths: pathMappings,
    },
  );
  const checker = program.getTypeChecker();

  const packages = packageMetas.map((meta) => {
    const sourceFile = program.getSourceFile(meta.entryPath);
    if (!sourceFile) {
      throw new Error(`Unable to load package entrypoint: ${meta.entryPath}`);
    }
    const moduleSymbol = checker.getSymbolAtLocation(sourceFile);
    if (!moduleSymbol) {
      throw new Error(`Unable to inspect TypeScript module symbol: ${meta.entryPath}`);
    }

    const rawExports = checker.getExportsOfModule(moduleSymbol);
    const symbols = rawExports
      .map((exportedSym) => {
        let target = exportedSym;
        if (exportedSym.getFlags() & ts.SymbolFlags.Alias) {
          try {
            const aliased = checker.getAliasedSymbol(exportedSym);
            if (aliased && aliased.getName() !== 'unknown') {
              target = aliased;
            }
          } catch {
            target = exportedSym;
          }
        }

        const kind = classifySymbolKind(target);
        const { sourcePath, line, decl } = resolveSymbolLocation(target);
        const docs =
          ts.displayPartsToString(target.getDocumentationComment(checker)).trim() ||
          ts.displayPartsToString(exportedSym.getDocumentationComment(checker)).trim();
        const tags = (target.getJsDocTags(checker) ?? [])
          .map((tag) => ({
            name: tag.name,
            text: ts.displayPartsToString(tag.text ?? []).trim(),
          }))
          .filter((tag) => tag.name);

        return {
          name: exportedSym.getName(),
          kind,
          signature: extractSymbolSignature(checker, target, kind, decl),
          description: docs,
          tags,
          members: extractPublicMembers(checker, target, kind),
          sourcePath,
          line,
        };
      })
      .sort((a, b) => {
        const kindDiff = KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind);
        return kindDiff !== 0 ? kindDiff : a.name.localeCompare(b.name);
      });

    return {
      name: meta.name,
      slug: meta.slug,
      version: meta.version,
      description: meta.description,
      symbolCount: symbols.length,
      symbols,
    };
  });

  return {
    generatedAt: new Date().toISOString(),
    packages,
  };
}

function buildSharedStylesheet() {
  return `:root {
  color-scheme: light dark;

  &:has(> head > meta[name='color-scheme'][content='dark']) {
    color-scheme: dark;
  }

  &:has(> head > meta[name='color-scheme'][content='light']) {
    color-scheme: light;
  }

  --bg: light-dark(#fafaf9, #0e1015);
  --bg-elevated: light-dark(#ffffff, #151821);
  --surface: light-dark(#f4f4f6, #1b1f2a);
  --border: light-dark(#dcdfe6, #2d3344);
  --border-strong: light-dark(#bcc2d0, #40485f);
  --text: light-dark(#14171f, #edf0f7);
  --text-muted: light-dark(#4c5467, #9aa4b8);
  --accent: light-dark(#c80036, #ff4d79);
  --accent-soft: light-dark(rgba(200, 0, 54, 0.1), rgba(255, 77, 121, 0.16));
  --code-bg: light-dark(#f3f4f8, #090b0f);
  --font-sans: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --font-mono: ui-monospace, 'SFMono-Regular', Menlo, Monaco, Consolas, monospace;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: var(--font-sans);
  line-height: 1.6;
  background: var(--bg);
  color: var(--text);
}

a {
  color: var(--accent);
  text-decoration-thickness: 1.5px;
  text-underline-offset: 0.2em;
}

:focus-visible {
  outline: 3px solid var(--accent);
  outline-offset: 2px;
}

code, pre {
  font-family: var(--font-mono);
}

pre {
  margin: 0;
  padding: 0.85rem 1rem;
  border-radius: 10px;
  background: var(--code-bg);
  border: 1px solid var(--border);
  overflow-x: auto;
  font-size: 0.86rem;
}

.topbar {
  position: sticky;
  top: 0;
  z-index: 20;
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
  padding: 0.85rem 1.25rem;
}

.topbar-inner,
.page-shell {
  width: min(1180px, calc(100% - 2rem));
  margin-inline: auto;
}

.topbar-inner {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.nav-links {
  display: flex;
  flex-wrap: wrap;
  gap: 0.9rem;
  font-weight: 600;
  font-size: 0.92rem;
}

.page-shell {
  padding-block: 2rem 4rem;
  display: grid;
  gap: 1.5rem;
}

.hero,
.card {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 1.25rem;
}

.search-input {
  width: 100%;
  padding: 0.65rem 0.9rem;
  border-radius: 10px;
  border: 1px solid var(--border-strong);
  background: var(--surface);
  color: var(--text);
  font: inherit;
}

.pkg-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 1rem;
}

.badge {
  display: inline-block;
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  padding: 0.14rem 0.5rem;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
}

.pkg-layout {
  display: grid;
  gap: 1.25rem;
  align-items: start;
}

@media (min-width: 960px) {
  .pkg-layout {
    grid-template-columns: 260px minmax(0, 1fr);
  }
}

.toc-sidebar {
  position: sticky;
  top: 4.5rem;
  max-height: 75vh;
  overflow-y: auto;
  display: grid;
  gap: 0.35rem;
}

.toc-sidebar a {
  text-decoration: none;
  font-size: 0.86rem;
  color: var(--text-muted);
}

.toc-sidebar a:hover {
  color: var(--accent);
}

.symbol-list {
  display: grid;
  gap: 1rem;
}

.symbol-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.65rem;
}

.symbol-head h2 {
  margin: 0;
  font-size: 1.15rem;
}

.muted {
  color: var(--text-muted);
}

.members-table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 0.85rem;
  font-size: 0.88rem;
}

.members-table th,
.members-table td {
  padding: 0.5rem 0.65rem;
  border-bottom: 1px solid var(--border);
  text-align: left;
  vertical-align: top;
}

.is-hidden {
  display: none;
}
`;
}

function buildHtmlHead(title, description) {
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="color-scheme" content="light dark" />
    <title>${escapeHtml(title)}</title>
    <meta name="description" content="${escapeHtml(description)}" />
    <link rel="stylesheet" href="styles.css" />
    <script>
      try {
        var saved = localStorage.getItem('ng-desktop-docs-theme');
        if (saved === 'light' || saved === 'dark') {
          var meta = document.querySelector('meta[name="color-scheme"]');
          if (meta) meta.setAttribute('content', saved);
        }
      } catch (_err) {}
    </script>
  </head>`;
}

function buildTsApiIndexHtml(catalog) {
  const totalSymbols = catalog.packages.reduce((acc, pkg) => acc + pkg.symbolCount, 0);
  const cardsHtml = catalog.packages
    .map(
      (pkg) => `
      <article class="card">
        <div class="symbol-head">
          <h2><a href="${escapeHtml(pkg.slug)}.html">${escapeHtml(pkg.name)}</a></h2>
          <span class="badge">${pkg.symbolCount} exports · v${escapeHtml(pkg.version)}</span>
        </div>
        <p class="muted">${escapeHtml(pkg.description)}</p>
      </article>`,
    )
    .join('\n');

  const symbolRowsHtml = catalog.packages
    .flatMap((pkg) =>
      pkg.symbols.map(
        (sym) => `
        <tr data-search="${escapeHtml(`${pkg.name} ${sym.name} ${sym.kind} ${sym.description}`.toLowerCase())}">
          <td><span class="badge">${escapeHtml(sym.kind)}</span></td>
          <td><a href="${escapeHtml(pkg.slug)}.html#sym-${escapeHtml(sym.name)}"><code>${escapeHtml(sym.name)}</code></a></td>
          <td><code>${escapeHtml(pkg.name)}</code></td>
          <td class="muted">${escapeHtml(truncateText(sym.description || sym.signature, 120))}</td>
        </tr>`,
      ),
    )
    .join('\n');

  return `${buildHtmlHead(
    'TypeScript API Reference — ng-desktop-engine',
    'Complete TypeScript and Angular API reference across all seven @ng-desktop/* packages.',
  )}
  <body>
    <header class="topbar">
      <div class="topbar-inner">
        <strong>ng-desktop-engine · TypeScript API Reference</strong>
        <nav class="nav-links" aria-label="API Navigation">
          <a href="../">Docs Portal</a>
          <a href="./" aria-current="page">TypeScript API</a>
          <a href="../api-rust/">Rust API</a>
        </nav>
      </div>
    </header>
    <main class="page-shell">
      <section class="hero">
        <span class="badge">Auto-Generated from packages/*/src/index.ts</span>
        <h1>&#64;ng-desktop/* TypeScript &amp; Angular API (${totalSymbols} Symbols)</h1>
        <p class="muted">
          Extracted via the TypeScript Compiler API across all 7 libraries (including <code>.d.mts</code>-typed CLI/compiler helpers).
        </p>
        <label for="api-global-search" class="muted">Filter all exported symbols:</label>
        <input id="api-global-search" type="search" class="search-input" placeholder="Search bootstrapNativeApplication, parseRuntimeStyles, Signal..." />
      </section>

      <section class="pkg-grid" aria-label="Package reference pages">
        ${cardsHtml}
      </section>

      <section class="card">
        <h2>All Exported Symbols</h2>
        <table class="members-table">
          <thead>
            <tr>
              <th scope="col">Kind</th>
              <th scope="col">Symbol</th>
              <th scope="col">Package</th>
              <th scope="col">Summary</th>
            </tr>
          </thead>
          <tbody id="api-symbol-rows">
            ${symbolRowsHtml}
          </tbody>
        </table>
      </section>
    </main>
    <script>
      (function () {
        var input = document.getElementById('api-global-search');
        var rows = Array.prototype.slice.call(document.querySelectorAll('#api-symbol-rows tr'));
        if (!input) return;
        input.addEventListener('input', function () {
          var q = String(input.value || '').trim().toLowerCase();
          for (var i = 0; i < rows.length; i += 1) {
            var text = rows[i].getAttribute('data-search') || '';
            rows[i].classList.toggle('is-hidden', Boolean(q) && text.indexOf(q) === -1);
          }
        });
      })();
    </script>
  </body>
</html>
`;
}

function buildPackageApiHtml(pkg) {
  const tocHtml = pkg.symbols
    .map((sym) => `<a href="#sym-${escapeHtml(sym.name)}"><code>${escapeHtml(sym.name)}</code></a>`)
    .join('\n');

  const symbolsHtml = pkg.symbols
    .map((sym) => {
      const sourceLink =
        sym.sourcePath && sym.line
          ? `<a href="${REPO_BLOB_BASE}/${escapeHtml(sym.sourcePath)}#L${sym.line}" target="_blank" rel="noopener noreferrer">${escapeHtml(sym.sourcePath)}:${sym.line}</a>`
          : '';
      const membersHtml =
        sym.members.length > 0
          ? `<table class="members-table">
              <thead><tr><th scope="col">Member</th><th scope="col">Description</th></tr></thead>
              <tbody>
                ${sym.members
                  .map(
                    (m) =>
                      `<tr><td><code>${escapeHtml(m.signature)}</code></td><td class="muted">${escapeHtml(m.description || 'Public member')}</td></tr>`,
                  )
                  .join('\n')}
              </tbody>
            </table>`
          : '';

      return `<article id="sym-${escapeHtml(sym.name)}" class="card" data-symbol="${escapeHtml(`${sym.name} ${sym.kind} ${sym.description}`.toLowerCase())}">
        <div class="symbol-head">
          <h2>${escapeHtml(sym.name)}</h2>
          <div>
            <span class="badge">${escapeHtml(sym.kind)}</span>
            ${sourceLink}
          </div>
        </div>
        <pre><code>${escapeHtml(sym.signature)}</code></pre>
        ${sym.description ? `<p>${escapeHtml(sym.description)}</p>` : ''}
        ${membersHtml}
      </article>`;
    })
    .join('\n');

  return `${buildHtmlHead(`${pkg.name} — TypeScript API Reference`, pkg.description)}
  <body>
    <header class="topbar">
      <div class="topbar-inner">
        <strong>${escapeHtml(pkg.name)} (v${escapeHtml(pkg.version)})</strong>
        <nav class="nav-links" aria-label="Package Navigation">
          <a href="../">Docs Portal</a>
          <a href="./">All TS Packages</a>
          <a href="../api-rust/">Rust API</a>
        </nav>
      </div>
    </header>
    <main class="page-shell">
      <section class="hero">
        <span class="badge">${pkg.symbolCount} Exports</span>
        <h1>${escapeHtml(pkg.name)}</h1>
        <p class="muted">${escapeHtml(pkg.description)}</p>
        <input id="pkg-symbol-filter" type="search" class="search-input" placeholder="Filter ${escapeHtml(pkg.name)} symbols..." aria-label="Filter symbols in ${escapeHtml(pkg.name)}" />
      </section>

      <div class="pkg-layout">
        <aside class="card toc-sidebar" aria-label="Package exports">
          <strong>Exports</strong>
          ${tocHtml}
        </aside>
        <section id="pkg-symbol-list" class="symbol-list" aria-label="Symbol definitions">
          ${symbolsHtml}
        </section>
      </div>
    </main>
    <script>
      (function () {
        var input = document.getElementById('pkg-symbol-filter');
        var cards = Array.prototype.slice.call(document.querySelectorAll('#pkg-symbol-list article'));
        if (!input) return;
        input.addEventListener('input', function () {
          var q = String(input.value || '').trim().toLowerCase();
          for (var i = 0; i < cards.length; i += 1) {
            var text = cards[i].getAttribute('data-symbol') || '';
            cards[i].classList.toggle('is-hidden', Boolean(q) && text.indexOf(q) === -1);
          }
        });
      })();
    </script>
  </body>
</html>
`;
}

function buildRustLandingHtml() {
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="color-scheme" content="light dark" />
    <meta http-equiv="refresh" content="0; url=ng_native_engine/index.html" />
    <title>ng_native_engine Rust API Documentation</title>
  </head>
  <body>
    <main style="font-family: system-ui, sans-serif; padding: 2rem; max-width: 44rem; margin: auto;">
      <h1>ng_native_engine Rust Documentation</h1>
      <p>
        Redirecting to <a href="ng_native_engine/index.html"><code>ng_native_engine/index.html</code></a>.
      </p>
      <p>
        <a href="../">Back to Docs Portal</a> · <a href="../api-ts/">TypeScript API Reference</a>
      </p>
    </main>
  </body>
</html>
`;
}

function main() {
  const { outDir, skipRust, baseHref } = parseCliArgs(process.argv.slice(2));
  mkdirSync(outDir, { recursive: true });

  const apiTsDir = resolve(outDir, 'api-ts');
  rmSync(apiTsDir, { recursive: true, force: true });
  mkdirSync(apiTsDir, { recursive: true });

  const catalog = extractTsApiCatalog();
  writeFileSync(resolve(apiTsDir, 'styles.css'), buildSharedStylesheet(), 'utf8');
  writeFileSync(
    resolve(apiTsDir, 'api-index.json'),
    `${JSON.stringify({ ...catalog, baseHref }, null, 2)}\n`,
    'utf8',
  );
  writeFileSync(resolve(apiTsDir, 'index.html'), buildTsApiIndexHtml(catalog), 'utf8');
  for (const pkg of catalog.packages) {
    writeFileSync(resolve(apiTsDir, `${pkg.slug}.html`), buildPackageApiHtml(pkg), 'utf8');
  }

  const totalTsSymbols = catalog.packages.reduce((acc, pkg) => acc + pkg.symbolCount, 0);
  process.stdout.write(
    `[Docs API] Generated TypeScript API reference (${catalog.packages.length} packages, ${totalTsSymbols} symbols) -> ${apiTsDir}\n`,
  );

  if (!skipRust) {
    process.stdout.write('[Docs API] Building Rust documentation (cargo doc --no-deps)...\n');
    execFileSync('cargo', ['doc', '--no-deps'], {
      cwd: ROOT_DIR,
      env: { ...process.env, RUSTDOCFLAGS: '-D warnings' },
      stdio: 'inherit',
    });

    const targetDocDir = resolve(ROOT_DIR, 'target/doc');
    const apiRustDir = resolve(outDir, 'api-rust');
    rmSync(apiRustDir, { recursive: true, force: true });
    cpSync(targetDocDir, apiRustDir, { recursive: true });
    writeFileSync(resolve(apiRustDir, 'index.html'), buildRustLandingHtml(), 'utf8');
    process.stdout.write(`[Docs API] Copied Rust documentation -> ${apiRustDir}\n`);
  }

  writeFileSync(resolve(outDir, '.nojekyll'), '', 'utf8');
  const indexHtmlPath = resolve(outDir, 'index.html');
  if (existsSync(indexHtmlPath)) {
    copyFileSync(indexHtmlPath, resolve(outDir, '404.html'));
  }
  process.stdout.write(
    `[Docs API] Wrote GitHub Pages .nojekyll and 404.html fallback -> ${outDir}\n`,
  );
}

main();
