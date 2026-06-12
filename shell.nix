{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    llvmPackages_21.llvm
    llvmPackages_21.libllvm
    llvmPackages_21.libclang
    pkg-config
    rustc
    cargo

    # Required for linking LLVM
    libffi
    zlib
    ncurses
    libxml2

    # HTTP / networking support
    curl.dev
    curl
    gcc
    binutils
  ];

  LLVM_SYS_211_PREFIX = "${pkgs.llvmPackages_21.llvm.dev}";
  LLVM_CONFIG_PATH = "${pkgs.llvmPackages_21.llvm.dev}/bin/llvm-config";
  shellHook = ''
    echo "Atomic Language Development Environment"
    echo "LLVM version: $(llvm-config --version)"
    echo "Rust version: $(rustc --version)"
  '';
}
