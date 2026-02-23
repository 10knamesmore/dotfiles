import argparse
import hashlib
import hmac
import json
import re
import time
import urllib.parse
import urllib.request

# 登录（不传 ip）
# python bsu_login.py --username 账号 --password 密码

# 登录（指定 ip）
# python bsu_login.py --username 账号 --password 密码 --ip 172.x.x.x

# 下线（用户名）
# python bsu_login.py --logout --username 账号

# 下线（IP）
# python bsu_login.py --logout --ip 172.x.x.x
#
N = "200"
TYPE = "1"
ENC = "srun_bx1"
BASE64_ALPHA = "LVoJPiCN2R8G90yg+hmFHuacZ1OWMnrsSTXkYpUq/3dlbfKwv6xztjI7DeBE45QA"
HOST = "http://219.242.208.131"


def _s(a: str, include_length: bool):
    """将字符串按小端序打包为 32 位整数数组，供 xEncode 使用。"""
    v = []
    for i in range(0, len(a), 4):
        c0 = ord(a[i]) if i < len(a) else 0
        c1 = ord(a[i + 1]) if i + 1 < len(a) else 0
        c2 = ord(a[i + 2]) if i + 2 < len(a) else 0
        c3 = ord(a[i + 3]) if i + 3 < len(a) else 0
        v.append(c0 | (c1 << 8) | (c2 << 16) | (c3 << 24))
    if include_length:
        v.append(len(a))
    return v


def _l(v, include_length: bool):
    """将 32 位整数数组还原为字符串，供 xEncode 结果拼接使用。"""
    out = []
    for x in v:
        out.append(chr(x & 0xFF))
        out.append(chr((x >> 8) & 0xFF))
        out.append(chr((x >> 16) & 0xFF))
        out.append(chr((x >> 24) & 0xFF))
    s = "".join(out)
    if include_length:
        return s[: v[-1]]
    return s


def xencode(s: str, key: str) -> str:
    """使用 SRun 的 xEncode 算法对原始字符串进行编码。"""
    if not s:
        return ""
    v = _s(s, True)
    k = _s(key, False)
    while len(k) < 4:
        k.append(0)

    n = len(v) - 1
    z = v[n]
    c = (0x86014019 | 0x183639A0) & 0xFFFFFFFF
    q = int(6 + 52 / (n + 1))
    d = 0

    while q > 0:
        d = (d + c) & (0x8CE0D9BF | 0x731F2640)
        e = (d >> 2) & 3
        for p in range(0, n):
            y = v[p + 1]
            m = ((z >> 5) ^ (y << 2)) & 0xFFFFFFFF
            m = (m + (((y >> 3) ^ (z << 4)) ^ (d ^ y))) & 0xFFFFFFFF
            m = (m + (k[(p & 3) ^ e] ^ z)) & 0xFFFFFFFF
            v[p] = (v[p] + m) & (0xEFB8D130 | 0x10472ECF)
            z = v[p]
        y = v[0]
        m = ((z >> 5) ^ (y << 2)) & 0xFFFFFFFF
        m = (m + (((y >> 3) ^ (z << 4)) ^ (d ^ y))) & 0xFFFFFFFF
        m = (m + (k[(n & 3) ^ e] ^ z)) & 0xFFFFFFFF
        v[n] = (v[n] + m) & (0xBB390742 | 0x44C6F8BD)
        z = v[n]
        q -= 1

    return _l(v, False)


def js_b64_encode(raw: str) -> str:
    """使用门户自定义 Base64 字典编码字符串。"""
    b = raw.encode("latin1")
    pad = "="
    out = []
    i = 0
    while i + 3 <= len(b):
        b10 = (b[i] << 16) | (b[i + 1] << 8) | b[i + 2]
        out.append(BASE64_ALPHA[(b10 >> 18) & 0x3F])
        out.append(BASE64_ALPHA[(b10 >> 12) & 0x3F])
        out.append(BASE64_ALPHA[(b10 >> 6) & 0x3F])
        out.append(BASE64_ALPHA[b10 & 0x3F])
        i += 3
    rem = len(b) - i
    if rem == 1:
        b10 = b[i] << 16
        out.append(BASE64_ALPHA[(b10 >> 18) & 0x3F])
        out.append(BASE64_ALPHA[(b10 >> 12) & 0x3F])
        out.append(pad)
        out.append(pad)
    elif rem == 2:
        b10 = (b[i] << 16) | (b[i + 1] << 8)
        out.append(BASE64_ALPHA[(b10 >> 18) & 0x3F])
        out.append(BASE64_ALPHA[(b10 >> 12) & 0x3F])
        out.append(BASE64_ALPHA[(b10 >> 6) & 0x3F])
        out.append(pad)
    return "".join(out)


def parse_jsonp(text: str):
    """从 JSONP 响应文本中提取 JSON 对象。"""
    text = text.strip()
    m = re.match(r"^[^(]+\((.*)\)$", text)
    if not m:
        raise ValueError(f"unexpected response: {text[:200]}")
    return json.loads(m.group(1))


def http_get(url: str, params: dict):
    """发送带查询参数的 GET 请求并返回文本响应。"""
    qs = urllib.parse.urlencode(params)
    req = urllib.request.Request(url + "?" + qs, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=10) as resp:
        return resp.read().decode("utf-8", errors="replace")


def get_challenge(host: str, username: str, ip: str):
    """向 SRun 认证服务请求 challenge token。"""
    callback = f"cb_{int(time.time() * 1000)}"
    text = http_get(
        f"{host}/cgi-bin/get_challenge",
        {
            "callback": callback,
            "username": username,
            "ip": ip,
            "_": int(time.time() * 1000),
        },
    )
    data = parse_jsonp(text)
    if data.get("error") != "ok":
        raise RuntimeError(f"get_challenge failed: {data}")
    return data


def build_login_params(username: str, password: str, ac_id: str, ip: str, token: str):
    """构造 /cgi-bin/srun_portal 登录所需的签名参数。"""
    info_obj = {
        "username": username,
        "password": password,
        "ip": ip,
        "acid": ac_id,
        "enc_ver": ENC,
    }
    info = "{SRBX1}" + js_b64_encode(
        xencode(json.dumps(info_obj, separators=(",", ":")), token)
    )

    hmd5 = hmac.new(token.encode(), password.encode(), hashlib.md5).hexdigest()

    chkstr = (
        token
        + username
        + token
        + hmd5
        + token
        + ac_id
        + token
        + ip
        + token
        + N
        + token
        + TYPE
        + token
        + info
    )
    chksum = hashlib.sha1(chkstr.encode()).hexdigest()

    return {
        "action": "login",
        "username": username,
        "password": "{MD5}" + hmd5,
        "ac_id": ac_id,
        "ip": ip,
        "chksum": chksum,
        "info": info,
        "n": N,
        "type": TYPE,
        "os": "Linux",
        "name": "Linux",
        "double_stack": "0",
    }


def login(host: str, username: str, password: str, ac_id: str, ip: str):
    """执行完整登录流程：获取 challenge、生成签名并发起登录。"""
    challenge_data = get_challenge(host, username, ip)
    token = challenge_data["challenge"]
    real_ip = challenge_data.get("client_ip") or ip
    params = build_login_params(username, password, ac_id, real_ip, token)

    callback = f"cb_{int(time.time() * 1000)}"
    params["callback"] = callback
    params["_"] = int(time.time() * 1000)
    text = http_get(f"{host}/cgi-bin/srun_portal", params)
    return parse_jsonp(text)


def logout(host: str, ac_id: str, ip: str, username: str):
    """发起下线请求，可按 username 或 ip 指定目标会话。"""
    callback = f"cb_{int(time.time() * 1000)}"
    params = {
        "callback": callback,
        "action": "logout",
        "ac_id": ac_id,
        "_": int(time.time() * 1000),
    }
    if ip:
        params["ip"] = ip
    if username:
        params["username"] = username
    text = http_get(f"{host}/cgi-bin/srun_portal", params)
    return parse_jsonp(text)


def main():
    """命令行入口：处理登录与下线参数并执行对应操作。"""
    p = argparse.ArgumentParser(description="Srun campus network login script")
    p.add_argument("--ac-id", default="2")
    p.add_argument("--ip", default="")
    p.add_argument("--username", default="")
    p.add_argument("--password", default="")
    p.add_argument("--logout", action="store_true")
    args = p.parse_args()

    if args.logout:
        if not args.ip and not args.username:
            raise SystemExit("--logout requires --username or --ip")
        print(
            json.dumps(
                logout(HOST, args.ac_id, args.ip, args.username),
                ensure_ascii=False,
                indent=2,
            )
        )
        return

    if not args.username or not args.password:
        raise SystemExit("login requires --username and --password")

    print(
        json.dumps(
            login(HOST, args.username, args.password, args.ac_id, args.ip),
            ensure_ascii=False,
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
