#!/bin/sh
set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version>" >&2
  echo "Example: $0 0.2.0" >&2
  exit 1
fi

if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: version must be in semver format (e.g. 0.2.0)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Update Cargo.toml
sed -i.bak "s/^version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$VERSION\"/" "$ROOT/Cargo.toml"
rm -f "$ROOT/Cargo.toml.bak"

# Update npm-package/package.json (version + binaryVersion)
sed -i.bak "s/\"version\": \"[0-9]*\.[0-9]*\.[0-9]*\"/\"version\": \"$VERSION\"/" "$ROOT/npm-package/package.json"
sed -i.bak "s/\"binaryVersion\": \"[0-9]*\.[0-9]*\.[0-9]*\"/\"binaryVersion\": \"$VERSION\"/" "$ROOT/npm-package/package.json"
rm -f "$ROOT/npm-package/package.json.bak"

echo "Bumped version to $VERSION in:"
echo "  Cargo.toml"
echo "  npm-package/package.json"
