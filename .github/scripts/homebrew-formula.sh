#!/usr/bin/env bash
# Print the Homebrew formula for a release.
# Usage: homebrew-formula.sh VERSION CHECKSUM_DIR
# CHECKSUM_DIR holds the release's anzan-<target>.sha256 files.
set -euo pipefail

version=$1
dir=$2
base="https://github.com/BenjaminLucier/anzan/releases/download/v${version}"

# The upload action writes `<hash> *anzan-<target>.tar.gz` to
# anzan-<target>.sha256. Read them all up front: a failure inside the
# heredoc's $(...) would not stop the script.
sha() {
  local file="$dir/anzan-$1.sha256" hash
  [ -f "$file" ] || { echo "missing $file" >&2; exit 1; }
  hash=$(cut -d ' ' -f 1 < "$file")
  [[ $hash =~ ^[0-9a-f]{64}$ ]] || { echo "no SHA-256 in $file" >&2; exit 1; }
  echo "$hash"
}

mac_arm=$(sha aarch64-apple-darwin)
mac_intel=$(sha x86_64-apple-darwin)
linux_arm=$(sha aarch64-unknown-linux-musl)
linux_intel=$(sha x86_64-unknown-linux-musl)

cat <<FORMULA
class Anzan < Formula
  desc "Terminal trainer for mental math"
  homepage "https://github.com/BenjaminLucier/anzan"
  version "${version}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "${base}/anzan-aarch64-apple-darwin.tar.gz"
      sha256 "${mac_arm}"
    end
    on_intel do
      url "${base}/anzan-x86_64-apple-darwin.tar.gz"
      sha256 "${mac_intel}"
    end
  end

  on_linux do
    on_arm do
      url "${base}/anzan-aarch64-unknown-linux-musl.tar.gz"
      sha256 "${linux_arm}"
    end
    on_intel do
      url "${base}/anzan-x86_64-unknown-linux-musl.tar.gz"
      sha256 "${linux_intel}"
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
