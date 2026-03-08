#!/usr/bin/env node
// bin/run.js — thin JS wrapper; npm symlinks this into PATH as 'squad-station'
const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const binaryPath = path.join(__dirname, 'squad-station');

if (!fs.existsSync(binaryPath)) {
  console.error('squad-station binary not found.');
  console.error('Re-run: npm install -g squad-station');
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status != null ? result.status : 0);
