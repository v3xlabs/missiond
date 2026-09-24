{
  fetchPnpmDeps,
  just,
  lib,
  nodejs,
  pkg-config,
  pnpm,
  pnpmConfigHook,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "missiond";
  version = "0.0.1";

  # Build outputs from a local development shell must not reach the derivation. pnpm refuses to
  # purge a node_modules it did not create, and the build fails with no TTY to confirm on.
  src = lib.cleanSourceWith {
    src = ../.;
    filter = path: type: let
      name = baseNameOf path;
    in
      lib.cleanSourceFilter path type
      && !(type == "directory" && builtins.elem name ["node_modules" "dist" "target" "web-dist" ".direnv"])
      && !(lib.hasPrefix "result" name);
  };

  cargoRoot = "daemon";
  cargoLock.lockFile = ../daemon/Cargo.lock;

  pnpmRoot = "web";
  pnpmDeps = fetchPnpmDeps {
    pname = "missiond-web";
    version = "0.0.1";
    src = ../web;
    fetcherVersion = 4;
    hash = "sha256-RzYZMUJJOFE30YxtM9KUAi2PR2y2bFqEn9K87aW5LhU=";
  };

  nativeBuildInputs = [
    just
    nodejs
    pkg-config
    pnpm
    pnpmConfigHook
  ];

  buildPhase = ''
    runHook preBuild
    just build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    install -Dm755 daemon/target/release/missiond $out/bin/missiond
    runHook postInstall
  '';

  meta = {
    description = "Mission Control display daemon";
    homepage = "https://github.com/v3xlabs/missiond";
    mainProgram = "missiond";
    platforms = lib.platforms.linux;
  };
}
