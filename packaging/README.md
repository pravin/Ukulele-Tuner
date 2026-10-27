# Packaging

| Where                | Files                     | Updated by                                     |
| -------------------- | ------------------------- | ---------------------------------------------- |
| GitHub Releases      | `dist-workspace.toml`     | The release workflow, automatically            |
| Homebrew tap         | `dist-workspace.toml`     | The release workflow, automatically            |
| crates.io            | `.github/workflows/publish-crates.yml` | The release workflow, automatically |
| AUR                  | `aur/`                    | By hand. See [aur/README.md](aur/README.md)    |
| nixpkgs              | `nix/package.nix`         | By hand, then nixpkgs bots keep it up to date  |
| Alpine (aports)      | `alpine/APKBUILD`         | By hand                                        |

## One-time setup for automatic publishing

The release workflow only publishes a GitHub release if every publish job succeeds, so set these up
before pushing the first tag:

1. **Homebrew tap:** create an empty public repo at `pravin/homebrew-tap`. Then create a fine-grained
   personal access token with *Contents: read and write* on that repo, and add it to this repo's
   Actions secrets as `HOMEBREW_TAP_TOKEN`.
2. **crates.io:** create an API token at https://crates.io/settings/tokens with the *publish-new* and
   *publish-update* scopes, and add it to this repo's Actions secrets as `CARGO_REGISTRY_TOKEN`.

Users then install with:

```sh
brew install pravin/tap/uketuner
cargo install uketuner
```

## Making a release

1. Bump `version` in `Cargo.toml`, then run `cargo build` to update `Cargo.lock`.
2. Commit, tag and push:

   ```sh
   git tag v0.2.0
   git push && git push --tags
   ```

3. Once the release workflow finishes, update the AUR packages (see `aur/README.md`).

## nixpkgs

1. Fill in the source hash. `cargoHash` is already set and only changes when `Cargo.lock` does. After
   the tag exists, build the package against a nixpkgs checkout and copy the `got:` hash from the error
   into `hash`:

   ```sh
   nix-build -E 'with import <nixpkgs> { }; callPackage ./package.nix { }'
   ```

2. Add yourself to `maintainers/maintainer-list.nix` in nixpkgs (one commit), then set
   `maintainers = with lib.maintainers; [ <your-handle> ];` in `package.nix`.
3. Copy `package.nix` to `pkgs/by-name/uk/uketuner/package.nix` and open a PR titled
   `uketuner: init at 0.2.0`. After it's merged, the `r-ryantm` bot opens update PRs for new tags.

## Alpine

New packages go into `testing/` in [aports](https://gitlab.alpinelinux.org/alpine/aports):

1. Fork aports on GitLab and copy `APKBUILD` to `testing/uketuner/APKBUILD`.
2. In an Alpine environment with `alpine-sdk`, run `abuild checksum && abuild -r` to fill in the
   checksums and test the build.
3. Commit as `testing/uketuner: new aport` and open a merge request.
