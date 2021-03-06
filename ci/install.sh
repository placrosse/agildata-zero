# `install` phase: install stuff needed for the `script` phase

set -ex
export NIGHTLY_VERSION=2016-09-12

case "$TRAVIS_OS_NAME" in
  linux)
    host=x86_64-unknown-linux-musl
    ;;
  osx)
    host=x86_64-apple-darwin
    ;;
esac

mktempd() {
  echo $(mktemp -d 2>/dev/null || mktemp -d -t tmp)
}

install_openssl() {
	export VERS=1.0.2g
	curl -O https://www.openssl.org/source/openssl-$VERS.tar.gz
	tar xzf openssl-$VERS.tar.gz
	cd openssl-$VERS
	env CC=musl-gcc ./config --prefix=/usr/local/musl
	env C_INCLUDE_PATH=/usr/local/musl/include/ make -s depend
	make
	sudo make -s install
	export OPENSSL_INCLUDE_DIR=/usr/local/musl/include/
	export OPENSSL_LIB_DIR=/usr/local/musl/lib/
	export OPENSSL_STATIC=1
	cd ..
}

install_musl() {
	git clone git://git.musl-libc.org/musl
	cd musl
	./configure
	make -s
	sudo make -s install
	cd ..
	export PATH=$PATH:/usr/local/musl/bin
	
	which musl-gcc
	ls -l /usr/local/musl/bin
}

install_rustup() {
  curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
  source ~/.cargo/env
  rustup override set nightly-${NIGHTLY_VERSION}
  rustup target add x86_64-unknown-linux-musl

#  curl -O https://static.rust-lang.org/rustup.sh
#  chmod +x rustup.sh
#  ./rustup.sh --yes --verbose
#  ./rustup.sh --channel=nightly --date=${NIGHTLY_VERSION}
#  ./rustup.sh --add-target=x86_64-unknown-linux-musl

  rustc -V
  cargo -V
}

install_standard_crates() {
  curl -O https://static.rust-lang.org/dist/${NIGHTLY_VERSION}/rust-std-nightly-${TARGET}.tar.gz
  tar -xf rust-std-nightly-${TARGET}.tar.gz
  cd rust-std-nightly-${TARGET}/
  sudo ./install.sh
  cd ..
}

main() {
  install_musl
  install_openssl
  install_rustup
  install_standard_crates

  # if you need to install extra stuff add it here
}

main
