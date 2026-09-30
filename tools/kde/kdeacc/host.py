# SPDX-License-Identifier: GPL-3.0-or-later
"""The host: distribution, kernel, Plasma, KWin, Frameworks, Qt, Mesa, GPU, portals, session.

Every version is queried from the machine at run time (rpm, the tools' own
`--version`, glxinfo); nothing is copied from an earlier session. Host tools
run in the clean host environment, never with the bundled Qt's paths.
"""

from __future__ import annotations

import os
import re

from .shell import host_env

RPM_VERSION_RE = re.compile(r"^(?P<name>.+?)-(?P<version>\d[^-]*)-(?P<release>[^-]+)\.(?P<arch>[^.]+)$")


def parse_rpm_version(nvra):
    """'plasma-workspace-6.7.5-1.fc44.x86_64' -> '6.7.5'; None for 'package x is not installed'."""
    match = RPM_VERSION_RE.match(nvra.strip())
    return match.group("version") if match else None


def parse_os_release(text):
    out = {}
    for line in text.splitlines():
        if "=" in line and not line.startswith("#"):
            key, value = line.split("=", 1)
            out[key] = value.strip().strip('"')
    return out


def parse_tool_version(text):
    """'plasmashell 6.7.5' or 'kwin 6.7.5' -> '6.7.5'."""
    match = re.search(r"\b(\d+\.\d+(?:\.\d+)*)\b", text)
    return match.group(1) if match else None


def parse_glxinfo(text):
    """{'renderer', 'accelerated', 'opengl'} from `glxinfo -B`."""
    def grab(pattern):
        match = re.search(pattern, text, re.M)
        return match.group(1).strip() if match else None
    return {
        "renderer": grab(r"OpenGL renderer string:\s*(.+)"),
        "accelerated": grab(r"Accelerated:\s*(\w+)"),
        "opengl": grab(r"OpenGL version string:\s*(.+)"),
    }


def parse_session(text):
    """{'Type': 'wayland', 'Desktop': 'KDE'} from `loginctl show-session -p ...`."""
    return dict(line.split("=", 1) for line in text.splitlines() if "=" in line)


def _first_line(done):
    return done.out.strip().splitlines()[0] if done.out.strip() else None


def rpm_versions(runner, names):
    out = {}
    for name in names:
        done = runner.run(["rpm", "-q", "--qf", "%{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}\n", name], env=host_env(), tag="host-rpm")
        out[name] = parse_rpm_version(done.out) if done.ok else None
    return out


def query_host(runner, qt_dir, repo):
    """The host record (host.json). Any field the machine cannot answer is None, never a guess."""
    env = host_env()
    rpms = rpm_versions(runner, [
        "kf6-kcoreaddons", "plasma-workspace", "plasma-integration", "kwin", "qt6-qtbase", "qt6-qtdeclarative",
        "qt6-qtwayland", "mesa-dri-drivers", "xdg-desktop-portal", "xdg-desktop-portal-kde", "breeze-icon-theme",
        "at-spi2-core", "xorg-x11-server-Xvfb", "podman"])
    osr = parse_os_release(open("/etc/os-release").read()) if os.path.exists("/etc/os-release") else {}
    session_id = os.environ.get("XDG_SESSION_ID", "")
    session = parse_session(runner.run(["loginctl", "show-session", session_id, "-p", "Type", "-p", "Desktop"], env=env, tag="host-session").out) if session_id else {}
    glx = parse_glxinfo(runner.run(["glxinfo", "-B"], env=env, tag="host-glx").out)
    gpus = [ln.split(": ", 1)[1] for ln in runner.run(["lspci"], env=env, tag="host-lspci").out.splitlines()
            if "VGA" in ln or "3D controller" in ln]
    qmake = runner.run([f"{qt_dir}/bin/qmake", "-query", "QT_VERSION"], env=env, tag="host-bundled-qt")
    rustc = runner.run(["rustc", "--version"], cwd=repo, env=env, tag="host-rustc")
    return {
        "fedora": {"pretty": osr.get("PRETTY_NAME"), "version_id": osr.get("VERSION_ID"), "variant": osr.get("VARIANT")},
        "kernel": runner.run(["uname", "-r"], env=env, tag="host-uname").out.strip() or None,
        "session": {"type": session.get("Type"), "desktop": session.get("Desktop"),
                    "wayland_display": os.environ.get("WAYLAND_DISPLAY"), "xdg_current_desktop": os.environ.get("XDG_CURRENT_DESKTOP")},
        "plasma": parse_tool_version(runner.run(["plasmashell", "--version"], env=env, tag="host-plasma").out),
        "kwin": parse_tool_version(runner.run(["kwin_wayland", "--version"], env=env, tag="host-kwin").out),
        "kde_frameworks": rpms.get("kf6-kcoreaddons"),
        "host_qt": rpms.get("qt6-qtbase"),
        "bundled_qt": qmake.out.strip() or None,
        "mesa": rpms.get("mesa-dri-drivers"),
        "gpu_renderer": glx["renderer"], "gpu_accelerated": glx["accelerated"], "opengl": glx["opengl"],
        "gpus": gpus,
        "xdg_desktop_portal": rpms.get("xdg-desktop-portal"),
        "xdg_desktop_portal_kde": rpms.get("xdg-desktop-portal-kde"),
        "plasma_integration": rpms.get("plasma-integration"),
        "breeze_icon_theme": rpms.get("breeze-icon-theme"),
        "rustc": (rustc.out.strip() or None),
        "packages": rpms,
    }


def host_gaps(host):
    """Fields the run needs and could not get, named; an empty list means the record is complete."""
    need = ["kernel", "plasma", "kwin", "kde_frameworks", "host_qt", "bundled_qt", "mesa", "gpu_renderer",
            "xdg_desktop_portal", "xdg_desktop_portal_kde"]
    gaps = [k for k in need if not host.get(k)]
    if not host["fedora"].get("version_id"):
        gaps.append("fedora")
    if host["session"].get("type") != "wayland":
        gaps.append("session is not Wayland")
    if not is_plasma_session(host["session"]):
        # Plasma's packages can be installed under GNOME or Sway: every version above would
        # resolve and the run would present another desktop's results as native Plasma evidence.
        gaps.append(f"session is not KDE Plasma (desktop {host['session'].get('desktop')!r}, "
                    f"XDG_CURRENT_DESKTOP {host['session'].get('xdg_current_desktop')!r})")
    return gaps


def is_plasma_session(session):
    """Whether the login session is a KDE Plasma one: logind's Desktop or XDG_CURRENT_DESKTOP names KDE."""
    names = [session.get("desktop") or "", session.get("xdg_current_desktop") or ""]
    return any("KDE" in part.upper().split(":") for name in names for part in [name])
