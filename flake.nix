{
  description = "Development flake for ruscv (provides Rust toolchain and QEMU)";

  inputs = {
    pins.url = "git+ssh://git@git.my.home/phantato/nix-pins?rev=1be378ae3e32f0b28ddb61f7602e3ced6e1553c9";
    nixpkgs.follows = "pins/nixpkgs";
    flake-utils.follows = "pins/flake-utils";
    rust-overlay.follows = "pins/rust-overlay";

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
      pins,
      flake-utils,
      rust-overlay,
      crane,
      rustsbi-src,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = pins.lib.pkgsFor system;
        craneLib = crane.mkLib pkgs;

        # Rust toolchain for the host (used to build the kernel and user apps)
        rust = pkgs.rust-bin.nightly."2026-10-28".default.override {
          targets = [ "riscv64gc-unknown-none-elf" ];
          extensions = [
            "llvm-tools-preview"
            "rust-src"
          ];
        };

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
            tmux
            netcat-openbsd

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
            # Value must be `true`/`false` or cargo errors out.
            export CARGO_NET_GIT_FETCH_WITH_CLI=true
          '';
        };
      }
    );
}
