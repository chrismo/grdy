#!/usr/bin/env bash
set -euo pipefail

REPO="chrismo/grdy"

# Resolve script location so it works from any directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TAP_ROOT="$REPO_ROOT/../homebrew-grdy"
FORMULA_PATH="$TAP_ROOT/Formula/grdy.rb"

if [ ! -d "$TAP_ROOT" ]; then
    echo "Error: tap repo not found at $TAP_ROOT" >&2
    echo "Clone it with: git clone git@github.com:chrismo/homebrew-grdy.git $TAP_ROOT" >&2
    exit 1
fi

# Determine version tag
if [ $# -ge 1 ]; then
    TAG="$1"
else
    echo "Fetching latest release tag..."
    TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
fi

if [ -z "$TAG" ]; then
    echo "Error: could not determine version tag" >&2
    exit 1
fi

# Strip leading 'v' for the version field
VERSION="${TAG#v}"

echo "Updating formula for $TAG (version $VERSION)..."

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Download macOS archives and compute checksums
for arch in x86_64 aarch64; do
    ARCHIVE="grdy-${TAG}-${arch}-apple-darwin.tar.gz"
    URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"
    echo "Downloading $ARCHIVE..."
    curl -fsSL "$URL" -o "$TMPDIR/$ARCHIVE"
done

SHA_X86=$(shasum -a 256 "$TMPDIR/grdy-${TAG}-x86_64-apple-darwin.tar.gz" | cut -d' ' -f1)
SHA_ARM=$(shasum -a 256 "$TMPDIR/grdy-${TAG}-aarch64-apple-darwin.tar.gz" | cut -d' ' -f1)

echo "x86_64 SHA256:  $SHA_X86"
echo "aarch64 SHA256: $SHA_ARM"

# Rewrite the formula
mkdir -p "$TAP_ROOT/Formula"
cat > "$FORMULA_PATH" <<EOF
class Grdy < Formula
  desc "CLI tool to render JSON data as tables"
  homepage "https://github.com/chrismo/grdy"
  license "BSD-3-Clause"
  version "$VERSION"

  on_intel do
    url "https://github.com/chrismo/grdy/releases/download/${TAG}/grdy-${TAG}-x86_64-apple-darwin.tar.gz"
    sha256 "$SHA_X86"
  end

  on_arm do
    url "https://github.com/chrismo/grdy/releases/download/${TAG}/grdy-${TAG}-aarch64-apple-darwin.tar.gz"
    sha256 "$SHA_ARM"
  end

  def install
    bin.install "grdy"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/grdy --version")
  end
end
EOF

echo "Updated $FORMULA_PATH to version $VERSION"
