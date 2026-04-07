{
  description = "Development flake for ruscv (provides Rust toolchain and QEMU)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    crane.url = "github:ipetkov/crane";

    rustsbi-src = {
      url = "github:rustsbi/rustsbi";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      rustsbi-src,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        craneLib = crane.mkLib pkgs;

        # Rust toolchain for the host (used to build the kernel and user apps)
        rust = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            targets = [ "riscv64gc-unknown-none-elf" ];
            extensions = [
              "llvm-tools-preview"
              "rust-src"
            ];
          }
        );

        rustsbi-prototyper = craneLib.buildPackage {
          src = craneLib.cleanCargoSource rustsbi-src;
          strictDeps = true;
          cargoToml = "${rustsbi-src}/prototyper/prototyper/Cargo.toml";
          buildInputs = with pkgs; [
            cargo-binutils
            ubootTools
            git
          ];
          cargoVendorDir = null;
        };

      in
      {
        packages = {
          # inherit rustsbi-prototyper;
        };
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            gnumake
            qemu
            git

            rust
            rust-analyzer
            cargo-binutils

            llvmPackages.lldb
            lld
            binutils
            llvmPackages.llvm

          ];

          shellHook = ''
            echo "Entering dev shell for ruscv (target: riscv64gc-unknown-none-elf)"
            export RUSTFLAGS="-C link-arg=-Tkernel/src/kernel.ld"
            # Keep build artifacts under project `target/` directory
            export CARGO_TARGET_DIR="$PWD/target"

            # Make cargo use the git CLI for fetching git dependencies (supports proxy/ssh)
            export CARGO_NET_GIT_FETCH_WITH_CLI=1
          '';
        };
      }
    );
}
