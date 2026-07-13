#!/usr/bin/env ruby
# frozen_string_literal: true

version = ARGV.fetch(0)
checksum_dir = ARGV.fetch(1)
version_without_v = version.delete_prefix("v")
repo = "https://github.com/SainyTK/goog-cli"

targets = {
  "aarch64-apple-darwin" => "macos_arm",
  "x86_64-apple-darwin" => "macos_intel",
  "x86_64-unknown-linux-gnu" => "linux_intel",
  "aarch64-unknown-linux-gnu" => "linux_arm"
}

checksums = targets.keys.to_h do |target|
  asset = "goog-#{version}-#{target}.tar.gz"
  path = File.join(checksum_dir, "#{asset}.sha256")
  checksum = File.read(path).split.first
  raise "missing checksum for #{target}" if checksum.nil? || checksum.empty?

  [target, checksum]
end

puts <<~RUBY
  class Goog < Formula
    desc "Early Open-Source CLI for Google APIs"
    homepage "#{repo}"
    version "#{version_without_v}"

    on_macos do
      if Hardware::CPU.arm?
        url "#{repo}/releases/download/#{version}/goog-#{version}-aarch64-apple-darwin.tar.gz"
        sha256 "#{checksums.fetch("aarch64-apple-darwin")}"
      else
        url "#{repo}/releases/download/#{version}/goog-#{version}-x86_64-apple-darwin.tar.gz"
        sha256 "#{checksums.fetch("x86_64-apple-darwin")}"
      end
    end

    on_linux do
      if Hardware::CPU.arm?
        url "#{repo}/releases/download/#{version}/goog-#{version}-aarch64-unknown-linux-gnu.tar.gz"
        sha256 "#{checksums.fetch("aarch64-unknown-linux-gnu")}"
      else
        url "#{repo}/releases/download/#{version}/goog-#{version}-x86_64-unknown-linux-gnu.tar.gz"
        sha256 "#{checksums.fetch("x86_64-unknown-linux-gnu")}"
      end
    end

    def install
      bin.install "goog"
    end

    test do
      assert_match "goog", shell_output("\#{bin}/goog --help")
    end
  end
RUBY
