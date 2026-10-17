#!/usr/bin/env sh
set -eu
printf '%s\n' 'HostGates : diagnostic seulement, aucune installation.'
for tool in cargo rustc cc pkg-config ssh sftp xfreerdp3 xfreerdp; do
  if command -v "$tool" >/dev/null 2>&1; then command -v "$tool"; else printf '%s : absent\n' "$tool"; fi
done
if command -v pkg-config >/dev/null 2>&1; then
  for package in dbus-1 x11 xkbcommon wayland-client egl; do
    pkg-config --modversion "$package" || true
  done
fi
printf '%s\n' 'Secret Service nécessite une session DBus et un coffre utilisateur déverrouillable.'
