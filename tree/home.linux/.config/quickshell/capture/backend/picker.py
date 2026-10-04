#!/usr/bin/env python3
"""向当前 Quickshell 请求 Portal 来源选择，输出 XDPH 的 selection 行。

仅转发 XDPH_WINDOW_SHARING_LIST，不根据进程外的待选目标批准请求。
无后端时快速失败；拒绝、socket 断开或脚本退出均不输出 selection。
XDPH 的 --allow-token 参数不启用 restore token，选择行始终不带 r。
"""

from __future__ import annotations

import json
import logging
import os
import socket
import sys

from common import LOG, socket_path


def parse_windows(value: str) -> list[dict]:
    """拆解 XDPH 的窗口列表，保留 Portal token 并把十进制地址转成 Hyprland 十六进制。

    每条记录为 ID[HC>]class[HT>]title[HE>]address[HA>]；address 可为 0，表示
    Portal 未提供映射。token 与 compositor 地址不能互换。
    """
    windows = []
    for item in value.split("[HA>]"):
        if not item:
            continue
        token, remaining = item.split("[HC>]", 1)
        app_id, remaining = remaining.split("[HT>]", 1)
        title, address = remaining.split("[HE>]", 1)
        windows.append({"id": str(int(token)), "address": hex(int(address)), "title": title, "appId": app_id})
    return windows


def main() -> int:
    """建立短连接并等待用户确认；只有成功选择写入 stdout。"""
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s %(message)s", stream=sys.stderr)
    try:
        windows = parse_windows(os.environ.get("XDPH_WINDOW_SHARING_LIST", ""))
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(0.5)
            connection.connect(str(socket_path()))
            connection.sendall(json.dumps({"type": "picker", "windows": windows}, ensure_ascii=False).encode() + b"\n")
            connection.settimeout(None)
            with connection.makefile("rb") as response:
                line = response.readline()
            if not line:
                LOG.info("picker server disconnected")
                return 1
            result = json.loads(line)
            if result["cancelled"]:
                LOG.info("picker cancelled")
                return 1
            selection = result["selection"]
            if not isinstance(selection, str) or not selection.startswith("[SELECTION]/") or "\n" in selection:
                LOG.error("picker server returned an invalid selection")
                return 1
            print(selection, flush=True)
            return 0
    except KeyboardInterrupt:
        LOG.info("picker interrupted")
        return 1
    except (OSError, KeyError, ValueError) as error:
        LOG.error("picker could not complete: %s", error)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
