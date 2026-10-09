{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    };

    outputs =
        { self, nixpkgs, ... }:
        let
            systems = [
                "x86_64-linux"
                "aarch64-linux"
                "x86_64-darwin"
                "aarch64-darwin"
            ];
            forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
        in
        {
            devShells = forEachSystem (pkgs: {
                default = pkgs.mkShell {
                    NIX_SHELL_NAME = "fabricate";

                    nativeBuildInputs = with pkgs; [
                        mdbook
                        rustup
                        pkgconf
                        openssl
                    ];
                };
            });

            packages = forEachSystem (pkgs: {
                default = pkgs.rustPlatform.buildRustPackage {
                    name = "fabricate";
                    src = self;

                    cargoLock.lockFile = ./Cargo.lock;

                    nativeBuildInputs = with pkgs; [
                        pkgconf
                    ];

                    buildInputs = with pkgs; [
                        openssl
                    ];

                    meta = {
                        description = "Simple yet powerful meta buildsystem.";
                        homepage = "https://github.com/elysium-os/fabricate";
                        license = pkgs.lib.licenses.bsd3;
                        maintainers = with pkgs.lib.maintainers; [ wux ];
                    };
                };
            });
        };
}
