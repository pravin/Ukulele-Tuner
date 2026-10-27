# AUR packages

- `uketuner` builds from the tagged source.
- `uketuner-bin` repackages the Linux binaries from the GitHub release.

## Publishing a new version

Do this after the release workflow has published the GitHub release, on an Arch machine (or in an
`archlinux` container) with `base-devel` and `pacman-contrib` installed.

1. Set `pkgver` in both PKGBUILDs to the new version and reset `pkgrel` to 1.
2. In each package directory, fill in the checksums, test the build and regenerate `.SRCINFO`:

   ```sh
   updpkgsums
   makepkg -sci
   makepkg --printsrcinfo > .SRCINFO
   ```

3. Commit both files and push them to the package's AUR repository:

   ```sh
   git clone ssh://aur@aur.archlinux.org/uketuner.git      # first time only
   cp PKGBUILD .SRCINFO <clone>/ && cd <clone>
   git commit -am "Update to <version>" && git push
   ```

The first push to an AUR repository creates the package. You need an
[AUR account](https://aur.archlinux.org/register) with your SSH public key added to it.
