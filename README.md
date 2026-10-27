# Ukulele Tuner

A small, fast ukulele tuner for your terminal. It listens through your microphone and shows how far each
string is from its target note, then walks you through the strings one by one.

```
Tuning string 4: G4 (392.0 Hz)
[.........................|..........#..............]  +21.3 cents  loosen
```

- Supports standard, low-G, D, low-A and baritone tunings
- Shows how far off you are in cents, with a colour-coded meter
- Lets you change the reference pitch (for example A4 = 442 Hz)
- Runs on Linux, macOS and Windows as a single binary with no runtime dependencies

## Installation

### Prebuilt binaries

**macOS and Linux:**

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/pravin/Ukulele-Tuner/releases/latest/download/uketuner-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pravin/Ukulele-Tuner/releases/latest/download/uketuner-installer.ps1 | iex"
```

You can also download an archive for your platform from the
[releases page](https://github.com/pravin/Ukulele-Tuner/releases).

### Package managers

```sh
brew install pravin/tap/uketuner     # Homebrew (macOS and Linux)
yay -S uketuner                      # Arch Linux (AUR), or uketuner-bin for the prebuilt binary
cargo install uketuner               # Any platform with Rust
```

### From source

Install [Rust](https://rustup.rs/), then:

```sh
git clone https://github.com/pravin/Ukulele-Tuner.git
cd Ukulele-Tuner
cargo install --path .
```

On Linux, building needs the ALSA headers and `pkg-config`:

```sh
sudo apt install libasound2-dev pkg-config      # Debian / Ubuntu
sudo dnf install alsa-lib-devel pkgconf         # Fedora
sudo pacman -S alsa-lib pkgconf                 # Arch
```

ALSA also covers PulseAudio and PipeWire systems through their ALSA plugins.

## Usage

Run `uketuner` and pluck the string it asks for:

1. The first line names the string to tune. String 4 is the one nearest your chin, and string 1 is nearest
   the floor.
2. The `#` shows how far out of tune the string is, in cents (hundredths of a semitone).
   - If the `#` is **left** of the `|`, the string is flat: **tighten** it.
   - If the `#` is **right** of the `|`, the string is sharp: **loosen** it.
3. When the `#` lines up with the `|` and the meter turns green, press `Enter` to move to the next string.

The meter is red when you're more than 20 cents out, yellow when you're within 20 cents, and green when
you're within 5 cents.

### Keys

| Key                    | Action                 |
| ---------------------- | ---------------------- |
| `Enter`, `Space`, `n`  | Next string            |
| `b`                    | Previous string        |
| `q`, `Esc`, `Ctrl-C`   | Quit                   |

### Tunings

Choose one with `--tuning`. Strings are listed from 4 to 1.

| Tuning               | Notes             |
| -------------------- | ----------------- |
| `standard` (default) | G4 C4 E4 A4       |
| `low-g`              | G3 C4 E4 A4       |
| `d`                  | A4 D4 F#4 B4      |
| `low-a`              | A3 D4 F#4 B4      |
| `baritone`           | D3 G3 B3 E4       |

### Options

| Option              | Description                                                       |
| ------------------- | ----------------------------------------------------------------- |
| `-t`, `--tuning`    | Tuning to use (see above)                                         |
| `--a4 <HZ>`         | Reference pitch for A4. Default: 440                              |
| `--threshold <N>`   | Input level below which sound is ignored. Default: 0.005          |
| `--list-devices`    | List microphones. The default one is marked with `*`              |
| `-d`, `--device`    | Microphone to use, by number or part of its name                  |

For example:

```sh
uketuner --tuning low-g --a4 442
uketuner --list-devices
uketuner --device "MacBook Pro Microphone"
```

## Troubleshooting

- **The meter never moves.** You may be using the wrong microphone. Run `uketuner --list-devices`, then pick
  the right one with `--device`. On macOS, check that your terminal app has microphone access under
  System Settings → Privacy & Security → Microphone.
- **The meter twitches when nobody is playing.** Background noise is getting through. Raise the threshold,
  for example `--threshold 0.02`.
- **The meter is pinned to one end.** The string is more than 50 cents out. Keep turning the peg in the
  direction shown until the `#` comes back into range.

## How it works

The tuner reads about 90 ms of audio at a time, applies a Blackman window and takes a zero-padded FFT. It looks
for the strongest peak within half an octave of the target note, which stops harmonics being mistaken for the
note itself. Quadratic interpolation around that peak gives an accuracy of well under a cent. The reading
shown is the median of the last few, so the meter doesn't jump around.

## Development

```sh
cargo test
cargo run -- --tuning low-g
```

Releases are built by [cargo-dist](https://opensource.axo.dev/cargo-dist/). To make one, bump the version in
`Cargo.toml`, then tag and push it:

```sh
git tag v0.2.0
git push --tags
```

GitHub Actions then builds the binaries and installers, publishes a GitHub release, updates the Homebrew
tap and publishes to crates.io. See [packaging/README.md](packaging/README.md) for the one-time setup and
for the AUR, Nix and Alpine packages.

## Contributing

Bug reports and pull requests are welcome. If you'd like another tuning, or support for another instrument,
open an issue or send a PR.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

— Prav
