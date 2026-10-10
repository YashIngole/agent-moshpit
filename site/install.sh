#!/bin/sh
# Agent Moshpit: install it, or bring it up to date, on macOS and Linux.
#
#   curl -fsSL https://agentmoshpit.com/install.sh | sh
#
# It fetches the newest release from github.com/YashIngole/agent-moshpit and puts it
# where applications go:
#   macOS    the app, into /Applications (or ~/Applications if that cannot be written)
#   Linux    the .deb with apt, the .rpm with dnf or zypper (these ask for your
#            password), or, anywhere else, the AppImage into ~/.local/bin
# Then it starts the app. Nothing else on the computer is changed.
#
# Nothing is unpacked or installed unchecked. The release's list of installer hashes
# must carry a signature from the key below (tools/install-signers in the repository),
# which ssh-keygen checks, and the installer must have the hash that list gives it.
set -eu

REPO="https://github.com/YashIngole/agent-moshpit/releases"
SIGNER="installers@agentmoshpit.com"
SIGNERS='installers@agentmoshpit.com namespaces="agentmoshpit-install" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIO+PcdRuFfWQEXF3m4TryI+MxJ0ZyAlrPf88PavyArDQ'

say() { printf '%s\n' "$*"; }
fail() {
  printf 'Agent Moshpit was not installed: %s\n' "$*" >&2
  exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || fail "this needs the program '$1', which is not here."; }
download() { curl -fL --proto '=https' --proto-redir '=https' "$@"; }

# Everything happens in main, called on the last line: a download cut short runs nothing.
main() {
need curl
command -v ssh-keygen >/dev/null 2>&1 || fail "checking the download's signature needs ssh-keygen, from OpenSSH. Install OpenSSH (openssh-client on Debian and Ubuntu), then run this again."
command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || fail "checking the download needs sha256sum or shasum, which are not here."
sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | awk '{ print $1 }'; }

tmp=$(mktemp -d 2>/dev/null || mktemp -d -t moshpit)
trap 'rm -rf "$tmp"' EXIT
trap 'exit 1' INT TERM

# A copy that is running has agents in it: it is the user who ends them, not this.
only_fetch=${MOSHPIT_INSTALL_ONLY_FETCH:-}
running() { pgrep -x agent-moshpit >/dev/null 2>&1; }
if [ -z "$only_fetch" ] && running; then
  fail "it is running. Quit it first (that ends its agents' programs), then run this again."
fi

# Which installer this computer takes.
case "$(uname -s)" in
Darwin)
  need tar
  case "$(uname -m)" in
  arm64) file="agent-moshpit_darwin_aarch64.app.tar.gz" ;;
  x86_64) file="agent-moshpit_darwin_x64.app.tar.gz" ;;
  *) fail "this kind of Mac ($(uname -m)) has no build." ;;
  esac
  ;;
Linux)
  [ "$(uname -m)" = "x86_64" ] || fail "so far there is a build for 64-bit Intel and AMD only, and this is $(uname -m)."
  if command -v apt-get >/dev/null 2>&1; then
    file="agent-moshpit_linux_amd64.deb"
  elif command -v dnf >/dev/null 2>&1 || command -v zypper >/dev/null 2>&1; then
    file="agent-moshpit_linux_x86_64.rpm"
  else
    file="agent-moshpit_linux_amd64.AppImage"
  fi
  ;;
*)
  fail "this script is for macOS and Linux. On Windows, in PowerShell: irm https://agentmoshpit.com/install.ps1 | iex"
  ;;
esac

# First the signed list of the newest release's installers, then the one installer, checked.
download -sS "$REPO/latest/download/SHA256SUMS" -o "$tmp/SHA256SUMS" || fail "the release's signed list of installers could not be downloaded."
download -sS "$REPO/latest/download/SHA256SUMS.sig" -o "$tmp/SHA256SUMS.sig" || fail "the signature of the release's list of installers could not be downloaded."
printf '%s\n' "$SIGNERS" >"$tmp/signers"
if ! ssh-keygen -Y verify -f "$tmp/signers" -I "$SIGNER" -n agentmoshpit-install -s "$tmp/SHA256SUMS.sig" <"$tmp/SHA256SUMS" >"$tmp/verify.out" 2>&1; then
  grep -qi 'option' "$tmp/verify.out" && fail "this computer's ssh-keygen is too old to check signatures (OpenSSH 8.1 or newer is needed)."
  fail "the release's list of installers is not signed with Agent Moshpit's key. Nothing was installed."
fi
tag=$(sed -n '1s/^# Agent Moshpit \(v[0-9][0-9A-Za-z.-]*\)$/\1/p' "$tmp/SHA256SUMS")
[ -n "$tag" ] || fail "the signed list of installers names no release."
want=$(awk -v f="$file" '$2 == f && length($1) == 64 { print $1 }' "$tmp/SHA256SUMS")
[ -n "$want" ] || fail "$file is not in the signed list of installers for $tag."

say "Getting $file ($tag)"
download --progress-bar "$REPO/download/$tag/$file" -o "$tmp/$file" || fail "$file could not be downloaded."
if [ "$(sha256 "$tmp/$file")" != "$want" ]; then
  rm -f "$tmp/$file"
  fail "$file is not the file the signed list names. Nothing was installed."
fi
if [ -n "$only_fetch" ]; then
  say "Fetched and checked $file ($tag). It was not installed: this was asked only to fetch it."
  exit 0
fi

case "$(uname -s)" in
Darwin)
  dest="/Applications"
  if [ ! -w "$dest" ]; then
    dest="$HOME/Applications"
    mkdir -p "$dest"
  fi
  tar -xzf "$tmp/$file" -C "$tmp" || fail "the download could not be unpacked."
  [ -d "$tmp/Agent Moshpit.app" ] || fail "the download did not hold the app."
  rm -rf "$dest/Agent Moshpit.app"
  mv "$tmp/Agent Moshpit.app" "$dest/"
  say "Agent Moshpit is in $dest. Opening it."
  open "$dest/Agent Moshpit.app"
  ;;
Linux)
  chmod 755 "$tmp"
  case "$file" in
  *.deb)
    say "Installing it with apt, which asks for your password."
    sudo apt-get install -y "$tmp/$file" || fail "apt could not install it."
    start="agent-moshpit"
    ;;
  *.rpm)
    if command -v dnf >/dev/null 2>&1; then
      say "Installing it with dnf, which asks for your password."
      sudo dnf install -y "$tmp/$file" || fail "dnf could not install it."
    else
      say "Installing it with zypper, which asks for your password."
      sudo zypper --non-interactive install --allow-unsigned-rpm "$tmp/$file" || fail "zypper could not install it."
    fi
    start="agent-moshpit"
    ;;
  *)
    mkdir -p "$HOME/.local/bin"
    start="$HOME/.local/bin/agent-moshpit"
    mv "$tmp/$file" "$start"
    chmod +x "$start"
    say "Agent Moshpit is $start."
    ;;
  esac
  if [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
    say "Starting it."
    nohup "$start" >/dev/null 2>&1 &
  else
    say "Start it with: $start"
  fi
  ;;
esac
}

main "$@"
