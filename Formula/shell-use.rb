class ShellUse < Formula
  desc "Headless terminal CLI for driving, asserting on, and recording shells"
  homepage "https://github.com/microsoft/shell-use"
  version "0.0.1-beta.3"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-apple-darwin.tar.gz"
      sha256 "cf6515e7400137dc0552c2f065fb416029dbe835d61dcd1bca6bbfdec58c3eee"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-apple-darwin.tar.gz"
      sha256 "053fde4fd4590df5719fe520b8ba26aa8f4adee4eeb3e852a7ebbc0f41f1da61"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-unknown-linux-musl.tar.gz"
      sha256 "247c72cf9b01f9ea06225f49f52c692e869e17378992ac4e7a6eae92f9ccc554"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-unknown-linux-musl.tar.gz"
      sha256 "08f6a88aa4de64d4097b0da720c89f2cd9c0de7af5a35feb84b644321747f36a"
    end
  end

  def install
    bin.install "shell-use"
  end

  test do
    assert_match "shell-use", shell_output("#{bin}/shell-use --help")
  end
end
