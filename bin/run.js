#!/usr/bin/env node
// bin/run.js — entry point for npx / npm global install
// Handles "install" in JS (download binary + scaffold project files).
// All other subcommands proxy to the native Rust binary.

const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const subcommand = process.argv[2];

if (subcommand === 'install') {
  install();
} else {
  proxyToBinary();
}

// ── Install ─────────────────────────────────────────────────────────
// 1. Download squad-station binary to system PATH
// 2. Copy .squad/ project files to CWD

function install() {
  const args = process.argv.slice(3);
  const force = args.includes('--force') || args.includes('-f');

  console.log('\n\x1b[32m══════════════════════════════════\x1b[0m');
  console.log('  \x1b[1mSquad Station Install\x1b[0m');
  console.log('\x1b[32m══════════════════════════════════\x1b[0m\n');

  // Step 1: Install binary
  installBinary();

  // Step 2: Scaffold project files
  scaffoldProject(force);

  // Done
  console.log('\n\x1b[1mNext steps:\x1b[0m');
  console.log('  1. Copy an example config:');
  console.log('     \x1b[36mcp .squad/examples/orchestrator-claude.yml squad.yml\x1b[0m');
  console.log('  2. Edit \x1b[36msquad.yml\x1b[0m — set project name, providers, models');
  console.log('  3. Run  \x1b[36msquad-station init\x1b[0m — launch tmux sessions\n');
}

function installBinary() {
  const VERSION = require('../package.json').version;
  const REPO = 'Toof-X/squad-station-2';

  const platformMap = { darwin: 'darwin', linux: 'linux' };
  const archMap = { x64: 'x86_64', arm64: 'arm64' };

  const p = platformMap[process.platform];
  const a = archMap[process.arch];

  if (!p || !a) {
    console.error('Unsupported platform: ' + process.platform + ' ' + process.arch);
    console.error('Manual install: https://github.com/' + REPO + '/releases');
    process.exit(1);
  }

  const assetName = 'squad-station-' + p + '-' + a;
  const url = 'https://github.com/' + REPO + '/releases/download/v' + VERSION + '/' + assetName;

  // Determine install directory
  var installDir = '/usr/local/bin';
  var fallback = false;
  try {
    fs.accessSync(installDir, fs.constants.W_OK);
  } catch (_) {
    installDir = path.join(process.env.HOME || process.env.USERPROFILE || '~', '.local', 'bin');
    fallback = true;
    fs.mkdirSync(installDir, { recursive: true });
  }

  const destPath = path.join(installDir, 'squad-station');

  // Check if binary already exists and is the right version
  if (fs.existsSync(destPath)) {
    try {
      const result = spawnSync(destPath, ['--version'], { encoding: 'utf8' });
      if (result.stdout && result.stdout.includes(VERSION)) {
        console.log('  \x1b[32m✓\x1b[0m squad-station v' + VERSION + ' already installed at ' + destPath);
        return;
      }
    } catch (_) {
      // Can't check version, re-download
    }
  }

  console.log('  Downloading ' + assetName + ' v' + VERSION + '...');

  // Use curl (available on macOS/Linux) for simplicity
  const curlResult = spawnSync('curl', [
    '-fsSL', '--proto', '=https', '--tlsv1.2',
    '-o', destPath,
    url
  ], { stdio: ['ignore', 'pipe', 'pipe'] });

  if (curlResult.status !== 0) {
    const stderr = curlResult.stderr ? curlResult.stderr.toString() : '';
    console.error('  Download failed: ' + stderr);
    console.error('  Manual install: https://github.com/' + REPO + '/releases');
    process.exit(1);
  }

  fs.chmodSync(destPath, 0o755);
  console.log('  \x1b[32m✓\x1b[0m Installed squad-station to ' + destPath);

  if (fallback) {
    console.log('  \x1b[33m!\x1b[0m Add ~/.local/bin to your PATH if not already present.');
  }
}

function scaffoldProject(force) {
  // Source: bundled .squad/ directory inside npm package
  const pkgRoot = path.join(__dirname, '..');
  const srcSquad = path.join(pkgRoot, '.squad');
  const destSquad = path.join(process.cwd(), '.squad');

  console.log('');

  // Copy sdd/ playbooks
  var sddSrc = path.join(srcSquad, 'sdd');
  var sddDest = path.join(destSquad, 'sdd');
  fs.mkdirSync(sddDest, { recursive: true });

  var sddFiles = fs.readdirSync(sddSrc).filter(function(f) { return f.endsWith('.md'); });
  sddFiles.forEach(function(file) {
    var dest = path.join(sddDest, file);
    if (fs.existsSync(dest) && !force) {
      console.log('  \x1b[33m–\x1b[0m .squad/sdd/' + file + ' \x1b[2m(exists, use --force to overwrite)\x1b[0m');
    } else {
      fs.copyFileSync(path.join(sddSrc, file), dest);
      console.log('  \x1b[32m✓\x1b[0m .squad/sdd/' + file);
    }
  });

  // Copy examples/
  var exSrc = path.join(srcSquad, 'examples');
  var exDest = path.join(destSquad, 'examples');
  fs.mkdirSync(exDest, { recursive: true });

  var exFiles = fs.readdirSync(exSrc).filter(function(f) { return f.endsWith('.yml'); });
  exFiles.forEach(function(file) {
    var dest = path.join(exDest, file);
    if (fs.existsSync(dest) && !force) {
      console.log('  \x1b[33m–\x1b[0m .squad/examples/' + file + ' \x1b[2m(exists, use --force to overwrite)\x1b[0m');
    } else {
      fs.copyFileSync(path.join(exSrc, file), dest);
      console.log('  \x1b[32m✓\x1b[0m .squad/examples/' + file);
    }
  });
}

// ── Proxy ───────────────────────────────────────────────────────────
// Forward all non-install subcommands to the native binary.

function proxyToBinary() {
  // Look for binary in system PATH first, then fallback to local bin/
  var binaryPath = null;

  // Try system-installed binary
  var which = spawnSync('which', ['squad-station'], { encoding: 'utf8' });
  if (which.status === 0 && which.stdout) {
    binaryPath = which.stdout.trim();
  }

  // Fallback: local binary (from postinstall)
  if (!binaryPath) {
    var localBin = path.join(__dirname, 'squad-station');
    if (fs.existsSync(localBin)) {
      binaryPath = localBin;
    }
  }

  if (!binaryPath) {
    console.error('squad-station binary not found.');
    console.error('Run: npx squad-station install');
    process.exit(1);
  }

  var result = spawnSync(binaryPath, process.argv.slice(2), { stdio: 'inherit' });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  process.exit(result.status != null ? result.status : 0);
}
