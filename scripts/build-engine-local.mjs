#!/usr/bin/env node
// Compiles the Rust engine (cargo --release) and stages it inside the host's
// @ng-desktop/engine-* npm package (dist/engine-packages/<dir>/bin).
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ENGINE_BINARY_PACKAGES } from '../packages/builders-native/src/resolve-engine-binary.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const key = `${process.platform}-${process.arch}`;
const meta = ENGINE_BINARY_PACKAGES[key];
if (!meta) throw new Error(`Unsupported host platform: ${key}`);

const run = (cmd, args) => {
  const r = spawnSync(cmd, args, { cwd: root, stdio: 'inherit' });
  if (r.status !== 0) process.exit(r.status ?? 1);
};
run('cargo', ['build', '--release']);
run('node', [
  'scripts/build-engine-packages.mjs',
  `--target=${key}`,
  `--binary=target/release/${meta.binaryName}`,
  ...process.argv.slice(2),
]);
