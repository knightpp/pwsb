let
  pkgs = import <nixpkgs> {};

  mkShell = pkgs.mkShell.override {
    stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;
  };
in
  mkShell {
    name = "rust-dev-env";

    buildInputs = with pkgs; [
      rustup
      pkg-config
      openssl.dev
      zstd.dev
    ];

    env = {
      OPENSSL_DEV = pkgs.openssl.dev;
      ZSTD_SYS_USE_PKG_CONFIG = "1";
      OPENSSL_NO_VENDOR = "1";
    };
  }
