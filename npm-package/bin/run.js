#!/usr/bin/env node
// bin/run.js — entry point for npx squad-station
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
  var args = process.argv.slice(3);
  var force = args.includes('--force') || args.includes('-f');
  var tui = args.includes('--tui');

  // Step 1: Install binary
  var destPath = installBinary();

  // Step 2: Scaffold project files
  scaffoldProject(force);

  // Done — print next steps
  console.log('');
  if (tui) {
    // --tui: launch welcome TUI directly via the installed binary
    // spawnSync with stdio inherit passes the real terminal FDs through
    // Clean up any stale marker before running TUI
    var marker = path.join(require('os').tmpdir(), '.squad-project-dir');
    try { fs.unlinkSync(marker); } catch (_) {}

    var tuiResult = spawnSync(destPath, ['--tui'], { stdio: 'inherit' });
    if (tuiResult.status !== 0) {
      // TUI failed (no TTY, e.g. piped or non-interactive shell) — show static fallback
      spawnSync(destPath, [], { stdio: 'inherit' });
      console.log('\n  \x1b[33mTip:\x1b[0m Run \x1b[36msquad-station --tui\x1b[0m in your terminal for the interactive welcome.\n');
    } else {
      // Check if binary wrote a project directory marker (wizard created project elsewhere)
      try {
        var projectDir = fs.readFileSync(marker, 'utf8').trim();
        fs.unlinkSync(marker);
        if (projectDir && projectDir !== process.cwd()) {
          console.log('\n  \x1b[1mProject created at:\x1b[0m \x1b[36m' + projectDir + '\x1b[0m');
          console.log('  To enter your project, run:\n');
          console.log('  \x1b[36mcd ' + projectDir + '\x1b[0m\n');
        }
      } catch (_) {
        // No marker = project created in cwd, nothing to do
      }
    }
  } else {
    console.log('\x1b[1mNext steps:\x1b[0m');
    console.log('  1. Copy an example config:');
    console.log('     \x1b[36mcp .squad/examples/orchestrator-claude.yml squad.yml\x1b[0m');
    console.log('  2. Edit \x1b[36msquad.yml\x1b[0m — set project name, providers, models');
    console.log('  3. Run  \x1b[36msquad-station init\x1b[0m — launch tmux sessions\n');
  }
}

function installBinary() {
  var pkg = require('../package.json');
  var VERSION = pkg.binaryVersion || pkg.version;
  var REPO = 'Toof-X/squad-station-2';

  var platformMap = { darwin: 'darwin', linux: 'linux' };
  var archMap = { x64: 'x86_64', arm64: 'arm64' };

  var p = platformMap[process.platform];
  var a = archMap[process.arch];

  if (!p || !a) {
    console.error('Unsupported platform: ' + process.platform + ' ' + process.arch);
    console.error('Manual install: https://github.com/' + REPO + '/releases');
    process.exit(1);
  }

  var assetName = 'squad-station-' + p + '-' + a;
  var url = 'https://github.com/' + REPO + '/releases/download/v' + VERSION + '/' + assetName;

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

  var destPath = path.join(installDir, 'squad-station');

  // Check if binary already exists and is the right version
  if (fs.existsSync(destPath)) {
    try {
      var result = spawnSync(destPath, ['--version'], { encoding: 'utf8' });
      if (result.stdout && result.stdout.includes(VERSION)) {
        console.log('  \x1b[32m✓\x1b[0m squad-station v' + VERSION + ' already installed at ' + destPath);
        return destPath;
      }
    } catch (_) {
      // Can't check version, re-download
    }
  }

  console.log('  Downloading ' + assetName + ' v' + VERSION + '...');

  // Use curl (available on macOS/Linux) for simplicity
  var curlResult = spawnSync('curl', [
    '-fsSL', '--proto', '=https', '--tlsv1.2',
    '-o', destPath,
    url
  ], { stdio: ['ignore', 'pipe', 'pipe'] });

  if (curlResult.status !== 0) {
    var stderr = curlResult.stderr ? curlResult.stderr.toString() : '';
    console.error('  Download failed: ' + stderr);
    console.error('  Manual install: https://github.com/' + REPO + '/releases');
    process.exit(1);
  }

  fs.chmodSync(destPath, 0o755);
  console.log('  \x1b[32m✓\x1b[0m Installed squad-station to ' + destPath);

  if (fallback) {
    console.log('  \x1b[33m!\x1b[0m Add ~/.local/bin to your PATH if not already present.');
  }

  return destPath;
}

function scaffoldProject(force) {
  // Source: bundled .squad/ directory inside npm package
  var pkgRoot = path.join(__dirname, '..');
  var srcSquad = path.join(pkgRoot, '.squad');
  var destSquad = path.join(process.cwd(), '.squad');

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
  var binaryPath = null;

  // Try system-installed binary via PATH
  var which = spawnSync('which', ['squad-station'], { encoding: 'utf8' });
  if (which.status === 0 && which.stdout) {
    binaryPath = which.stdout.trim();
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
