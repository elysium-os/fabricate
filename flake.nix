{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs =
        {
            self,
            nixpkgs,
            flake-utils,
            ...
        }:
        flake-utils.lib.eachDefaultSystem (
            system:
            let
                pkgs = import nixpkgs { inherit system; };
                nativeBuildInputs = with pkgs; [
                    pkgconf
                    mdbook
                ];
                buildInputs = with pkgs; [
                    openssl
                    ninja
                ];
            in
            {
                devShells.default = pkgs.mkShell {
                    shellHook = "export NIX_SHELL_NAME='fabricate'";
                    nativeBuildInputs = nativeBuildInputs ++ [ pkgs.rustup ];
                    inherit buildInputs;
                };

                defaultPackage = pkgs.rustPlatform.buildRustPackage {
                    name = "fabricate";
                    src = self;

                    cargoLock.lockFile = ./Cargo.lock;

                    inherit nativeBuildInputs;
                    inherit buildInputs;

                    meta = {
                        description = "Simple yet powerful meta buildsystem.";
                        homepage = "https://github.com/elysium-os/fabricate";
                        license = pkgs.lib.licenses.bsd3;
                        maintainers = with pkgs.lib.maintainers; [ wux ];
                    };
                };
            }
        );
}
