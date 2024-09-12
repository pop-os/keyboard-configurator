{
  description = "System76 Keyboard Configurator";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = naersk.lib."${system}";
      in
      {
        packages = rec {
          default = system76-keyboard-configurator;

          system76-keyboard-configurator = naersk-lib.buildPackage {
            name = "system76-keyboard-configurator";
            version = "1.3.12";
            src = ./.;
            buildInputs = with pkgs; [ pkg-config rustc cargo hidapi glib gtk3 ];
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ pkg-config rustc cargo hidapi glib gtk3 ];
        };

        formatter = nixpkgs.legacyPackages."${system}".nixfmt;
      });
}
