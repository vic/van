{ inputs, lib, ... }:
{
  imports = [
    inputs.flake-file.flakeModules.dendritic
  ];

  flake-file.outputs = lib.mkForce ''
    inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } (inputs.import-tree ./nix)
  '';

  perSystem.treefmt = {
    programs.rustfmt.enable = true;
    settings.on-unmatched = "warn";
  };
}
