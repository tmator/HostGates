#!/usr/bin/env python3
"""Build native DEB/RPM packages from a Linux release binary; never installs them."""
import hashlib
import json
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def run(*args, cwd=ROOT):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def copy(source, destination, executable=False):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    destination.chmod(0o755 if executable else 0o644)


def main():
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        raise SystemExit("Run this script on x86_64 Linux (including WSL2).")
    for tool in ("cargo", "dpkg-deb", "dpkg-shlibdeps", "rpmbuild", "readelf", "desktop-file-validate"):
        if not shutil.which(tool):
            raise SystemExit(f"Missing build tool: {tool}")
    metadata = json.loads(run("cargo", "metadata", "--locked", "--format-version", "1"))
    package = next(p for p in metadata["packages"] if p["id"] == metadata["resolve"]["root"])
    version = package["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise SystemExit("Expected a numeric release version.")
    binary = Path(metadata["target_directory"]) / "release" / "hostgates"
    if binary.read_bytes()[:4] != b"\x7fELF":
        raise SystemExit("Expected a Linux ELF executable, not a Windows binary.")
    symbols = run("readelf", "--version-info", str(binary))
    versions = {tuple(map(int, v.split("."))) for v in re.findall(r"GLIBC_(\d+\.\d+(?:\.\d+)?)", symbols)}
    glibc = max(versions)
    # RHEL/Rocky/Alma 9 ships glibc 2.34. Refuse to silently ship a newer ABI.
    if glibc > (2, 34):
        raise SystemExit(f"Requires glibc {glibc}; rebuild on an older Linux baseline for RHEL 9.")
    work_parent = ROOT / "target" / "package-work"
    work_parent.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="hostgates-", dir=work_parent))
    stage = work / "payload"
    output = ROOT / "target" / "linux-release"
    output.mkdir(parents=True, exist_ok=True)
    copy(binary, stage / "usr/bin/hostgates", executable=True)
    copy(ROOT / "packaging/linux/hostgates.desktop", stage / "usr/share/applications/hostgates.desktop")
    copy(ROOT / "packaging/linux/hostgates.svg", stage / "usr/share/icons/hicolor/scalable/apps/hostgates.svg")
    run("desktop-file-validate", str(stage / "usr/share/applications/hostgates.desktop"))
    documentation = stage / "usr/share/doc/hostgates"
    for name in ("LICENSE", "licence.md", "README.md", "securite.txt"):
        copy(ROOT / name, documentation / name)
    for path in (ROOT / "docs").rglob("*"):
        if path.is_file() and path.name not in {"REQUIREMENTS.md", "STATUS.md", "GITHUB.md"}:
            copy(path, documentation / "docs" / path.relative_to(ROOT / "docs"))
    for path in (ROOT / "locales").glob("*.json"):
        copy(path, stage / "usr/share/hostgates/locales" / path.name)
    for path in (ROOT / "assets/fonts").iterdir():
        if path.suffix in {".txt", ".json", ".md"}:
            copy(path, documentation / "fonts" / path.name)
    for dependency in metadata["packages"]:
        if not dependency.get("source"):
            continue
        folder = Path(dependency["manifest_path"]).parent
        dest = documentation / "third-party" / f'{dependency["name"]}-{dependency["version"]}'
        for path in folder.rglob("*"):
            if not path.is_file():
                continue
            if (path.name.lower().startswith(("license", "licence", "copying", "copyright", "notice"))
                    or dependency["name"] == "option-ext"
                    or (dependency["name"] == "epaint_default_fonts" and path.suffix == ".txt")):
                copy(path, dest / path.relative_to(folder))
    (documentation / "third-party/README.txt").write_text(
        "Bundled dependency notices (all Cargo targets). Unmodified option-ext source is included "
        "under MPL-2.0. Other sources: https://crates.io using Cargo.lock versions. "
        "External RDP/VNC clients are not bundled. Noto font licenses are in ../fonts/.\n")

    debian = work / "debian"
    debian.mkdir()
    (debian / "control").write_text("Source: hostgates\nSection: net\nPriority: optional\nMaintainer: Brice Rouanet <tmator@gmail.com>\n\nPackage: hostgates\nArchitecture: amd64\nDescription: Remote connection manager\n")
    dependencies = run("dpkg-shlibdeps", "-O", "-e" + str(binary), cwd=work)
    dependencies = next(line.removeprefix("shlibs:Depends=") for line in dependencies.splitlines() if line.startswith("shlibs:Depends="))
    # Loaded at runtime by winit/glutin, so these are not all visible to ELF scanners.
    dependencies += ", libx11-6, libxcursor1, libxrandr2, libxi6, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libegl1, libgl1, xdg-utils"
    control_dir = stage / "DEBIAN"
    control_dir.mkdir()
    size = sum(p.stat().st_size for p in stage.rglob("*") if p.is_file()) // 1024
    (control_dir / "control").write_text(f"""Package: hostgates
Version: {version}-1
Section: net
Priority: optional
Architecture: amd64
Maintainer: Brice Rouanet <tmator@gmail.com>
Installed-Size: {size}
Depends: {dependencies}
Recommends: xdg-desktop-portal, xdg-desktop-portal-gtk | xdg-desktop-portal-kde, gnome-keyring
Suggests: openssh-client, tigervnc-viewer
Homepage: https://github.com/tmator/HostGates
Description: Native SSH, RDP, VNC and web connection manager
 Integrated SSH terminal, SFTP, tunnels, encrypted vault and 12 interface languages.
 RDP and VNC require separately installed clients. A graphical desktop is required.
""")
    deb = output / f"hostgates_{version}-1_amd64.deb"
    subprocess.run(["dpkg-deb", "--root-owner-group", "-Zxz", "--build", str(stage), str(deb)], check=True)
    # DEBIAN metadata is not part of an RPM payload. Only copy the usr tree below.
    top = work / "rpm"
    for directory in ("BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"):
        (top / directory).mkdir(parents=True)
    spec = top / "SPECS/hostgates.spec"
    spec.write_text(f"""%global debug_package %{{nil}}
%global _build_id_links none
%global __strip /bin/true
Name: hostgates
Version: {version}
Release: 1
Summary: Native SSH, RDP, VNC and web connection manager
License: MIT
URL: https://github.com/tmator/HostGates
ExclusiveArch: x86_64
Requires: libX11.so.6()(64bit), libXcursor.so.1()(64bit), libXrandr.so.2()(64bit), libXi.so.6()(64bit)
Requires: libxkbcommon.so.0()(64bit), libxkbcommon-x11.so.0()(64bit), libwayland-client.so.0()(64bit)
Requires: libEGL.so.1()(64bit), libGL.so.1()(64bit), xdg-utils
Recommends: xdg-desktop-portal, gnome-keyring

%description
Integrated SSH terminal, SFTP, tunnels, encrypted vault and 12 interface languages.
RDP and VNC require separately installed clients. A graphical desktop is required.
Bundled dependency and font licenses accompany the package in its documentation.

%prep
%build
%install
mkdir -p "%{{buildroot}}/usr"
cp -a {shlex.quote(str(stage / 'usr'))}/. "%{{buildroot}}/usr/"

%files
%defattr(-,root,root,-)
/usr/bin/hostgates
/usr/share/applications/hostgates.desktop
/usr/share/icons/hicolor/scalable/apps/hostgates.svg
/usr/share/hostgates
%doc /usr/share/doc/hostgates
""")
    subprocess.run(["rpmbuild", "-bb", "--define", f"_topdir {top}", str(spec)], check=True)
    rpm = next((top / "RPMS/x86_64").glob("*.rpm"))
    copy(rpm, output / rpm.name)
    report = {"version": version, "architecture": "x86_64", "glibc_required": ".".join(map(str, glibc)),
              "deb_dependencies": dependencies, "rpm_dependencies": run("rpm", "-qpR", str(output / rpm.name)).splitlines()}
    (output / f"hostgates-{version}-linux-build.json").write_text(json.dumps(report, indent=2) + "\n")
    files = [deb, output / rpm.name]
    (output / f"SHA256SUMS-{version}-linux.txt").write_text("".join(hashlib.sha256(p.read_bytes()).hexdigest() + "  " + p.name + "\n" for p in files))
    print(f"Packages ready in {output}; minimum GLIBC {report['glibc_required']}")


if __name__ == "__main__":
    main()
