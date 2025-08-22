{ inputs, ... }:
{
  flake-file.inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    devenv.url = "github:cachix/devenv";
  };

  flake-file.nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  imports = [
    inputs.devenv.flakeModule
  ];

  perSystem =
    { config, pkgs, ... }:
    let
      app = config.devshells.default.languages.rust.import ./crates { };
      wrapped = pkgs.stdenvNoCC.mkDerivation {
        name = "van-wrapped";
        nativeBuildInputs = [pkgs.makeWrapper];
        phases = ["wrap"];
        wrap = ''
          wrapProgram ${app}/bin/van $out/bin/van --prefix PATH : ${pkgs.lib.makeBinPath [pkgs.carapace]}
        '';
      };
    in
    {
      packages.default = wrapped;
      devenv.shells.default = {
        languages.rust.enable = true;
        packages = [
          pkgs.carapace
        ];
      };
    };

}
