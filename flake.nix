{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, flake-utils, ... }: flake-utils.lib.eachDefaultSystem (system:
        let pkgs = import nixpkgs { inherit system; }; in {
            devShells.default = pkgs.mkShell {
                shellHook = "export NIX_SHELL_NAME='fabricate'";
                buildInputs = with pkgs; [ go sphinx python312Packages.sphinx-rtd-theme ];
                nativeBuildInputs = with pkgs; [ ninja ];
            };

            defaultPackage = pkgs.buildGoModule {
                name = "fabricate";
                src = self;

                vendorHash = "sha256-xY/VVT/KFEbipLTPXUr5EM+uCO1E9KHvLy74LtFusWM=";

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
