{
  lib,
  stdenv,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  alsa-lib,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "uketuner";
  version = "0.2.0";

  src = fetchFromGitHub {
    owner = "pravin";
    repo = "Ukulele-Tuner";
    tag = "v${finalAttrs.version}";
    hash = lib.fakeHash;
  };

  cargoHash = "sha256-AF4+NRBSzj96aMFhyi5H1QffA8IvwFWYkNGCaA5UAwU=";

  nativeBuildInputs = lib.optionals stdenv.hostPlatform.isLinux [ pkg-config ];
  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [ alsa-lib ];

  meta = {
    description = "Console utility to tune your ukulele";
    homepage = "https://github.com/pravin/Ukulele-Tuner";
    license = lib.licenses.gpl3Plus;
    maintainers = [ ];
    mainProgram = "uketuner";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
})
