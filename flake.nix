{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { nixpkgs, flake-utils, ... }: flake-utils.lib.eachDefaultSystem (system:
        let pkgs = import nixpkgs { inherit system; }; in {
            devShells.default = pkgs.mkShell {
                shellHook = "export NIX_SHELL_NAME='fab'";
                buildInputs = with pkgs; [ pkgconf openssl ];
                nativeBuildInputs = with pkgs; [ ninja ];
            };

            defaultPackage = pkgs.rustPlatform.buildRustPackage {
                name = "fabricate";
                src = self;

                cargoLock.lockFile = ./Cargo.lock;

                meta = {
                    description = " Simple yet powerful meta buildsystem.";
                    homepage = "https://github.com/elysium-os/fabricate";
                    license = pkgs.lib.licenses.bsd3;
                    maintainers = with pkgs.lib.maintainers; [ wux ];
                };
            };
        }
    );
}
