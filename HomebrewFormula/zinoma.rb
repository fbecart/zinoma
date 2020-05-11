require  'formula'
class Zinoma < Formula
  version '0.2.0'
  desc "Make your build flow incremental"
  homepage "https://github.com/fbecart/zinoma"

  url "https://github.com/fbecart/zinoma/releases/download/#{version}/zinoma-#{version}-x86_64-apple-darwin.tar.gz"
  sha256 "5c83c1d4837ecab5b21da9d3491c4379151b282da68c96c9095107cf7b99f8e3"

  def install
    bin.install "zinoma"
  end
end
