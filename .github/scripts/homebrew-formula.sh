#!/usr/bin/env bash
# Print the Homebrew formula for a release.
# Usage: homebrew-formula.sh VERSION CHECKSUM_DIR
# CHECKSUM_DIR holds the release's anzan-<target>.tar.gz.sha256 files.
set -euo pipefail

version=$1
dir=$2
base="https://github.com/BenjaminLucier/anzan/releases/download/v${version}"

sha() {
  local file="$dir/anzan-$1.tar.gz.sha256"
  [ -f "$file" ] || { echo "missing $file" >&2; exit 1; }
  cut -d ' ' -f 1 < "$file"
}

cat <<FORMULA
class Anzan < Formula
  desc "Terminal trainer for mental math"
  homepage "https://github.com/BenjaminLucier/anzan"
  version "${version}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "${base}/anzan-aarch64-apple-darwin.tar.gz"
      sha256 "$(sha aarch64-apple-darwin)"
    end
    on_intel do
      url "${base}/anzan-x86_64-apple-darwin.tar.gz"
      sha256 "$(sha x86_64-apple-darwin)"
    end
  end

  on_linux do
    on_arm do
      url "${base}/anzan-aarch64-unknown-linux-musl.tar.gz"
      sha256 "$(sha aarch64-unknown-linux-musl)"
    end
    on_intel do
      url "${base}/anzan-x86_64-unknown-linux-musl.tar.gz"
      sha256 "$(sha x86_64-unknown-linux-musl)"
    end
  end

  def install
    bin.install "anzan"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/anzan --version")
  end
end
FORMULA
