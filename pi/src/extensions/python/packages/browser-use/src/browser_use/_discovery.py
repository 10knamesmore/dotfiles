"""Resolve desktop process IDs to installed Chromium browsers and CDP endpoints."""

import os
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

import psutil


@dataclass(frozen=True)
class BrowserInstance:
    """Describe a current-user browser main process, which may own several windows.

    Attributes:
        pid: Main process PID, usable with connect().
        name: Operating-system process name.
        executable: Browser executable path.
        user_data_dir: Profile root, not its Default or Profile N subdirectory.
    """

    pid: int
    name: str
    executable: str
    user_data_dir: str


_PROFILES = {
    "chrome": ("google-chrome", "Google/Chrome"),
    "google chrome": ("google-chrome", "Google/Chrome"),
    "google chrome beta": ("google-chrome-beta", "Google/Chrome Beta"),
    "google chrome dev": ("google-chrome-unstable", "Google/Chrome Dev"),
    "google chrome canary": ("google-chrome-canary", "Google/Chrome Canary"),
    "chromium": ("chromium", "Chromium"),
    "brave": ("BraveSoftware/Brave-Browser", "BraveSoftware/Brave-Browser"),
    "brave browser": ("BraveSoftware/Brave-Browser", "BraveSoftware/Brave-Browser"),
    "msedge": ("microsoft-edge", "Microsoft Edge"),
    "microsoft edge": ("microsoft-edge", "Microsoft Edge"),
}


def _flag(process: psutil.Process, name: str) -> str | None:
    arguments = process.cmdline()
    if len(arguments) == 1:
        # Chrome can replace argv with a single title; preserve spaces in profile paths.
        title = arguments[0]
        if name == "--user-data-dir":
            for separator in (f" {name}=", f" {name} "):
                if separator in title:
                    value = title.split(separator, 1)[1].split(" --", 1)[0]
                    for end in [
                        len(value),
                        *reversed([i for i, char in enumerate(value) if char == " "]),
                    ]:
                        candidate = Path(value[:end])
                        if not candidate.is_absolute():
                            candidate = Path(process.cwd()) / candidate
                        if candidate.is_dir():
                            return str(candidate)
        arguments = title.split()
    for index, argument in enumerate(arguments):
        if argument.startswith(f"{name}="):
            return argument[len(name) + 1 :]
        if argument == name and index + 1 < len(arguments):
            return arguments[index + 1]
    return None


def _instance(process: psutil.Process) -> BrowserInstance | None:
    if process.uids().real != os.getuid():
        return None
    name = process.name()
    profiles = _PROFILES.get(name.lower())
    if profiles is None or _flag(process, "--type") is not None:
        return None
    executable = Path(process.exe())
    environment = process.environ() if sys.platform == "linux" else {}
    explicit = _flag(process, "--user-data-dir") or environment.get(
        "CHROME_USER_DATA_DIR"
    )
    if explicit:
        directory = Path(explicit)
        if not directory.is_absolute():
            directory = Path(process.cwd()) / directory
    elif sys.platform == "darwin":
        directory = Path.home() / "Library/Application Support" / profiles[1]
    else:
        root = Path(
            environment.get("CHROME_CONFIG_HOME")
            or environment.get("XDG_CONFIG_HOME")
            or Path.home() / ".config"
        )
        profile = {
            "chrome-beta": "google-chrome-beta",
            "chrome-unstable": "google-chrome-unstable",
        }.get(executable.parent.name, profiles[0])
        directory = root / profile
    return BrowserInstance(process.pid, name, str(executable), str(directory))


def discover() -> list[BrowserInstance]:
    """List current-user browser processes without connecting or asking Chrome for access."""
    browsers = []
    for process in psutil.process_iter():
        try:
            if (browser := _instance(process)) is not None:
                browsers.append(browser)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    return sorted(browsers, key=lambda browser: browser.pid)


def endpoint_from_file(path: Path) -> str:
    """Read Chrome's published debugging port and browser WebSocket path."""
    lines = path.read_text().splitlines()
    if (
        len(lines) < 2
        or not lines[0].isdigit()
        or not 0 < int(lines[0]) <= 65535
        or not lines[1].startswith("/devtools/browser/")
    ):
        raise ValueError("Invalid Chrome DevToolsActivePort file")
    return f"ws://127.0.0.1:{int(lines[0])}{lines[1]}"


def resolve(pid: int) -> tuple[BrowserInstance, str]:
    """Walk a window or renderer's parents and resolve its browser's existing CDP endpoint."""
    process: psutil.Process | None = psutil.Process(pid)
    while process is not None:
        if (browser := _instance(process)) is not None:
            try:
                return browser, endpoint_from_file(
                    Path(browser.user_data_dir) / "DevToolsActivePort"
                )
            except FileNotFoundError:
                port = _flag(process, "--remote-debugging-port")
                if port is not None and port.isdigit() and 0 < int(port) <= 65535:
                    return browser, f"http://127.0.0.1:{int(port)}"
                raise RuntimeError(
                    f"PID {browser.pid} has no CDP endpoint. Enable remote debugging in "
                    "chrome://inspect/#remote-debugging and approve Chrome's connection prompt."
                ) from None
        process = process.parent()
    raise ValueError(f"PID {pid} does not belong to a supported Chromium browser")


def installed_executable() -> str:
    """Select installed Chrome/Chromium without downloading a Playwright browser."""
    if sys.platform == "darwin":
        for app, binary in (
            ("Google Chrome", "Google Chrome"),
            ("Chromium", "Chromium"),
            ("Brave Browser", "Brave Browser"),
            ("Microsoft Edge", "Microsoft Edge"),
        ):
            for root in (Path("/Applications"), Path.home() / "Applications"):
                path = root / f"{app}.app/Contents/MacOS/{binary}"
                if path.is_file():
                    return str(path)
    else:
        for path in ("/opt/google/chrome/chrome", "/usr/lib/chromium/chromium"):
            if Path(path).is_file():
                return path
        for name in (
            "google-chrome-stable",
            "google-chrome",
            "chromium",
            "chromium-browser",
            "brave-browser",
            "microsoft-edge",
        ):
            if path := shutil.which(name):
                return path
    raise FileNotFoundError(
        "No installed Chrome/Chromium found; supply executable_path"
    )
