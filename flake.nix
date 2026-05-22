{
  description = "GitHub Mock API devshell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs { inherit system; };
      llvm = pkgs.llvmPackages.llvm;
      libclang = pkgs.llvmPackages.libclang;
    in {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          cargo-llvm-cov
          clang
          libclang
          llvm
          ruby
          rustc
        ];

        shellHook = ''
          export LLVM_COV="${llvm}/bin/llvm-cov"
          export LLVM_PROFDATA="${llvm}/bin/llvm-profdata"
          export LIBCLANG_PATH="${libclang.lib}/lib"
        '';
      };
    });
}
