#!/usr/bin/env python3
"""Stream the default IPv4 interface's status when routing or NetworkManager data changes.

Requires the system's libnm and PyGObject. Route notifications select the same kernel
interface as the previous ip-route query; libnm supplies IPv4 addresses and Wi-Fi data.
"""

import json
import logging
import socket

import gi

gi.require_version("NM", "1.0")
from gi.repository import GLib, NM

LOG = logging.getLogger("network-status")
RTMGRP_IPV4_ROUTE = 0x40


def default_interface() -> str:
    """Select the up default IPv4 route with the lowest metric from the kernel table."""
    routes = []
    with open("/proc/net/route") as source:
        next(source)
        for line in source:
            fields = line.split()
            if fields[1] == "00000000" and int(fields[3], 16) & 1:
                routes.append((int(fields[6]), fields[0]))
    return min(routes, key=lambda route: route[0])[1] if routes else ""


class NetworkStatus:
    """Observe the selected interface and emit only changed structured snapshots."""

    def __init__(self):
        self.client = NM.Client.new(None)
        self.observers = {}
        self.refresh_pending = False
        self.previous = None
        self.client.connect("notify", self.queue_refresh)
        self.routes = socket.socket(socket.AF_NETLINK, socket.SOCK_RAW, socket.NETLINK_ROUTE)
        self.routes.bind((0, RTMGRP_IPV4_ROUTE))
        GLib.io_add_watch(self.routes, GLib.IO_IN, self.route_changed)
        self.queue_refresh()

    def queue_refresh(self, *_args):
        """Coalesce related D-Bus and route notifications into one snapshot."""
        if not self.refresh_pending:
            self.refresh_pending = True
            GLib.idle_add(self.refresh)

    def route_changed(self, _source, _condition):
        """Re-read the route table after a kernel notification, without polling."""
        self.routes.recv(65536)
        self.queue_refresh()
        return GLib.SOURCE_CONTINUE

    def observe(self, objects):
        """Follow property changes only on the current device, IP config and access point."""
        current = {obj for obj in objects if obj is not None}
        for obj in self.observers.keys() - current:
            obj.handler_disconnect(self.observers.pop(obj))
        for obj in current - self.observers.keys():
            self.observers[obj] = obj.connect("notify", self.queue_refresh)

    def refresh(self):
        """Publish connection kind, interface, IPv4/prefix list and active Wi-Fi details."""
        self.refresh_pending = False
        interface = default_interface()
        device = self.client.get_device_by_iface(interface) if interface else None
        ip_config = device.get_ip4_config() if device else None
        wifi = isinstance(device, NM.DeviceWifi)
        access_point = device.get_active_access_point() if wifi else None
        self.observe((device, ip_config, access_point))
        ssid = access_point.get_ssid() if access_point else None
        snapshot = {
            "connectionType": "disconnected" if not interface else "wifi" if wifi else "ethernet",
            "interfaceName": interface,
            "address": " ".join(
                f"{address.get_address()}/{address.get_prefix()}"
                for address in ip_config.get_addresses()
            ) if ip_config else "",
            "ssid": NM.utils_ssid_to_utf8(ssid.get_data()) if ssid else "",
            "signalStrength": access_point.get_strength() if access_point else 0,
        }
        if snapshot != self.previous:
            if self.previous is None or any(
                snapshot[key] != self.previous[key]
                for key in ("connectionType", "interfaceName", "address", "ssid")
            ):
                LOG.info("connection=%s interface=%s", snapshot["connectionType"], interface or "none")
            self.previous = snapshot
            print(json.dumps(snapshot, ensure_ascii=False, separators=(",", ":")), flush=True)
        return GLib.SOURCE_REMOVE


def main():
    """Keep the event-driven collector alive until Quickshell closes its process."""
    logging.basicConfig(level=logging.INFO, format="[network-status] %(message)s")
    status = NetworkStatus()
    try:
        GLib.MainLoop().run()
    finally:
        status.routes.close()


if __name__ == "__main__":
    main()
