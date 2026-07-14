class ShellUse < Formula
  desc "Headless terminal CLI for driving, asserting on, and recording shells"
  homepage "https://github.com/microsoft/shell-use"
  version "0.0.1-beta.4"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-apple-darwin.tar.gz"
      sha256 "b8b3030d23e411dfdbb21dc2caf07dbafaa8fb506bb31db0a5689e88a4ba1511"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-apple-darwin.tar.gz"
      sha256 "cad4771ccce1700db36e07b8ea088dcbade3ed995b12c58a36350d5d5d67a5c8"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-unknown-linux-musl.tar.gz"
      sha256 "c8c3e3fa6728e5c982901ead423b09aa90660fe09066826a3388d6d40925543f"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-unknown-linux-musl.tar.gz"
      sha256 "8214c9d4c1c14d91f746fd4625e3706217b11b54f20831214383e5a5e3fd9b2c"
    end
  end

  def install
    bin.install "shell-use"
  end

  test do
    assert_match "shell-use", shell_output("#{bin}/shell-use --help")
  end
end
