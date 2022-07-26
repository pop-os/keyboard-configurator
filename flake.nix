{
  description = "System76 Keyboard Configurator";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-22.05";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = naersk.lib."${system}";
        cargo = pkgs.cargo;
      in {
        defaultPackage = naersk-lib.buildPackage {
          name = "system76-keyboard-configurator";
          version = "1.2.0";
          src = ./.;
          buildInputs =
            (with pkgs; [ pkg-config rustc cargo hidapi glib gtk3 ]);
        };
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [ pkg-config rustc cargo hidapi glib gtk3 ];
        };
        formatter = nixpkgs.legacyPackages."${system}".nixfmt;
      });
}
