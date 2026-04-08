{
  description = "Rust project template with development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustup
              rust-analyzer
              cargo-nextest
              cargo-llvm-cov
              cargo-watch
              k9s
              go-task
              skaffold
              kubectl
              kubernetes-helm
              kind
              docker
              git
            ];

            shellHook = ''
              export CARGO_HOME="$HOME/.cargo"
              export PATH="$CARGO_HOME/bin:$PATH"
            '';
          };
        }
      );
    };
}
