#!/bin/sh

VERSION=$1

if [ -z $VERSION ]; then
	echo "USAGE: $(basename $0) <version>"
	exit
fi

# Ensure we have the repos
if [ ! -d homebrew-blightmud ]; then
	git clone git@github.com:Blightmud/homebrew-blightmud homebrew-blightmud
fi
if [ ! -d blightmud-pkg ]; then
	git clone ssh://aur@aur.archlinux.org/blightmud.git blightmud-pkg
fi

echo -n "Downloading Mac release... "
curl -L https://github.com/Blightmud/Blightmud/releases/download/v${VERSION}/blightmud-v${VERSION}-macos.zip --output blightmud-mac.zip > /dev/null 2>&1
echo "Done"
echo -n "Getting sha256 sum... "
SHA256=$(sha256sum blightmud-mac.zip | cut --delimiter=" " -f1)
echo "Done"
echo -n "Deleting file... "
rm blightmud-mac.zip
echo "Done"

cd homebrew-blightmud
echo -n "Updating homebrew formula... "
sed -i -e"s/  version \".*\"/  version \"${VERSION}\"/" Formula/blightmud.rb
sed -i -e"s/  sha256 \".*\"/  sha256 \"${SHA256}\"/" Formula/blightmud.rb
echo "Done"
echo -n "Pushing changes... "
git commit -a -m"Version ${VERSION}" > /dev/null 2>&1
git push > /dev/null 2>&1
echo "Done"
cd - > /dev/null

cd blightmud-pkg
echo -n "Updating PKGBUILD... "
sed -i -e"s/pkgver=.*/pkgver=${VERSION}/" PKGBUILD
updpkgsums > /dev/null 2>&1
makepkg --printsrcinfo > .SRCINFO
echo "Done"
echo -n "Pushing changes... "
git commit -a -m"Version ${VERSION}" > /dev/null 2>&1
git push > /dev/null 2>&1
echo "Done"
cd - > /dev/null
