{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  # https://devenv.sh/packages/
  packages = [pkgs.git];

  languages.rust = {
    enable = true;
    # https://devenv.sh/reference/options/#languagesrustchannel
    channel = "nightly";

    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer"];
  };

  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
  };
}
