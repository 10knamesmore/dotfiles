"""Own background compositor/app process groups until the worker's pipe closes."""

import ctypes
import json
import os
from pathlib import Path
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time


def run():
    # Adopt and reap Hyprland/app descendants after their immediate parent exits.
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise OSError(ctypes.get_errno(), "cannot supervise background descendants")
    environment = dict(os.environ)
    runtime = Path(environment["XDG_RUNTIME_DIR"])
    width, height = map(int, sys.argv[1:3])
    root = Path(tempfile.mkdtemp(prefix="computer-use-"))
    children = []
    target = None
    inner_pid = None
    instance_directory = None
    kwin_socket = f"computer-use-{os.getpid()}"
    desktop_socket = f"computer-use-desktop-{os.getpid()}"
    pending = bytearray()

    def log(operation, outcome):
        path = Path(environment.get("COMPUTER_USE_LOG") or tempfile.gettempdir() + "/computer-use-sdk.log")
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            mode = "w" if path.exists() and path.stat().st_size >= 1048576 else "a"
            with path.open(mode) as stream:
                stream.write(f"{int(time.time())} pid={os.getpid()} {operation} {outcome}\n")
        except OSError:
            pass

    def owned_groups():
        groups = set()
        parents = [os.getpid()]
        while parents:
            parent = parents.pop()
            for task in Path(f"/proc/{parent}/task").glob("*/children"):
                try:
                    pids = task.read_text().split()
                except FileNotFoundError:
                    continue
                for value in pids:
                    pid = int(value)
                    try:
                        groups.add(os.getpgid(pid))
                        parents.append(pid)
                    except ProcessLookupError:
                        pass
        return groups

    def remember_instance():
        nonlocal instance_directory
        if inner_pid is None or instance_directory is not None:
            return
        for descriptor in Path(f"/proc/{inner_pid}/fd").glob("*"):
            try:
                path = descriptor.resolve()
            except FileNotFoundError:
                continue
            if path.name == "hyprland.log" and path.parent.parent == runtime / "hypr":
                instance_directory = path.parent
                return

    def find_target():
        for lock in (runtime / "hypr").glob("*/hyprland.lock"):
            try:
                lines = lock.read_text().splitlines()
            except FileNotFoundError:
                continue
            if len(lines) >= 2 and lines[0] == str(inner_pid):
                return {"runtime": str(runtime), "display": lines[1], "signature": lock.parent.name}
        return None

    def reply(value):
        print(json.dumps(value), flush=True)

    def command(timeout):
        if b"\n" not in pending:
            if not select.select([sys.stdin.fileno()], [], [], timeout)[0]:
                return None
            chunk = os.read(sys.stdin.fileno(), 65536)
            if not chunk:
                raise EOFError
            pending.extend(chunk)
        if b"\n" not in pending:
            return None
        line, _, rest = pending.partition(b"\n")
        pending[:] = rest
        return json.loads(line)

    def terminate(_signum, _frame):
        raise SystemExit

    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(sig, terminate)

    try:
        hyprland = shutil.which("Hyprland")
        kwin = shutil.which("kwin_wayland")
        if not hyprland or not kwin:
            raise RuntimeError("create_background requires Hyprland and kwin_wayland on PATH")
        config = root / "hyprland.lua"
        config.write_text(
            "hl.config({xwayland={enabled=false},cursor={no_hardware_cursors=true,"
            "enable_hyprcursor=false,inactive_timeout=0},misc={disable_hyprland_logo=true,"
            "disable_splash_rendering=true}})\n"
            f'hl.monitor({{output="WAYLAND-1",mode="{width}x{height}@60",scale=1}})\n'
            'hl.layer_rule({name="computer_control",match={namespace="^pi-computer-control$"},no_anim=true})\n'
        )
        # Restore the worker's development environment for Hyprland; only KWin gets temporary XDG homes.
        inner_env = environment.copy()
        for name in ("HYPRLAND_INSTANCE_SIGNATURE", "DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "WAYLAND_SOCKET"):
            inner_env.pop(name, None)
        inner_env.update(
            LIBSEAT_BACKEND="seatd",
            SEATD_SOCK=str(root / "no-seatd.sock"),
            HYPRLAND_NO_SD_NOTIFY="1",
            HYPRLAND_NO_SD_VARS="1",
            HYPRLAND_NO_CRASHREPORTER="1",
        )
        inner = root / "inner.py"
        inner.write_text(
            f"#!{sys.executable}\nimport os, socket\nfrom pathlib import Path\n"
            f"environment = {inner_env!r}\n"
            "environment['WAYLAND_DISPLAY'] = os.environ['WAYLAND_DISPLAY']\n"
            f"Path({str(root / 'inner.pid')!r}).write_text(str(os.getpid()))\n"
            f"if Path({str(root / 'closing')!r}).exists(): raise SystemExit\n"
            f"server = socket.socket(socket.AF_UNIX)\nserver.bind({str(runtime / desktop_socket)!r})\nserver.listen()\n"
            "os.set_inheritable(server.fileno(), True)\n"
            f"os.execve({hyprland!r}, [{hyprland!r}, '--config', {str(config)!r}, '--socket', {desktop_socket!r}, '--wayland-fd', str(server.fileno())], environment)\n"
        )
        inner.chmod(0o700)
        kwin_env = {name: environment[name] for name in ("HOME", "USER", "LOGNAME", "PATH", "LANG", "XDG_RUNTIME_DIR") if name in environment}
        for category in ("CONFIG", "STATE", "CACHE", "DATA"):
            directory = root / f"kwin-{category.lower()}"
            directory.mkdir()
            kwin_env[f"XDG_{category}_HOME"] = str(directory)
        with (root / "compositor.log").open("wb") as compositor_log:
            compositor = subprocess.Popen(
                [kwin, "--virtual", "--socket", kwin_socket, "--width", str(width), "--height", str(height),
                 "--scale", "2", "--no-lockscreen", "--no-global-shortcuts", "--no-kactivities",
                 "--exit-with-session", str(inner)],
                env=kwin_env, stdin=subprocess.DEVNULL, stdout=compositor_log, stderr=compositor_log, start_new_session=True,
            )
        children.append(compositor)
        deadline = time.monotonic() + 20
        while target is None:
            command(0.02)  # EOF during startup must clean half-created resources too.
            if compositor.poll() is not None:
                raise RuntimeError(f"background compositor exited during startup ({compositor.returncode})")
            if time.monotonic() >= deadline:
                raise RuntimeError("background Hyprland startup timed out")
            pid_file = root / "inner.pid"
            if inner_pid is None and pid_file.exists():
                inner_pid = int(pid_file.read_text())
            if inner_pid is not None:
                remember_instance()
                candidate = find_target()
                if candidate and (runtime / "hypr" / candidate["signature"] / ".socket.sock").exists():
                    target = candidate
        log("background.supervisor", "ready")
        reply(target)
        while True:
            if compositor.poll() is not None:
                raise RuntimeError("background compositor exited")
            try:
                os.kill(inner_pid, 0)
            except ProcessLookupError:
                raise RuntimeError("background Hyprland exited") from None
            request = command(0.02)
            for child in children:
                child.poll()
            if request is None:
                continue
            if request["type"] == "close":
                break
            if request["type"] == "launch":
                app_env = environment.copy()
                app_env.update(WAYLAND_DISPLAY=target["display"], HYPRLAND_INSTANCE_SIGNATURE=target["signature"], XDG_RUNTIME_DIR=target["runtime"])
                for name in ("DISPLAY", "WAYLAND_SOCKET"):
                    app_env.pop(name, None)
                try:
                    child = subprocess.Popen(request["argv"], cwd=request["cwd"], env=app_env,
                                             stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                             stderr=subprocess.DEVNULL, start_new_session=True)
                except (OSError, ValueError) as error:
                    reply({"error": str(error)})
                else:
                    children.append(child)
                    reply({"pid": child.pid})
    except (EOFError, BrokenPipeError):
        log("background.worker", "disconnected")
    except Exception as error:
        log("background.supervisor", "failed")
        try:
            reply({"error": str(error)})
        except BrokenPipeError:
            pass
    finally:
        # SIGTERM remains ignored during bounded cleanup so repeated cancellation cannot skip it.
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(sig, signal.SIG_IGN)
        (root / "closing").touch()
        if inner_pid is None and (root / "inner.pid").exists():
            inner_pid = int((root / "inner.pid").read_text())
        # The log FD identifies this instance before Hyprland writes hyprland.lock.
        deadline = time.monotonic() + 1
        while inner_pid is not None and instance_directory is None and time.monotonic() < deadline:
            remember_instance()
            if not Path(f"/proc/{inner_pid}").exists():
                break
            time.sleep(0.02)
        for sig in (signal.SIGTERM, signal.SIGKILL):
            for group in owned_groups():
                try:
                    os.killpg(group, sig)
                except ProcessLookupError:
                    pass
            deadline = time.monotonic() + (1.5 if sig == signal.SIGTERM else 0.5)
            while time.monotonic() < deadline:
                if all(child.poll() is not None for child in children):
                    break
                time.sleep(0.02)
        for child in children:
            child.wait()
        while True:
            # Reparented descendants can have their own process groups (for example an app helper).
            adopted = Path(f"/proc/self/task/{os.getpid()}/children").read_text().split()
            for pid in adopted:
                try:
                    os.kill(int(pid), signal.SIGKILL)
                except ProcessLookupError:
                    pass
            try:
                os.waitpid(-1, 0)
            except ChildProcessError:
                break
        # A cancelled startup may have created sockets before reporting its target.
        if target is None and inner_pid is not None:
            target = find_target()
        # These names are owned by this supervisor and cannot refer to the user's compositor.
        for name in (kwin_socket, desktop_socket):
            for suffix in ("", ".lock"):
                try:
                    (runtime / (name + suffix)).unlink(missing_ok=True)
                except OSError:
                    log("background.socket.cleanup", "failed")
        if instance_directory is None and target:
            instance_directory = runtime / "hypr" / target["signature"]
        for directory in (instance_directory, root):
            if directory is not None:
                try:
                    shutil.rmtree(directory)
                except FileNotFoundError:
                    pass
                except OSError:
                    log("background.directory.cleanup", "failed")
        log("background.supervisor.close", "reaped")


run()
