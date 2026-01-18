#!/usr/bin/env node
/**
 * Sync version from Cargo workspace to package.json
 * Called as pre-release-hook by cargo-release
 *
 * Environment variables from cargo-release:
 * - NEW_VERSION: The new version being released
 * - PREV_VERSION: The previous version
 */

const fs = require('fs');
const path = require('path');

// Get new version from cargo-release environment variable
const newVersion = process.env.NEW_VERSION;
const prevVersion = process.env.PREV_VERSION;

if (!newVersion) {
  console.error('ERROR: NEW_VERSION environment variable not set');
  console.error('This script should be run by cargo-release');
  process.exit(1);
}

console.log(`Syncing version: ${prevVersion || '?'} → ${newVersion}`);

// Find workspace root by looking for Cargo.toml with [workspace]
function findWorkspaceRoot(startDir) {
  let currentDir = startDir;

  while (true) {
    const cargoToml = path.join(currentDir, 'Cargo.toml');

    if (fs.existsSync(cargoToml)) {
      const content = fs.readFileSync(cargoToml, 'utf8');
      if (content.includes('[workspace]')) {
        return currentDir;
      }
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      // Reached filesystem root
      console.error('ERROR: Could not find workspace root');
      process.exit(1);
    }
    currentDir = parentDir;
  }
}

const workspaceRoot = findWorkspaceRoot(process.cwd());
console.log(`Workspace root: ${workspaceRoot}`);

// Files to update (relative to workspace root)
const filesToUpdate = [
  {
    path: 'web/package.json',
    type: 'json'
  },
  {
    path: 'dist-workspace.toml',
    type: 'toml',
    pattern: /^cargo-dist-version\s*=\s*"[^"]*"/m,
    // Keep cargo-dist version separate - don't update
    skip: true
  }
];

let updatedCount = 0;
let errors = [];

for (const file of filesToUpdate) {
  const fullPath = path.join(workspaceRoot, file.path);

  if (file.skip) {
    console.log(`  Skipping ${file.path} (managed separately)`);
    continue;
  }

  if (!fs.existsSync(fullPath)) {
    console.warn(`  Warning: ${file.path} not found, skipping`);
    continue;
  }

  try {
    if (file.type === 'json') {
      // Update JSON files (package.json)
      const content = fs.readFileSync(fullPath, 'utf8');
      const json = JSON.parse(content);

      if (json.version === newVersion) {
        console.log(`  ✓ ${file.path} already at ${newVersion}`);
        continue;
      }

      json.version = newVersion;

      // Write with same formatting (2 spaces, newline at end)
      fs.writeFileSync(fullPath, JSON.stringify(json, null, 2) + '\n', 'utf8');
      console.log(`  ✓ Updated ${file.path}: ${json.version || '?'} → ${newVersion}`);
      updatedCount++;

    } else if (file.type === 'toml') {
      // Update TOML files
      let content = fs.readFileSync(fullPath, 'utf8');

      if (file.pattern) {
        const match = content.match(file.pattern);
        if (match) {
          const newLine = match[0].replace(/"[^"]*"/, `"${newVersion}"`);
          content = content.replace(file.pattern, newLine);
          fs.writeFileSync(fullPath, content, 'utf8');
          console.log(`  ✓ Updated ${file.path}`);
          updatedCount++;
        }
      }
    }
  } catch (error) {
    const errorMsg = `Failed to update ${file.path}: ${error.message}`;
    console.error(`  ✗ ${errorMsg}`);
    errors.push(errorMsg);
  }
}

console.log();
console.log(`Summary: Updated ${updatedCount} file(s)`);

if (errors.length > 0) {
  console.error(`Errors encountered: ${errors.length}`);
  errors.forEach(err => console.error(`  - ${err}`));
  process.exit(1);
}

console.log('Version sync complete!');
process.exit(0);
