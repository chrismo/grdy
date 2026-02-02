#!/usr/bin/env bash
set -euo pipefail

if [ $# -eq 0 ]; then
    echo "Usage: $0 <version>" >&2
    echo "Example: $0 0.1.0" >&2
    exit 1
fi

VERSION="$1"
TAG="v$VERSION"

# Ensure we're on main
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "main" ]; then
    echo "Warning: not on main branch (currently on '$BRANCH')" >&2
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "Error: uncommitted changes present" >&2
    exit 1
fi

# Update version in Cargo.toml
echo "Updating Cargo.toml to version $VERSION..."
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

# Commit the version bump
git add Cargo.toml
git commit -m "Bump version to $VERSION"

echo "Creating tag $TAG..."
git tag "$TAG"

echo "Pushing commit and tag..."
git push origin "$BRANCH"
git push origin "$TAG"

echo "Released $TAG"
