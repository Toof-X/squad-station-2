# Phase 8: npm Package - Research

**Researched:** 2026-03-08
**Domain:** npm packaging for native Rust binaries, postinstall scripts, platform detection
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| NPM-01 | `npm install -g squad-station` installs the binary globally | postinstall pattern downloads correct binary; bin field makes it globally available |
| NPM-02 | Postinstall script detects OS + CPU architecture and downloads correct binary from GitHub Releases | `process.platform` / `process.arch` → asset name mapping; built-in `https` module for download |
| NPM-03 | Binary placed in npm bin directory, executable without extra steps | bin field points to JS wrapper; wrapper resolves and spawns the native binary; `fs.chmodSync` sets +x |
| NPM-04 | `package.json` correctly configured: `bin`, `version`, `repository`, `engines` fields | Standard npm field semantics documented; version must match Cargo.toml; engines `node >= 14` |
</phase_requirements>

---

## Summary

The goal is to make `npm install -g squad-station` download and wire up the correct pre-built Rust binary for the user's platform. npm itself has no native understanding of compiled binaries, so the standard approach is a two-part wrapper: a `postinstall` script that runs at install time to download and place the binary, and a thin JS `bin` entry-point that npm symlinks into PATH which then exec-spawns the native binary.

There are two mainstream strategies: (1) a `postinstall`-only approach where the install script downloads the binary from GitHub Releases using Node's built-in `https` module, and (2) an `optionalDependencies` approach where each platform ships its binary as a separate scoped npm package and the package manager selects the right one automatically. The optionalDependencies approach (pioneered by esbuild) is more robust against network issues and postinstall-disabled environments, but requires publishing N+1 packages to the npm registry. For a project where the binaries already live on GitHub Releases, the postinstall approach is simpler and proven — it is used by tools like `@vscode/ripgrep`, `wasm-pack`, and many others.

**Primary recommendation:** Use the postinstall-download approach. Write a zero-dependency `scripts/postinstall.js` that uses Node's built-in `https` module to download the correct binary from GitHub Releases, set it executable, and write a JS `bin/run.js` entry-point that resolves and spawns it. Publish a single package — no sub-packages needed.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Node.js built-in `https` | N/A | Download binary from GitHub Releases | Zero dependencies; always available |
| Node.js built-in `fs` | N/A | Write file, `chmodSync` to set +x | Same reason |
| Node.js built-in `path` | N/A | Resolve binary path relative to package | Portability |
| Node.js built-in `child_process` `spawnSync` | N/A | Exec native binary in `bin/run.js` | Standard pattern; forwards stdin/stdout/stderr cleanly |

### No External Dependencies Required
The postinstall script and bin wrapper need zero npm dependencies. Relying on `node-fetch`, `axios`, or any third-party library in a postinstall context creates a chicken-and-egg problem: dependencies install after postinstall runs in some edge cases. Using only Node built-ins avoids this entirely.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| postinstall download | `optionalDependencies` (esbuild style) | optionalDeps is more reliable when postinstall is disabled but requires publishing 4 extra packages to npm registry and a more complex publish workflow |
| postinstall download | `binary-install` npm package | Convenience but adds a dependency that may drift; for a minimal package, built-ins are preferable |

---

## Architecture Patterns

### Recommended File Structure

```
squad-station/              (project root)
├── package.json            # npm package manifest
├── scripts/
│   └── postinstall.js      # runs on npm install — downloads binary
└── bin/
    └── run.js              # JS entry-point; symlinked by npm into PATH
```

The `scripts/` and `bin/` directories are published to npm via the `files` whitelist in `package.json`. Nothing else (Rust source, tests, CI config) is included in the tarball.

### Pattern 1: package.json Core Fields

```json
{
  "name": "squad-station",
  "version": "0.1.0",
  "description": "Message routing and orchestration for AI agent squads",
  "repository": {
    "type": "git",
    "url": "https://github.com/OWNER/squad-station.git"
  },
  "bin": {
    "squad-station": "./bin/run.js"
  },
  "scripts": {
    "postinstall": "node scripts/postinstall.js"
  },
  "files": [
    "bin/",
    "scripts/"
  ],
  "engines": {
    "node": ">=14"
  },
  "license": "MIT"
}
```

Key points:
- `bin` key name matches the command users will type (`squad-station`)
- `bin` value is a JS file (npm requires a JS entry-point for bin fields, not a raw binary)
- `files` is a whitelist — only `bin/` and `scripts/` are published; Rust source, `.planning/`, CI config are excluded
- `engines` advertises minimum Node.js version; Node 14 is safe (LTS, widely available)
- `version` must match the tag used on GitHub Releases (e.g. `0.1.0` → `v0.1.0` tag)

### Pattern 2: Platform Detection in postinstall.js

```javascript
#!/usr/bin/env node
// scripts/postinstall.js — zero external dependencies
const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');

const VERSION = require('../package.json').version;
const REPO = 'OWNER/squad-station';

function getPlatformAssetName() {
  const platform = process.platform; // 'darwin', 'linux', 'win32'
  const arch = process.arch;         // 'x64', 'arm64', 'ia32'

  // Map Node platform/arch names to our binary naming convention:
  //   squad-station-darwin-arm64
  //   squad-station-darwin-x86_64
  //   squad-station-linux-arm64
  //   squad-station-linux-x86_64
  const platformMap = { darwin: 'darwin', linux: 'linux' };
  const archMap = { x64: 'x86_64', arm64: 'arm64' };

  const p = platformMap[platform];
  const a = archMap[arch];

  if (!p || !a) {
    console.error(`Unsupported platform: ${platform} ${arch}`);
    process.exit(1);
  }

  return `squad-station-${p}-${a}`;
}

function downloadFile(url, destPath, redirectCount = 0) {
  if (redirectCount > 5) {
    console.error('Too many redirects');
    process.exit(1);
  }
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { 'User-Agent': 'squad-station-installer' } }, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        // GitHub Releases always redirects to S3
        return downloadFile(res.headers.location, destPath, redirectCount + 1)
          .then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        return;
      }
      const file = fs.createWriteStream(destPath);
      res.pipe(file);
      file.on('finish', () => { file.close(); resolve(); });
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
  console.log(`Downloading ${assetName}...`);

  try {
    await downloadFile(url, destPath);
    fs.chmodSync(destPath, 0o755); // must be +x
    console.log('squad-station installed successfully');
  } catch (err) {
    console.error(`Download failed: ${err.message}`);
    console.error(`Manual install: https://github.com/${REPO}/releases`);
    process.exit(1);
  }
}

main();
```

### Pattern 3: bin/run.js (JS entry-point)

```javascript
#!/usr/bin/env node
// bin/run.js — thin wrapper; npm symlinks this into PATH
const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const binaryPath = path.join(__dirname, 'squad-station');

if (!fs.existsSync(binaryPath)) {
  console.error('squad-station binary not found. Re-run: npm install -g squad-station');
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 0);
```

The `#!/usr/bin/env node` shebang is mandatory — without it npm will not set up the symlink correctly and the command will fail with "permission denied" or run as a shell script.

### Anti-Patterns to Avoid

- **Raw binary in `bin` field**: npm's `bin` field expects a JS file with a shebang. Pointing directly at a native binary causes silent failures on some systems and breaks the PATH symlink on Windows.
- **Using `npm_config_prefix` to place the binary**: This is fragile; the postinstall script cannot reliably compute where npm's global bin directory is. Instead, install the binary relative to the package directory and have the JS wrapper find it.
- **External dependencies in postinstall**: `node-fetch`, `got`, `axios`, etc. are not available when postinstall runs if npm hasn't finished installing dependencies. Built-in `https` only.
- **Not handling GitHub's redirect**: GitHub Releases URLs always redirect (302) to S3. The `https.get` call must follow this redirect manually or the download will get a 0-byte redirect response.
- **Forgetting `chmod +x`**: GitHub Actions artifacts lose executable bits. The postinstall script must call `fs.chmodSync(path, 0o755)` after download.
- **Hardcoding the version string**: The version in `postinstall.js` should be read from `package.json` (`require('../package.json').version`) so it stays in sync automatically.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Binary path resolution across npm local/global installs | Complex PATH scanning | `__dirname` relative to `bin/run.js` | npm guarantees the package directory layout; binary placed next to run.js is always findable |
| Multi-package optionalDependencies system | Custom platform sub-packages | Use postinstall for this project | Over-engineering for 4 targets; postinstall is well-understood and sufficient |
| Checksum verification | SHA256 download + verify | Out of scope (VER-01, VER-02 deferred to v1.3) | Adds complexity; requirements explicitly exclude it |
| Tarball extraction | Unpacking .tar.gz in JS | N/A — GitHub release assets are plain binaries, not archives | Phase 7 workflow uploads raw binary files, not archives |

---

## Common Pitfalls

### Pitfall 1: GitHub Releases HTTP Redirect Not Followed
**What goes wrong:** `https.get` receives a 302 redirect to S3 with an empty body. The downloaded file is 0 bytes. The binary fails to execute.
**Why it happens:** GitHub's release download URLs are short-form redirects. Standard `https.get` does not follow redirects automatically.
**How to avoid:** Implement redirect following in the download function (check `statusCode === 301 || 302`, recurse with `res.headers.location`). Limit to 5 redirects.
**Warning signs:** Binary file exists but is 0 bytes; `squad-station --version` fails with "Exec format error".

### Pitfall 2: Missing +x Permission
**What goes wrong:** `squad-station --version` fails with "permission denied".
**Why it happens:** GitHub Actions upload/download actions strip executable bits from artifacts. The downloaded file has 0644 permissions.
**How to avoid:** Always call `fs.chmodSync(binaryPath, 0o755)` immediately after the download completes.
**Warning signs:** File exists, has size > 0, but chmod is 0644.

### Pitfall 3: Binary Naming Mismatch
**What goes wrong:** Download URL 404s — binary not found on the release.
**Why it happens:** Phase 7 established a specific naming convention: `squad-station-{os}-{arch}` where os is `darwin`/`linux` and arch is `arm64`/`x86_64`. Node.js `process.arch` returns `x64` not `x86_64`.
**How to avoid:** Use explicit mapping: `{ x64: 'x86_64', arm64: 'arm64' }`. Never use `process.arch` directly in the URL.
**Warning signs:** 404 on download; check the exact asset names on the GitHub Release.

### Pitfall 4: Version Mismatch Between package.json and Git Tag
**What goes wrong:** Postinstall downloads from `v0.1.0` but the release tag on GitHub is `v0.2.0`, or vice versa.
**Why it happens:** Version in package.json, Cargo.toml, and git tag can drift.
**How to avoid:** The release URL uses `package.json` version. The publish workflow must ensure git tag matches `package.json` version. Document the release process.
**Warning signs:** 404 on download despite binary existing on GitHub.

### Pitfall 5: `npm install -g` in CI Without Network
**What goes wrong:** postinstall fails silently in offline CI environments.
**Why it happens:** Some CI setups use `--ignore-scripts` or offline mode.
**How to avoid:** Postinstall should exit with a non-zero code and a helpful error message pointing to the GitHub Releases page for manual download. This makes failure visible.
**Warning signs:** `squad-station` command not found after install in CI.

### Pitfall 6: `bin/run.js` Missing Shebang
**What goes wrong:** Running `squad-station` opens a shell error or the JS file is executed as shell script.
**Why it happens:** npm uses the shebang line (`#!/usr/bin/env node`) to determine the interpreter when symlinking. Without it, the OS defaults to `/bin/sh`.
**How to avoid:** Always include `#!/usr/bin/env node` as the first line of `bin/run.js`.

---

## Code Examples

Verified patterns from official sources and real-world packages:

### Platform Mapping (Node.js built-ins)
```javascript
// Source: Node.js docs - process.platform values
// darwin = macOS, linux = Linux, win32 = Windows (not relevant for this project)
const platformMap = {
  darwin: 'darwin',
  linux: 'linux',
};
// Source: Node.js docs - process.arch values
// x64 = 64-bit x86, arm64 = 64-bit ARM
const archMap = {
  x64: 'x86_64',  // maps to Phase 7 binary naming convention
  arm64: 'arm64',
};
```

### GitHub Release URL Pattern (matches Phase 7 convention)
```
https://github.com/{owner}/{repo}/releases/download/v{version}/squad-station-{os}-{arch}
```
Example: `https://github.com/OWNER/squad-station/releases/download/v0.1.0/squad-station-darwin-arm64`

### spawnSync for Binary Execution
```javascript
// Source: Node.js child_process docs
// stdio: 'inherit' passes stdin/stdout/stderr directly — crucial for TUI and interactive output
const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status ?? 0);
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| postinstall-only download | optionalDependencies (esbuild-style) | ~2021 (esbuild 0.13.0) | More reliable when scripts disabled, but requires N+1 packages |
| Shell script wrappers | JS file with `#!/usr/bin/env node` shebang | Pre-2020 | Cross-platform; works on Windows (irrelevant here but standard) |
| `curl` in postinstall | Node.js built-in `https` module | Pre-2020 | No shell dependency; works where curl is not present |

**Current standard for this use case:** For tools targeting developers who already have Node.js, the postinstall-download approach with built-in `https` is the most widely deployed pattern. Used by: `@vscode/ripgrep`, `wasm-pack` npm wrapper, many Rust CLI tools.

---

## Open Questions

1. **GitHub Repository Name/Owner**
   - What we know: The binary naming convention is `squad-station-{os}-{arch}`; Phase 7 uses `softprops/action-gh-release@v2`
   - What's unclear: The exact GitHub org/username to use in the download URL
   - Recommendation: Read from `package.json` `repository` field at runtime, or hardcode after confirming the repo URL

2. **npm Account and Package Name Availability**
   - What we know: Package name `squad-station` is the intended name
   - What's unclear: Whether `squad-station` is taken on the npm registry
   - Recommendation: Check `npm info squad-station` before planning the publish step; if taken, use a scoped name like `@owner/squad-station`

3. **Version Synchronization Strategy**
   - What we know: Cargo.toml has `version = "0.1.0"`; git tags are `v*`
   - What's unclear: Whether the publish workflow should auto-update `package.json` version from the git tag, or require manual sync
   - Recommendation: The plan should include a task for keeping `package.json` version in sync with `Cargo.toml` version; simplest approach is to maintain them manually since both are in the same repo

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (58 unit + integration tests) + `tests/e2e_cli.sh` |
| Config file | `Cargo.toml` (no separate test config) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && ./tests/e2e_cli.sh` |

Note: Phase 8 work is primarily JavaScript (postinstall script) and configuration files (package.json). The existing Rust test suite does not cover npm packaging. Validation for this phase is manual smoke testing with `npm install -g .` (local) and checking `squad-station --version`.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| NPM-01 | `npm install -g squad-station` completes | smoke | `npm pack && npm install -g ./squad-station-*.tgz` | ❌ Wave 0 |
| NPM-02 | Postinstall downloads correct platform binary | smoke/manual | `node scripts/postinstall.js` (local dry run) | ❌ Wave 0 |
| NPM-03 | Binary is executable after install | smoke | `squad-station --version` after global install | ❌ Wave 0 |
| NPM-04 | package.json fields are correct | static | `node -e "const p=require('./package.json'); ..."` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `node -e "const p=require('./package.json'); ['bin','version','repository','engines'].forEach(f => { if(!p[f]) throw new Error('missing: '+f); }); console.log('package.json OK')"`
- **Per wave merge:** `npm pack --dry-run` to verify tarball contents
- **Phase gate:** Local `npm install -g .` smoke test before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `scripts/postinstall.js` — must be created (Wave 1 deliverable)
- [ ] `bin/run.js` — must be created (Wave 1 deliverable)
- [ ] `package.json` — must be created at project root (Wave 1 deliverable)
- No test framework install needed — validation is smoke testing only; `npm pack` is built into npm

---

## Sources

### Primary (HIGH confidence)
- Node.js `https` module docs — built-in HTTP/HTTPS request handling with redirect behavior
- Node.js `child_process.spawnSync` docs — process spawning with stdio inheritance
- Node.js `fs.chmodSync` docs — setting file permissions
- [npm package.json docs](https://docs.npmjs.com/cli/v11/configuring-npm/package-json) — `bin`, `files`, `engines`, `os`, `cpu` field semantics

### Secondary (MEDIUM confidence)
- [Sentry Engineering: Publishing Native Binaries on npm](https://sentry.engineering/blog/publishing-binaries-on-npm) — verified pattern combining optionalDependencies + postinstall fallback; `chmod +x` requirement confirmed
- [Orhun's Blog: Packaging Rust for npm](https://blog.orhun.dev/packaging-rust-for-npm/) — publishing order, optionalDependencies architecture, platform naming pitfalls
- [esbuild optionalDependencies PR #1621](https://github.com/evanw/esbuild/pull/1621) — esbuild's canonical reference for the optionalDeps approach
- [@vscode/ripgrep on npm](https://www.npmjs.com/package/@vscode/ripgrep) — real-world postinstall binary download from GitHub Releases

### Tertiary (LOW confidence)
- [woubuc: Publishing a Rust binary on npm](https://www.woubuc.be/blog/post/publishing-rust-binary-on-npm/) — older article but confirms bin/run.js + preinstall/postinstall pattern; preinstall gotcha flagged
- [cloudflare/binary-install](https://github.com/cloudflare/binary-install) — install/run/uninstall lifecycle; note: adds a dependency, not recommended for this project

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Node.js built-ins; no third-party libraries needed; patterns verified across multiple real-world packages
- Architecture: HIGH — package.json field semantics and postinstall pattern are stable npm features
- Pitfalls: HIGH — GitHub redirect behavior, chmod requirement, naming mismatch are all documented in official sources and real-world examples

**Research date:** 2026-03-08
**Valid until:** 2026-06-08 (npm package.json spec is stable; postinstall behavior has been consistent for years)
