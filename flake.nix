{
  description = "Rust example flake for Zero to Nix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";
    flake-utils.url  = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      # inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, crane, nixpkgs, flake-utils, rust-overlay }:
    let
      genericVersion = self.lastModifiedDate + "-" + (self.shortRev or self.dirtyShortRev);
      semanticReleaseVersion = builtins.getEnv "SEMANTIC_RELEASE_VERSION";
      version = if semanticReleaseVersion != "" then semanticReleaseVersion else genericVersion;
    in flake-utils.lib.eachDefaultSystem (system:
      let
        # FIXME probably `armv7-unknown-linux-gnueabihf` is more accurate
        aarch64Target = "aarch64-unknown-linux-musl";
        desktopTarget = "x86_64-unknown-linux-musl";
        kindleTarget = "arm-unknown-linux-musleabi";
        kindlehfTarget = "arm-unknown-linux-musleabihf";

        pkgs = import nixpkgs {
          inherit system;

          config = {
            permittedInsecurePackages = [ "openssl-1.1.1w" ];
          };
        };

        buildBackendRustPackage = {
          packageName,
          copyTarget ? false
        }: target:
          let
            pkgs = import nixpkgs {
              inherit system;
              config.allowUnsupportedSystem = true;
              overlays = [ (import rust-overlay) ];
            };

            pkgsCross = import nixpkgs {
              inherit system;
              config.allowUnsupportedSystem = true;
              crossSystem.config = target;
              };

            targetCLibs = [
              pkgsCross.freetype
              pkgsCross.fontconfig
              pkgsCross.expat
              pkgsCross.zlib
              pkgsCross.libpng
              pkgsCross.bzip2
              pkgsCross.brotli
            ];

            craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable."1.95.0".default.override {
              targets = [target];
            });
          in
            with pkgs; craneLib.buildPackage rec {
              doInstallCargoArtifacts = copyTarget;
              installCargoArtifactsMode = "use-symlink";

              doCheck = false;

              src = ./backend;
              cargoExtraArgs = "--package=${packageName}";

              nativeBuildInputs = [
                stdenv.cc
              ];
              
              TARGET_CC = with pkgsCross.stdenv; "${cc}/bin/${cc.targetPrefix}cc";
              CARGO_BUILD_TARGET = target;
              # https://github.com/rust-lang/cargo/issues/4133
              CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static -C linker=${TARGET_CC}";
              RUST_FONTCONFIG_DLOPEN = "on";
              FONTCONFIG_NO_PKG_CONFIG = "1";
              CARGO_BUILD_ENV = ''FREETYPE_NO_PKG_CONFIG=1
 FONTCONFIG_NO_PKG_CONFIG=1
 RUST_FONTCONFIG_DLOPEN=on
 RUST_FREETYPE_DLOPEN=on
 FREETYPE_DLOPEN=1
'';

            };

          mkCbzMetadataReaderPackage = buildBackendRustPackage { packageName = "cbz_metadata_reader"; };
          mkServerPackage = buildBackendRustPackage { packageName = "server"; };
          mkUdsHttpRequestPackage = buildBackendRustPackage { packageName = "uds_http_request"; };
          mkSharedPackage = buildBackendRustPackage { packageName = "shared"; copyTarget = true; };

          mkBuildInfoFile = packageName: pkgs.writers.writeJSON "BUILD_INFO.json" {
            version = version;
            build = packageName;
          };

          pluginFolderWithoutServer = with pkgs; stdenv.mkDerivation {
            name = "rakuyomi-plugin-without-server";
            # Filter out unittests (*_spec.lua) files.
            src = lib.fileset.toSource {
              root = ./frontend;
              fileset = (lib.fileset.fileFilter (file: !(lib.strings.hasSuffix "_spec.lua" file.name)) ./frontend);
            };
            phases = [ "unpackPhase" "installPhase" ];
            installPhase = ''
              mkdir $out
              cp -r $src/rakuyomi.koplugin/* $out/
            '';
          };

          mkPluginFolderWithServer = { buildName, target }:
            let
              cbzMetadataReader = mkCbzMetadataReaderPackage target;
              server = mkServerPackage target;
              udsHttpRequest = mkUdsHttpRequestPackage target;
              buildInfoFile = mkBuildInfoFile buildName;
            in
              with pkgs; stdenv.mkDerivation {
                name = "rakuyomi-plugin";
                phases = [ "installPhase" ];
                installPhase = ''
                  mkdir $out
                  cp -r ${pluginFolderWithoutServer}/* $out/
                  cp ${cbzMetadataReader}/bin/cbz_metadata_reader $out/cbz_metadata_reader
                  cp ${server}/bin/server $out/server
                  cp ${udsHttpRequest}/bin/uds_http_request $out/uds_http_request
                  cp ${buildInfoFile} $out/BUILD_INFO.json
                '';
              };

          koreader = pkgs.callPackage ./packages/koreader.nix {};
          
          koreaderWithRakuyomiFrontend = pkgs.callPackage ./packages/koreader.nix {
            plugins = [ pluginFolderWithoutServer ];
          };
          # dev / debug: real binaries (not bash shellHook functions) so they
          # work in any shell — fish, bash, zsh, nushell — under
          # `use flake` / direnv. The shellHook `dev()` / `debug()` bash
          # functions are kept for back-compat with bash-only users; bash
          # prefers the function, fish has only the binary.
          rakuyomiDev = pkgs.writeShellScriptBin "dev" ''
            set -e
            cd "$(git rev-parse --show-toplevel)"
            exec bash ${./tools/run-koreader-with-plugin.sh}
          '';
          rakuyomiDebug = pkgs.writeShellScriptBin "debug" ''
            set -e
            cd "$(git rev-parse --show-toplevel)"
            exec bash ${./tools/run-koreader-with-plugin.sh} --debug
          '';
          # All other devShell helpers (cargo-test, check-format,
          # check-lint, fix-rust-format, fix-rust-lint, docs,
          # prepare-sql-queries, remote-install, remote-ssh,
          # test-frontend, test-e2e, dev-linux, dev-macos): same
          # writeShellScriptBin pattern as dev/debug above. The
          # shellHook bash functions are kept for back-compat with
          # `nix develop` interactive bash.
          rakuyomiCargoTest = pkgs.writeShellScriptBin "cargo-test" ''
            set -e
            REPO_ROOT="$(git rev-parse --show-toplevel)"
            (cd "$REPO_ROOT/backend" && cargo test --all)
            exec bash ${./tools/run-koreader-with-plugin.sh}
          '';
          rakuyomiCheckFormat = pkgs.writeShellScriptBin "check-format" ''
            set -e
            cd "$(git rev-parse --show-toplevel)/backend"
            exec cargo fmt --check
          '';
          rakuyomiCheckLint = pkgs.writeShellScriptBin "check-lint" ''
            REPO_ROOT="$(git rev-parse --show-toplevel)"
            cd "$REPO_ROOT/backend" && cargo clippy -- -D warnings
            cd "$REPO_ROOT" && python3 ci/lua-language-server-check.py frontend/
          '';
          rakuyomiFixRustFormat = pkgs.writeShellScriptBin "fix-rust-format" ''
            set -e
            cd "$(git rev-parse --show-toplevel)/backend"
            exec cargo fmt --all
          '';
          rakuyomiFixRustLint = pkgs.writeShellScriptBin "fix-rust-lint" ''
            set -e
            cd "$(git rev-parse --show-toplevel)/backend"
            exec cargo clippy --fix --allow-dirty -- -D warnings
          '';
          rakuyomiDocs = pkgs.writeShellScriptBin "docs" ''
            set -e
            cd "$(git rev-parse --show-toplevel)/docs"
            exec mdbook serve --open
          '';
          rakuyomiPrepareSqlQueries = pkgs.writeShellScriptBin "prepare-sql-queries" ''
            set -e
            cd "$(git rev-parse --show-toplevel)"
            exec bash ${./tools/prepare-sqlx-queries.sh}
          '';
          rakuyomiRemoteInstall = pkgs.writeShellScriptBin "remote-install" ''
            set -e
            cd "$(git rev-parse --show-toplevel)"
            exec python3 tools/install-into-remote-koreader.py "$@"
          '';
          rakuyomiRemoteSsh = pkgs.writeShellScriptBin "remote-ssh" ''
            exec sshpass -p "" ssh -p "$REMOTE_KOREADER_SSH_PORT" -o StrictHostKeyChecking=no "root@$REMOTE_KOREADER_HOST" "$@"
          '';
          rakuyomiTestFrontend = pkgs.writeShellScriptBin "test-frontend" ''
            set -e
            cd "$(git rev-parse --show-toplevel)"
            exec busted -C frontend/rakuyomi.koplugin .
          '';
          rakuyomiTestE2e = pkgs.writeShellScriptBin "test-e2e" ''
            set -e
            REPO_ROOT="$(git rev-parse --show-toplevel)"
            cd "$REPO_ROOT/e2e-tests"
            poetry env use "$(which python)"
            poetry install --no-root
            exec poetry run pytest "$@"
          '';
          # dev-linux / dev-macos: their .sh scripts compute REPO_ROOT
          # from BASH_SOURCE[0], which resolves to the /nix/store path
          # once writeShellScriptBin copies them. The wrapper passes the
          # real repo root via the RAKUYOMI_REPO_ROOT env var, and the
          # scripts honor the override (with BASH_SOURCE[0] as fallback
          # so direct invocation from the repo still works).
          rakuyomiDevLinux = pkgs.writeShellScriptBin "dev-linux" ''
            set -e
            cd "$(git rev-parse --show-toplevel)"
            exec env RAKUYOMI_REPO_ROOT="$(pwd)" bash ${./tools/dev-linux.sh} "$@"
          '';
          rakuyomiDevMacos = pkgs.writeShellScriptBin "dev-macos" ''
            set -e
            cd "$(git rev-parse --show-toplevel)"
            exec env RAKUYOMI_REPO_ROOT="$(pwd)" bash ${./tools/dev-macos.sh} "$@"
          '';

          pkgsDev = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
            config.permittedInsecurePackages = [ "openssl-1.1.1w" ];
          };

          koreaderFrontendPath = if pkgs.stdenv.isDarwin
            then "${koreader}/Applications/KOReader.app/Contents/koreader/frontend"
            else "${koreader}/lib/koreader/frontend";
          
          # FIXME this is really bad and relies on `mkCliPackage` copying the _entire_
          # target folder to the nix store (which is really bad too)
          mkSchemaFile = target:
            let
              shared = mkSharedPackage target;
            in
              with pkgs; stdenv.mkDerivation {
                name = "rakuyomi-settings-schema";
                phases = [ "installPhase" ];
                installPhase = ''
                  cp ${shared}/target/${target}/release/settings.schema.json $out
                '';
              };

          cargoDebugger = 
            let
              pkgs = import nixpkgs {
                inherit system;
                overlays = [ (import rust-overlay) ];
              };
              craneLib = (crane.mkLib pkgs);
            in craneLib.buildPackage {
              pname = "cargo-debugger";
              version = "0.1.0";
              src = pkgs.fetchFromGitHub {
                owner = "jkelleyrtp";
                repo = "cargo-debugger";
                rev = "master";
                sha256 = "sha256-5LJvGy6jZLsN3IhgWktLKvH8seAvee0cAH4Rs+1Wmuk="; # You'll need to replace this with the actual hash
              };
            };
      in
      let
        buildTargets = {
          aarch64 = aarch64Target;
          desktop = desktopTarget;
          kindle = kindleTarget;
          kindlehf = kindlehfTarget;
        };

        builds = (builtins.mapAttrs 
          (name: target: mkPluginFolderWithServer { buildName = name; target = target; })
          buildTargets
        );

      in {
        packages.koreader = koreader;
        packages.rakuyomi = builds // {
          koreader-with-plugin = koreaderWithRakuyomiFrontend;
          shared = mkSharedPackage desktopTarget;
          settings-schema = mkSchemaFile desktopTarget;
        };
        packages.cargo-debugger = cargoDebugger;

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            (pkgsDev.rust-bin.stable."1.95.0".default.override {
              extensions = [ "rustfmt" "clippy" "rust-src" ];
            })
            clang
            gcc
            git
            pkg-config
            lua-language-server
            luajitPackages.busted
            mdbook
            mdbook-admonish
            python313
            python313Packages.tkinter
            poetry
            sqlx-cli
            sshpass
            cargo-flamegraph
            freetype
            fontconfig
            koreader
            cargoDebugger
            rakuyomiDev
            rakuyomiDebug
            rakuyomiCargoTest
            rakuyomiCheckFormat
            rakuyomiCheckLint
            rakuyomiFixRustFormat
            rakuyomiFixRustLint
            rakuyomiDocs
            rakuyomiPrepareSqlQueries
            rakuyomiRemoteInstall
            rakuyomiRemoteSsh
            rakuyomiTestFrontend
            rakuyomiTestE2e
            rakuyomiDevLinux
            rakuyomiDevMacos
          ] ++ lib.optionals pkgs.stdenv.isLinux [
            mold-wrapped
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            libiconv
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          shellHook = ''
            mkdir -p .cargo
          '' + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            cat > .cargo/config.toml << CONFIGEOF
        [target.x86_64-unknown-linux-gnu]
        linker = "${pkgs.clang}/bin/clang"
        rustflags = ["-C", "link-arg=--ld-path=${pkgs.mold-wrapped}/bin/mold"]
        CONFIGEOF
          '' + ''
            cat > .luarc.json << 'LUACEOF'
        {
          "$schema": "https://raw.githubusercontent.com/sumneko/vscode-lua/master/setting/schema.json",
          "diagnostics.globals": ["G_reader_settings"],
          "workspace.library": [
            "''${3rd}/luassert/library",
            "''${3rd}/busted/library",
            "${koreaderFrontendPath}"
          ],
          "runtime.version": "LuaJIT",
          "diagnostics.neededFileStatus": {
            "codestyle-check": "Any"
          }
        }
        LUACEOF
            cp .luarc.json frontend/.luarc.json

            cargo fetch --manifest-path="$PWD/backend/Cargo.toml"

            check-format() { cd "$PWD/backend" && cargo fmt --check; }
            check-lint() {
              cd "$PWD/backend" && cargo clippy -- -D warnings
              cd "$PWD" && python3 ci/lua-language-server-check.py frontend/
            }
            fix-rust-format() { cd "$PWD/backend" && cargo fmt --all; }
            fix-rust-lint() { cd "$PWD/backend" && cargo clippy --fix --allow-dirty -- -D warnings; }
            dev() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
            debug() { cd "$PWD" && . tools/run-koreader-with-plugin.sh --debug; }
            dev-linux() { cd "$PWD" && bash tools/dev-linux.sh "$@"; }
            cargo-test() {
              (cd "$PWD/backend" && cargo test --all) && . tools/run-koreader-with-plugin.sh
            }

            docs() { cd "$PWD/docs" && exec mdbook serve --open; }
            prepare-sql-queries() { cd "$PWD" && . tools/prepare-sqlx-queries.sh; }
            remote-install() { cd "$PWD" && python3 tools/install-into-remote-koreader.py; }
            remote-ssh() { sshpass -p "" ssh -p "$REMOTE_KOREADER_SSH_PORT" -o StrictHostKeyChecking=no "root@$REMOTE_KOREADER_HOST" "$@"; }
            test-frontend() { cd "$PWD" && busted -C frontend/rakuyomi.koplugin .; }
            test-e2e() {
              cd "$PWD/e2e-tests" && \
              poetry env use "$(which python)" && \
              poetry install --no-root && \
              poetry run pytest "$@"
            }

            echo "RakuYomi devShell activated."
          '';
        };
      } // {
        # `nix run .#` / `nix run .#start` — builds server locally and launches KOReader with rakuyomi
        apps.default = {
          type = "app";
          program = "${rakuyomiDev}/bin/dev";
        };
        apps.start = {
          type = "app";
          program = "${rakuyomiDev}/bin/dev";
        };
      }
    );
}
