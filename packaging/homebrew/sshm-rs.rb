class SsshmRs < Formula
  desc "SSH connection manager with integrated terminal and SFTP browser"
  homepage "https://github.com/bit5hift/sshm-rs"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/bit5hift/sshm-rs/releases/download/v#{version}/sshm-rs-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_PLACEHOLDER_AARCH64_APPLE_DARWIN__"
    else
      url "https://github.com/bit5hift/sshm-rs/releases/download/v#{version}/sshm-rs-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_PLACEHOLDER_X86_64_APPLE_DARWIN__"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/bit5hift/sshm-rs/releases/download/v#{version}/sshm-rs-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_PLACEHOLDER_AARCH64_LINUX__"
    else
      url "https://github.com/bit5hift/sshm-rs/releases/download/v#{version}/sshm-rs-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_PLACEHOLDER_X86_64_LINUX__"
    end
  end

  def install
    bin.install "sshm-rs"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sshm-rs --version")
  end
end
