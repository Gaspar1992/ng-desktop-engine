import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { ENGINE_BINARY_PACKAGES } from '../../../packages/builders-native/src/resolve-engine-binary.mjs';
import { PACKAGES_CATALOG } from './data/packages-catalog.data';
import { PRIMITIVES_CATALOG } from './data/primitives-catalog.data';

interface ApiSymbolRecord {
  readonly name: string;
  readonly kind: string;
}

interface ApiPackageRecord {
  readonly name: string;
  readonly slug: string;
  readonly symbolCount: number;
  readonly symbols: readonly ApiSymbolRecord[];
}

interface ApiIndexJson {
  readonly packages: readonly ApiPackageRecord[];
}

describe('build-api-docs pipeline and docs catalog parity', () => {
  it('generates TS API reference pages with responsive tables and extracts all 7 packages', () => {
    const repoRoot = resolve(process.cwd());
    const scriptPath = resolve(repoRoot, 'scripts/build-api-docs.mjs');
    const outDir = mkdtempSync(join(tmpdir(), 'ng-desktop-api-docs-'));

    try {
      const sentinelHtml = '<!doctype html><html><body>ng-desktop-docs-spa</body></html>\n';
      writeFileSync(join(outDir, 'index.html'), sentinelHtml, 'utf8');

      execFileSync(process.execPath, [scriptPath, '--skip-rust', '--out-dir', outDir], {
        cwd: repoRoot,
        stdio: 'pipe',
      });

      expect(existsSync(join(outDir, '.nojekyll'))).toBe(true);
      expect(readFileSync(join(outDir, '404.html'), 'utf8')).toBe(sentinelHtml);
      expect(existsSync(join(outDir, 'api-ts/styles.css'))).toBe(true);
      expect(existsSync(join(outDir, 'api-ts/index.html'))).toBe(true);

      const platformHtml = readFileSync(join(outDir, 'api-ts/platform.html'), 'utf8');
      expect(platformHtml).toContain('class="table-scroll"');
      expect(platformHtml).not.toContain('Public member');

      const indexJson = JSON.parse(
        readFileSync(join(outDir, 'api-ts/api-index.json'), 'utf8'),
      ) as ApiIndexJson;

      expect(indexJson.packages.length).toBe(7);

      const findPkg = (name: string): ApiPackageRecord | undefined =>
        indexJson.packages.find((pkg) => pkg.name === name);

      for (const pkg of indexJson.packages) {
        expect(pkg.symbolCount).toBeGreaterThan(0);
        expect(existsSync(join(outDir, `api-ts/${pkg.slug}.html`))).toBe(true);
      }

      const platformNames = new Set(findPkg('@ng-desktop/platform')?.symbols.map((s) => s.name));
      expect(platformNames.has('bootstrapNativeApplication')).toBe(true);
      expect(platformNames.has('parseRuntimeStyles')).toBe(true);

      const httpNames = new Set(findPkg('@ng-desktop/http')?.symbols.map((s) => s.name));
      expect(httpNames.has('createNativeHttpResource')).toBe(true);

      // Verify `.d.mts`-typed `.mjs` re-exports from @ng-desktop/builders are extracted cleanly.
      const builderNames = new Set(findPkg('@ng-desktop/builders')?.symbols.map((s) => s.name));
      expect(builderNames.has('compileBundle')).toBe(true);
      expect(builderNames.has('parseStylesCss')).toBe(true);
      expect(builderNames.has('analyzeProjectPermissions')).toBe(true);

      // Parity check: every package in docs catalog must match real packages/* or ENGINE_BINARY_PACKAGES,
      // and every documented library export must exist in extracted TypeScript symbols.
      const binaryNames = new Set(Object.values(ENGINE_BINARY_PACKAGES).map((b) => b.pkgName));
      for (const entry of PACKAGES_CATALOG) {
        if (entry.kind === 'binary') {
          expect(binaryNames.has(entry.name)).toBe(true);
        } else {
          const extracted = findPkg(entry.name);
          expect(extracted).toBeDefined();
          const exported = new Set(extracted?.symbols.map((s) => s.name));
          for (const exp of entry.exportsList) {
            expect(exported.has(exp)).toBe(true);
          }
        }
      }
    } finally {
      rmSync(outDir, { recursive: true, force: true });
    }
  });

  it('keeps every documented primitive canonicalTag aligned with src/engine/widget.rs', () => {
    const repoRoot = resolve(process.cwd());
    const widgetRs = readFileSync(resolve(repoRoot, 'src/engine/widget.rs'), 'utf8');
    for (const primitive of PRIMITIVES_CATALOG) {
      expect(widgetRs).toContain(`"${primitive.canonicalTag}"`);
    }
  });
});
