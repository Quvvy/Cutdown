#!/usr/bin/env bash
# Build an Arch Linux package (cutdown-<ver>-<rel>-x86_64.pkg.tar.zst) from this checkout.
# Uses packaging/arch/PKGBUILD, but feeds it the committed HEAD instead of the GitHub tag,
# so it works before tagging (checksums are skipped: the tarball is local). Output lands in
# target/release/bundle/arch/.
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(sed -n 's/^pkgver=//p' packaging/arch/PKGBUILD)
app_version=$(node -p "require('./package.json').version")
if [[ "$version" != "$app_version" ]]; then
  echo "PKGBUILD pkgver ($version) doesn't match package.json ($app_version)." >&2
  exit 1
fi
if [[ -n "$(git status --porcelain)" ]]; then
  echo "Note: uncommitted changes are not included; the package is built from HEAD." >&2
fi

work=target/arch-build
out=target/release/bundle/arch
rm -rf "$work"
mkdir -p "$work" "$out"
cp packaging/arch/PKGBUILD "$work/"
git archive --format=tar.gz --prefix="Cutdown-$version/" -o "$work/cutdown-$version.tar.gz" HEAD

# The package is for other people's machines: build for baseline x86-64, not this CPU.
# (CachyOS and many tuned setups ship -march=native / target-cpu=native in makepkg.conf.)
# Passing --config also skips makepkg.conf.d/ and ~/.makepkg.conf.
cp /etc/makepkg.conf "$work/makepkg.conf"
cat >> "$work/makepkg.conf" <<'EOF'

# --- Cutdown release build overrides ---
CFLAGS="-march=x86-64 -mtune=generic -O2 -pipe -fno-plt -fexceptions -Wp,-D_FORTIFY_SOURCE=3 -Wformat -Werror=format-security -fstack-clash-protection -fcf-protection"
CXXFLAGS="$CFLAGS -Wp,-D_GLIBCXX_ASSERTIONS"
RUSTFLAGS="-C target-cpu=x86-64"
EOF

(
  cd "$work"
  PKGDEST="$PWD/../../$out" SRCDEST="$PWD" makepkg --config "$PWD/makepkg.conf" --skipchecksums --force --cleanbuild --noconfirm "$@"
)

ls -l "$out"/cutdown-"$version"-*.pkg.tar.zst
echo "Install with: sudo pacman -U $out/cutdown-$version-*.pkg.tar.zst"
