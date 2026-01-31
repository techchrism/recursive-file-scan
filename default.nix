{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage (finalAttrs: {
  pname = "recursive-file-scan";
  version = "0.1.0";

  src = ./.;

  cargoHash = "sha256-gFmdzikemXnHWl6U0KCdQo2PMIcbxgnRs4VLgorQeo0=";

  meta = {
    description = "A quickly-whipped-together utility to write a directory tree to a sqlite database";
    homepage = "https://github.com/techchrism/recursive-file-scan";
    license = pkgs.lib.licenses.mit;
    maintainers = [ ];
  };
})