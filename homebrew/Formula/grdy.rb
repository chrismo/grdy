class Grdy < Formula
  desc "CLI tool to render JSON data as tables"
  homepage "https://github.com/chrismo/grdy"
  license "BSD-3-Clause"
  version "0.4.0"

  on_intel do
    url "https://github.com/chrismo/grdy/releases/download/v0.4.0/grdy-v0.4.0-x86_64-apple-darwin.tar.gz"
    sha256 "d725a72f5335736e29ab2507e526a115c2a49db98b152855728f14f614b8a02e"
  end

  on_arm do
    url "https://github.com/chrismo/grdy/releases/download/v0.4.0/grdy-v0.4.0-aarch64-apple-darwin.tar.gz"
    sha256 "c0a02bcba12d7d07c2d5b9f84fef11aafc30a124c589f04a0e682d66f8437760"
  end

  def install
    bin.install "grdy"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/grdy --version")
  end
end
