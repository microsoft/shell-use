class ShellUse < Formula
  desc "Headless terminal CLI for driving, asserting on, and recording shells"
  homepage "https://github.com/microsoft/shell-use"
  version "0.0.1-beta.2"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-apple-darwin.tar.gz"
      sha256 "01822db7b5883bae44f559e4222d4d9e8695e6ae339cf69ab5f2dcd948423d0e"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-apple-darwin.tar.gz"
      sha256 "27c1f49c71707824a2d16914e45b8029ec59876d4357b33db87788bb0de8ec79"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-aarch64-unknown-linux-musl.tar.gz"
      sha256 "82f7fedf8f8cdc1d4bb48319d23318651c516c519fe19d9ab71e1948781447c5"
    end
    on_intel do
      url "https://github.com/microsoft/shell-use/releases/download/v#{version}/shell-use-x86_64-unknown-linux-musl.tar.gz"
      sha256 "c40590fb132cf22e4b675a241b16b1ab19fcffd6a76f4be1c2d6c2bec47f1113"
    end
  end

  def install
    bin.install "shell-use"
  end

  test do
    assert_match "shell-use", shell_output("#{bin}/shell-use --help")
  end
end
