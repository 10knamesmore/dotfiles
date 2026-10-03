#!/usr/bin/env python3
"""读取 Codex 周额度与 DeepSeek 余额，输出供顶栏消费的结构化状态。

只读 $CODEX_HOME/auth.json（默认 ~/.codex）与 DEEPSEEK_API_KEY；不刷新或写回凭据。
HTTP 请求继承桌面进程的代理设置，凭据不进入命令行，日志不包含响应正文。
"""

import json
import os
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from http.client import HTTPException
from pathlib import Path


def get_json(url: str, token: str, headers: dict | None = None) -> dict:
    """向指定提供商发送只读请求，网络或 HTTP 失败交由调用方分类。"""
    request = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/json",
            **(headers or {}),
        },
    )
    with urllib.request.urlopen(request, timeout=15) as response:
        return json.load(response)


def codex_usage() -> dict:
    """读取标准 Codex 七天窗口；按窗口时长识别，不假设它位于 secondary。"""
    auth_path = Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex"))) / "auth.json"
    try:
        auth = json.loads(auth_path.read_text())
        token = auth["tokens"]["access_token"]
        account_id = auth["tokens"]["account_id"]
        if auth.get("auth_mode") != "chatgpt" or not token or not account_id:
            return {"status": "auth_required"}
    except FileNotFoundError:
        return {"status": "missing_credentials"}
    except (OSError, ValueError, KeyError, TypeError):
        return {"status": "auth_required"}

    data = get_json(
        "https://chatgpt.com/backend-api/wham/usage",
        token,
        {"ChatGPT-Account-Id": account_id, "User-Agent": "codex-cli"},
    )
    limits = data["rate_limit"]
    for name in ("primary_window", "secondary_window"):
        window = limits.get(name)
        if window and window["limit_window_seconds"] == 604800:
            return {
                "status": "ok",
                "usedPercent": window["used_percent"],
                "resetAt": window["reset_at"],
            }
    return {"status": "no_weekly_limit"}


def deepseek_balance() -> dict:
    """读取含赠送与充值部分的可用余额，保留接口给出的币种。"""
    token = os.environ.get("DEEPSEEK_API_KEY")
    if not token:
        return {"status": "missing_credentials"}
    data = get_json("https://api.deepseek.com/user/balance", token)
    balances = [
        {"currency": balance["currency"], "total": float(balance["total_balance"])}
        for balance in data["balance_infos"]
    ]
    if not balances:
        raise ValueError("missing balances")
    return {
        "status": "ok",
        "balances": balances,
    }


def collect(provider: str) -> tuple[str, dict]:
    """独立采集一个提供商；失败仅输出状态码，不泄漏后端错误文本。"""
    started = time.monotonic()
    http_status = None
    try:
        result = codex_usage() if provider == "codex" else deepseek_balance()
    except urllib.error.HTTPError as error:
        http_status = error.code
        if error.code in (401, 403):
            status = "auth_required"
        elif error.code == 429:
            status = "rate_limited"
        else:
            status = "unavailable"
        result = {"status": status}
    except (urllib.error.URLError, OSError, HTTPException):
        result = {"status": "network_error"}
    except (ValueError, KeyError, TypeError):
        result = {"status": "invalid_response"}
    result["checkedAt"] = int(time.time())
    elapsed_ms = round((time.monotonic() - started) * 1000)
    print(
        f"[ai-usage] provider={provider} status={result['status']} "
        f"http={http_status or '-'} elapsed_ms={elapsed_ms}",
        file=sys.stderr,
        flush=True,
    )
    return provider, result


if __name__ == "__main__":
    with ThreadPoolExecutor(max_workers=2) as pool:
        print(json.dumps(dict(pool.map(collect, ("codex", "deepseek")))))
