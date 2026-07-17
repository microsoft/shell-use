class ShellUse < Formula
  desc "Headless terminal CLI for driving, asserting on, and recording shells"
  homepage "https://github.com/microsoft/shell-use"
  version "0.0.1-beta.5"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-apple-darwin.tar.gz"
      sha256 "75c0a4ff9fb7dc2f92d9579862156e2f5bba371cd93267e27f4a0d18b9a41127"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-apple-darwin.tar.gz"
      sha256 "35a13ff4ae2487a548b4f3918de70cfaafba7e50f4a9b49fbe36fa4d00dcabef"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-unknown-linux-musl.tar.gz"
      sha256 "c9426154a9e83c11c2f6a32145891504cf1a8ecf87d640f5c63af742d47db737"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-unknown-linux-musl.tar.gz"
      sha256 "0d2baa11bfde7a0b118a6b8836c829b87a31194d714484d36955ee65dbcf1dc2"
    end
  end

  def install
    bin.install "shell-use"
  end

  test do
    assert_match "shell-use", shell_output("#{bin}/shell-use --help")
  end
end
