#!/usr/bin/env bash
# `before_deploy` phase: here we package the build artifacts

set -ex

mktempd() {
  echo $(mktemp -d 2>/dev/null || mktemp -d -t tmp)
}

# Generate artifacts for release
mk_artifacts() {
  cargo build --target $TARGET --release
}

mk_tarball() {
  # create a "staging" directory
  local temp_dir=$(mktempd)
  local out_dir=$(pwd)

  # update this part to copy the artifacts that make sense for your project
  # NOTE All Cargo build artifacts will be under the 'target/$TARGET/{debug,release}'
  mkdir -p $temp_dir/agildata-zero
  cp target/$TARGET/release/agildata-zero $temp_dir/agildata-zero
  cp dist/zero-config.xml $temp_dir/agildata-zero
  cp dist/log.toml $temp_dir/agildata-zero
  cp dist/README.md $temp_dir/agildata-zero

  pushd $temp_dir

  # release tarball will look like 'agildata-zero-v0.1.0-x86_64-unknown-linux-musl.tar.gz'
  tar czf $out_dir/${PROJECT_NAME}-${TRAVIS_TAG}-${TARGET}.tar.gz *

  popd $temp_dir
  rm -r $temp_dir
}

main() {
  source ~/.cargo/env
  mk_artifacts
  mk_tarball
}

main
