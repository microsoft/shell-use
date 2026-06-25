class ShellUse < Formula
  desc "Headless terminal CLI for driving, asserting on, and recording shells"
  homepage "https://github.com/microsoft/shell-use"
  version "0.0.1-beta.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-apple-darwin.tar.gz"
      sha256 "55aa4fe313c21f54a782247f956d3625882f02684bade03caee98e668673412e"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-apple-darwin.tar.gz"
      sha256 "4a242afcacc3e7de1fd300b9a19d56565efc4d1721f3b41bbf3c83b4c6428b7c"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-unknown-linux-musl.tar.gz"
      sha256 "4073470f33836a8f8a29561cfe23628ad7d970f713bb2a783bdb6c0cc6c32bd4"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-unknown-linux-musl.tar.gz"
      sha256 "b9e1a571fbfa59f5882b813d2d41db64b7eb236df101ab2e2fabc38a2a7b3f48"
    end
  end

  def install
    bin.install "shell-use"
  end

  test do
    assert_match "shell-use", shell_output("#{bin}/shell-use --help")
  end
end
