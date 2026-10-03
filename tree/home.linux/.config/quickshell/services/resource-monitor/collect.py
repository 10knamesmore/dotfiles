#!/usr/bin/env python3
"""Stream one open resource panel's Linux metrics as JSON lines every 100 ms."""

import argparse
import errno
import fcntl
import json
import logging
import os
from pathlib import Path
import socket
import struct
import time

INTERVAL = 0.1
PAGE_BYTES = os.sysconf("SC_PAGE_SIZE")
LOG = logging.getLogger("resource-monitor")


def read_cpu_times() -> tuple[int, int]:
    """Read whole-machine total and idle jiffies; per-core bar data is sampled separately."""
    with open("/proc/stat") as source:
        # guest and guest_nice are already included in user and nice.
        ticks = [int(value) for value in next(source).split()[1:9]]
    return sum(ticks), ticks[3] + ticks[4]


def cpu_usage(current, previous):
    elapsed = current[0] - previous[0]
    return 100 * (elapsed - current[1] + previous[1]) / elapsed if elapsed else 0


def read_processes():
    """Read CPU jiffies and resident bytes; start time distinguishes reused PIDs."""
    processes = {}
    with os.scandir("/proc") as entries:
        for entry in entries:
            if not entry.name.isdecimal():
                continue
            try:
                with open(f"/proc/{entry.name}/stat") as source:
                    stat = source.read()
            except (FileNotFoundError, ProcessLookupError, PermissionError):
                # A process may exit between listing /proc and opening its stat.
                continue
            end_name = stat.rfind(")")
            fields = stat[end_name + 2 :].split()
            pid = int(entry.name)
            processes[pid] = {
                "pid": pid,
                "name": stat[stat.index("(") + 1 : end_name],
                "ticks": int(fields[11]) + int(fields[12]),
                "started": int(fields[19]),
                "rssBytes": int(fields[21]) * PAGE_BYTES,
            }
    return processes


def optional_number(path):
    try:
        return float(path.read_text())
    except (FileNotFoundError, PermissionError):
        return None


def cpu_temperature_paths():
    paths = []
    for monitor in Path("/sys/class/hwmon").glob("hwmon*"):
        name = (monitor / "name").read_text().strip()
        if name not in {"k10temp", "coretemp", "zenpower", "cpu_thermal"}:
            continue
        for sensor in monitor.glob("temp*_input"):
            label = sensor.with_name(sensor.name.replace("_input", "_label"))
            if not label.exists() or label.read_text().strip() in {"Tctl", "Tdie", "Package id 0", "Package id 1"}:
                paths.append(sensor)
    return paths


class CpuSampler:
    """Measure interval CPU usage, with every process normalized to whole-machine 100%."""

    def __init__(self):
        self.previous_cpu = read_cpu_times()
        self.previous_processes = read_processes()
        self.frequency_paths = list(Path("/sys/devices/system/cpu/cpufreq").glob("policy*/scaling_cur_freq"))
        self.temperature_paths = cpu_temperature_paths()

    def sample(self):
        current = read_cpu_times()
        processes = read_processes()
        elapsed = current[0] - self.previous_cpu[0]
        ranked = []
        for pid, process in processes.items():
            previous = self.previous_processes.get(pid)
            if previous is None or previous["started"] != process["started"]:
                continue
            usage = 100 * (process["ticks"] - previous["ticks"]) / elapsed if elapsed else 0
            ranked.append({"pid": pid, "name": process["name"], "usage": usage})
        frequencies = [value / 1_000_000 for path in self.frequency_paths if (value := optional_number(path)) is not None]
        temperatures = [value / 1000 for path in self.temperature_paths if (value := optional_number(path)) is not None]
        snapshot = {
            "usage": cpu_usage(current, self.previous_cpu),
            "frequencyGHz": sum(frequencies) / len(frequencies) if frequencies else None,
            "temperatureC": max(temperatures) if temperatures else None,
            "processes": sorted(ranked, key=lambda process: process["usage"], reverse=True)[:8],
        }
        self.previous_cpu = current
        self.previous_processes = processes
        return snapshot


class MemorySampler:
    """Report available-based RAM usage, swap bytes, PSI some avg10 and top process RSS."""

    def sample(self):
        with open("/proc/meminfo") as source:
            values = {fields[0].rstrip(":"): int(fields[1]) * 1024 for line in source if (fields := line.split())}
        pressure = None
        try:
            with open("/proc/pressure/memory") as source:
                for line in source:
                    if line.startswith("some "):
                        pressure = float(dict(field.split("=") for field in line.split()[1:])["avg10"])
        except FileNotFoundError:
            pass
        largest = sorted(read_processes().values(), key=lambda process: process["rssBytes"], reverse=True)[:8]
        return {
            "totalBytes": values["MemTotal"],
            "usedBytes": values["MemTotal"] - values["MemAvailable"],
            "availableBytes": values["MemAvailable"],
            "breakdown": {
                "freeBytes": values["MemFree"],
                "buffersBytes": values["Buffers"],
                "cachedBytes": values["Cached"],
                "sharedBytes": values["Shmem"],
            },
            "swapTotalBytes": values["SwapTotal"],
            "swapUsedBytes": values["SwapTotal"] - values["SwapFree"],
            "pressureSome10": pressure,
            "processes": [{key: process[key] for key in ("pid", "name", "rssBytes")} for process in largest],
        }


def read_network_counters():
    counters = {}
    with open("/proc/net/dev") as source:
        for line in source:
            if ":" not in line:
                continue
            name, fields = line.split(":", 1)
            name = name.strip()
            if name == "lo":
                continue
            fields = fields.split()
            counters[name] = (int(fields[0]), int(fields[8]))
    return counters


def default_routes():
    routes = {}
    with open("/proc/net/route") as source:
        next(source)
        for line in source:
            fields = line.split()
            if fields[1] != "00000000" or not int(fields[3], 16) & 1:
                continue
            name, metric = fields[0], int(fields[6])
            if name not in routes or metric < routes[name][0]:
                routes[name] = (metric, socket.inet_ntoa(struct.pack("<I", int(fields[2], 16))))
    return routes


class NetworkSampler:
    """Measure per-interface bytes/s using monotonic elapsed time and interface counters."""

    def __init__(self):
        self.previous = read_network_counters()
        self.previous_time = time.monotonic()
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

    def ipv4(self, name):
        try:
            result = fcntl.ioctl(self.socket.fileno(), 0x8915, struct.pack("256s", name.encode()))
            return socket.inet_ntoa(result[20:24])
        except OSError as error:
            if error.errno in {errno.EADDRNOTAVAIL, errno.ENODEV}:
                return ""
            raise

    def sample(self):
        current = read_network_counters()
        now = time.monotonic()
        elapsed = now - self.previous_time
        routes = default_routes()
        default = min(routes, key=lambda name: routes[name][0]) if routes else ""
        interfaces = []
        for name, (received, sent) in current.items():
            previous = self.previous.get(name, (received, sent))
            interfaces.append({
                "name": name,
                "ipv4": self.ipv4(name),
                "gateway": routes[name][1] if name in routes else "",
                "downSpeed": max(0, received - previous[0]) / elapsed,
                "upSpeed": max(0, sent - previous[1]) / elapsed,
                "downTotal": received,
                "upTotal": sent,
            })
        self.previous = current
        self.previous_time = now
        return {"defaultInterface": default, "interfaces": interfaces}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("resource", choices=("cpu", "memory", "network"))
    resource = parser.parse_args().resource
    logging.basicConfig(level=logging.INFO, format="[resource-monitor] %(message)s")
    sampler = {"cpu": CpuSampler, "memory": MemorySampler, "network": NetworkSampler}[resource]()
    deadline = time.monotonic() + INTERVAL
    while True:
        time.sleep(max(0, deadline - time.monotonic()))
        snapshot = sampler.sample()
        print(json.dumps({"resource": resource, "time": time.time(), "snapshot": snapshot}, separators=(",", ":")), flush=True)
        deadline += INTERVAL
        if deadline < time.monotonic():
            deadline = time.monotonic() + INTERVAL


if __name__ == "__main__":
    try:
        main()
    except BrokenPipeError:
        pass
    except Exception:
        LOG.exception("sampling failed")
        raise SystemExit(1)
