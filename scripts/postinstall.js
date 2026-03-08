#!/usr/bin/env node
// scripts/postinstall.js — zero external dependencies
// Downloads the correct platform binary from GitHub Releases on npm install.
const https = require('https');
const fs = require('fs');
const path = require('path');

const VERSION = require('../package.json').version;
const REPO = 'thientranhung/squad-station';

function getPlatformAssetName() {
  const platformMap = { darwin: 'darwin', linux: 'linux' };
  const archMap = { x64: 'x86_64', arm64: 'arm64' };

  const p = platformMap[process.platform];
  const a = archMap[process.arch];

  if (!p || !a) {
    console.error(`Unsupported platform: ${process.platform} ${process.arch}`);
    console.error(`Manual install: https://github.com/${REPO}/releases`);
    process.exit(1);
  }

  return `squad-station-${p}-${a}`;
}

function downloadFile(url, destPath, redirectCount) {
  if (redirectCount === undefined) redirectCount = 0;
  if (redirectCount > 5) {
    console.error('Too many redirects');
    process.exit(1);
  }
  return new Promise(function(resolve, reject) {
    https.get(url, { headers: { 'User-Agent': 'squad-station-installer' } }, function(res) {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return downloadFile(res.headers.location, destPath, redirectCount + 1)
          .then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        reject(new Error('HTTP ' + res.statusCode + ': ' + url));
        return;
      }
      var file = fs.createWriteStream(destPath);
      res.pipe(file);
      file.on('finish', function() { file.close(); resolve(); });
      file.on('error', reject);
    }).on('error', reject);
  });
}

async function main() {
  const assetName = getPlatformAssetName();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${assetName}`;
  const destDir = path.join(__dirname, '..', 'bin');
  const destPath = path.join(destDir, 'squad-station');

  fs.mkdirSync(destDir, { recursive: true });
  console.log('Downloading ' + assetName + '...');

  try {
    await downloadFile(url, destPath);
    fs.chmodSync(destPath, 0o755);
    console.log('squad-station installed successfully');
  } catch (err) {
    console.error('Download failed: ' + err.message);
    console.error('Manual install: https://github.com/' + REPO + '/releases');
    process.exit(1);
  }
}

main();
