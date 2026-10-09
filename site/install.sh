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
set -eu

BASE="https://github.com/YashIngole/agent-moshpit/releases/latest/download"

say() { printf '%s\n' "$*"; }
fail() {
  printf 'Agent Moshpit was not installed: %s\n' "$*" >&2
  exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || fail "this needs the program '$1', which is not here."; }

need curl
tmp=$(mktemp -d 2>/dev/null || mktemp -d -t moshpit)
trap 'rm -rf "$tmp"' EXIT
trap 'exit 1' INT TERM
fetch() {
  say "Getting $1"
  curl -fL --progress-bar "$BASE/$1" -o "$tmp/$1" || fail "$1 could not be downloaded."
}

# A copy that is running has agents in it: it is the user who ends them, not this.
running() { pgrep -x agent-moshpit >/dev/null 2>&1; }
running && fail "it is running. Quit it first (that ends its agents' programs), then run this again."

case "$(uname -s)" in
Darwin)
  need tar
  case "$(uname -m)" in
  arm64) file="agent-moshpit_darwin_aarch64.app.tar.gz" ;;
  x86_64) file="agent-moshpit_darwin_x64.app.tar.gz" ;;
  *) fail "this kind of Mac ($(uname -m)) has no build." ;;
  esac
  dest="/Applications"
  if [ ! -w "$dest" ]; then
    dest="$HOME/Applications"
    mkdir -p "$dest"
  fi
  fetch "$file"
  tar -xzf "$tmp/$file" -C "$tmp" || fail "the download could not be unpacked."
  [ -d "$tmp/Agent Moshpit.app" ] || fail "the download did not hold the app."
  rm -rf "$dest/Agent Moshpit.app"
  mv "$tmp/Agent Moshpit.app" "$dest/"
  say "Agent Moshpit is in $dest. Opening it."
  open "$dest/Agent Moshpit.app"
  ;;
Linux)
  [ "$(uname -m)" = "x86_64" ] || fail "so far there is a build for 64-bit Intel and AMD only, and this is $(uname -m)."
  chmod 755 "$tmp"
  if command -v apt-get >/dev/null 2>&1; then
    file="agent-moshpit_linux_amd64.deb"
    fetch "$file"
    say "Installing it with apt, which asks for your password."
    sudo apt-get install -y "$tmp/$file" || fail "apt could not install it."
    start="agent-moshpit"
  elif command -v dnf >/dev/null 2>&1; then
    file="agent-moshpit_linux_x86_64.rpm"
    fetch "$file"
    say "Installing it with dnf, which asks for your password."
    sudo dnf install -y "$tmp/$file" || fail "dnf could not install it."
    start="agent-moshpit"
  elif command -v zypper >/dev/null 2>&1; then
    file="agent-moshpit_linux_x86_64.rpm"
    fetch "$file"
    say "Installing it with zypper, which asks for your password."
    sudo zypper --non-interactive install --allow-unsigned-rpm "$tmp/$file" || fail "zypper could not install it."
    start="agent-moshpit"
  else
    file="agent-moshpit_linux_amd64.AppImage"
    fetch "$file"
    mkdir -p "$HOME/.local/bin"
    start="$HOME/.local/bin/agent-moshpit"
    mv "$tmp/$file" "$start"
    chmod +x "$start"
    say "Agent Moshpit is $start."
  fi
  if [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
    say "Starting it."
    nohup "$start" >/dev/null 2>&1 &
  else
    say "Start it with: $start"
  fi
  ;;
*)
  fail "this script is for macOS and Linux. On Windows, in PowerShell: irm https://agentmoshpit.com/install.ps1 | iex"
  ;;
esac
