import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

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

describe('build-api-docs pipeline', () => {
  it('generates TS API reference pages for all 7 @ng-desktop/* packages plus .nojekyll and 404.html', () => {
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
    } finally {
      rmSync(outDir, { recursive: true, force: true });
    }
  });
});
