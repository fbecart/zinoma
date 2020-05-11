#!/bin/sh

set -e

if [ $# != 1 ]; then
  echo "Usage: $(basename $0) version" >&2
  exit 1
fi
version="$1"

# Linux and Darwin builds.
arch=x86_64
for target in apple-darwin unknown-linux; do
  url="https://github.com/fbecart/zinoma/releases/download/$version/zinoma-$version-$arch-$target.tar.gz"
  sha=$(curl -sfSL "$url" | sha256sum)
  echo "$version-$arch-$target $sha"
done

# Source.
for ext in zip tar.gz; do
  url="https://github.com/fbecart/zinoma/archive/$version.$ext"
  sha=$(curl -sfSL "$url" | sha256sum)
  echo "source.$ext $sha"
done
