#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "========================================"
echo "  CCG Gateway Release Tool"
echo "========================================"
echo ""

# Check for uncommitted changes
if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "[ERROR] Working tree has uncommitted changes. Please commit or stash first."
    echo ""
    git status --short
    exit 1
fi

# Show current versions
echo "[Current Versions]"
echo "  tauri.conf.json : $(grep -o '"version": "[^"]*"' src-tauri/tauri.conf.json | head -1 | cut -d'"' -f4)"
echo "  Cargo.toml      : $(grep '^version' src-tauri/Cargo.toml | head -1 | cut -d'"' -f2)"
echo "  package.json    : $(grep -o '"version": "[^"]*"' frontend/package.json | head -1 | cut -d'"' -f4)"
echo ""

# Get version
read -rp "New version (e.g. 1.2.0): " VERSION
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "[ERROR] Invalid version format. Expected: X.Y.Z"
    exit 1
fi

# Check if tag already exists
if git tag -l "v${VERSION}" | grep -q .; then
    echo "[ERROR] Tag v${VERSION} already exists"
    exit 1
fi

# Get release notes (multi-line)
echo ""
echo "Release notes (empty line to finish):"
NOTES=""
while IFS= read -r line; do
    [[ -z "$line" ]] && break
    if [[ -z "$NOTES" ]]; then
        NOTES="$line"
    else
        NOTES="$NOTES
$line"
    fi
done
if [[ -z "$NOTES" ]]; then
    echo "[ERROR] Release notes cannot be empty"
    exit 1
fi

# Confirm
echo ""
echo "========================================"
echo "  Version : v${VERSION}"
echo "  Notes   :"
echo "$NOTES" | sed 's/^/    /'
echo "========================================"
echo ""
read -rp "Confirm release? (y/N): " CONFIRM
if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
    echo "[Cancelled]"
    exit 0
fi
echo ""

# Update versions
echo "[1/6] Updating tauri.conf.json ..."
sed -i "0,/\"version\": \"[^\"]*\"/{s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/}" src-tauri/tauri.conf.json

echo "[2/6] Updating Cargo.toml ..."
sed -i "0,/^version = \".*\"/s//version = \"${VERSION}\"/" src-tauri/Cargo.toml

echo "[3/6] Updating package.json ..."
sed -i "0,/\"version\": \"[^\"]*\"/{s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/}" frontend/package.json

# Git commit
echo "[4/6] Committing version change ..."
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml frontend/package.json
git commit -m "version ${VERSION}"

# Create annotated tag
echo "[5/6] Creating tag v${VERSION} ..."
git tag -a "v${VERSION}" -m "$NOTES"

# Push
echo "[6/6] Pushing to remote ..."
git push && git push --tags

echo ""
echo "========================================"
echo "  Release v${VERSION} pushed successfully!"
echo "  CI/CD will start building automatically."
echo "========================================"
