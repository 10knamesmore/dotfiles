#!/bin/sh
# 按 Kimi Code 的 ensureFresh 语义读取或刷新 OAuth 凭据，供额度采集器复用。
# Usage: . "$SCRIPT_DIR/kimi_oauth.sh"; kimi_ensure_fresh_access_token
# 依赖：curl, jq
# 读取环境变量：KIMI_CODE_HOME（缺省为 ~/.kimi-code）、KIMI_CODE_OAUTH_HOST、KIMI_OAUTH_HOST
# 写入环境变量：无；刷新成功时原子替换 Kimi Code OAuth 凭据文件

KIMI_HOME="${KIMI_CODE_HOME:-$HOME/.kimi-code}"
KIMI_AUTH_FILE="$KIMI_HOME/credentials/kimi-code.json"
KIMI_OAUTH_HOST_VALUE="${KIMI_CODE_OAUTH_HOST:-${KIMI_OAUTH_HOST:-https://auth.kimi.com}}"
KIMI_OAUTH_HOST_VALUE="${KIMI_OAUTH_HOST_VALUE%/}"
KIMI_OAUTH_CLIENT_ID="17e5f671-d194-4dfb-9706-5516cb48c098"
KIMI_OAUTH_LOCK_TARGET="$KIMI_HOME/oauth/kimi-code"
KIMI_OAUTH_LOCK_DIR="$KIMI_OAUTH_LOCK_TARGET.lock"

# 凭据临时文件与 lock 只允许当前用户读写，避免 refresh token 泄露给同机其他用户。
umask 077

_kimi_read_credentials() {
  local raw

  raw="$(jq -c 'select(type == "object")' "$KIMI_AUTH_FILE" 2>/dev/null)"
  if [ -z "$raw" ]; then
    return 1
  fi

  ACCESS_TOKEN="$(printf '%s\n' "$raw" | jq -r '.access_token // empty')"
  KIMI_REFRESH_TOKEN="$(printf '%s\n' "$raw" | jq -r '.refresh_token // empty')"
  KIMI_EXPIRES_AT="$(printf '%s\n' "$raw" | jq -r '.expires_at // 0')"
  KIMI_EXPIRES_IN="$(printf '%s\n' "$raw" | jq -r '.expires_in // 0')"
  case "$KIMI_EXPIRES_AT" in
    ''|*[!0-9]*) KIMI_EXPIRES_AT=0 ;;
  esac
  case "$KIMI_EXPIRES_IN" in
    ''|*[!0-9]*) KIMI_EXPIRES_IN=0 ;;
  esac
  return 0
}

# 上游在剩余寿命低于 max(5 分钟, 原始 TTL 的一半) 时刷新，避免用即将过期的 token 发起额度请求。
_kimi_token_needs_refresh() {
  local threshold=300
  local half_life

  if [ -z "$ACCESS_TOKEN" ]; then
    return 0
  fi
  if [ "$KIMI_EXPIRES_IN" -gt 0 ]; then
    half_life=$((KIMI_EXPIRES_IN / 2))
    if [ "$half_life" -gt "$threshold" ]; then
      threshold="$half_life"
    fi
  fi
  [ "$KIMI_EXPIRES_AT" -le "$(($(date +%s) + threshold))" ]
}

_kimi_release_refresh_lock() {
  if [ -n "${KIMI_LOCK_HEARTBEAT_PID:-}" ]; then
    kill "$KIMI_LOCK_HEARTBEAT_PID" 2>/dev/null || true
    wait "$KIMI_LOCK_HEARTBEAT_PID" 2>/dev/null || true
    KIMI_LOCK_HEARTBEAT_PID=""
  fi
  if [ "${KIMI_LOCK_HELD:-0}" -eq 1 ]; then
    rmdir "$KIMI_OAUTH_LOCK_DIR" 2>/dev/null || true
    KIMI_LOCK_HELD=0
  fi
}

# 与 Kimi Code 上游共用 oauth/kimi-code.lock 目录，避免两个进程同时消费会轮换的 refresh token。
_kimi_acquire_refresh_lock() {
  local attempt=0
  local lock_mtime
  local lock_age

  mkdir -p "$(dirname "$KIMI_OAUTH_LOCK_TARGET")" || return 1
  chmod 700 "$(dirname "$KIMI_OAUTH_LOCK_TARGET")" 2>/dev/null || true
  : > "$KIMI_OAUTH_LOCK_TARGET" || return 1
  chmod 600 "$KIMI_OAUTH_LOCK_TARGET" 2>/dev/null || true

  while ! mkdir "$KIMI_OAUTH_LOCK_DIR" 2>/dev/null; do
    lock_mtime="$(stat -f '%m' "$KIMI_OAUTH_LOCK_DIR" 2>/dev/null || printf '0')"
    lock_age="$(($(date +%s) - lock_mtime))"
    # proper-lockfile 上游 5 秒判 stale；这里用 10 秒避免误删正在 heartbeat 的 lock。
    if [ "$lock_mtime" -gt 0 ] && [ "$lock_age" -gt 10 ]; then
      rmdir "$KIMI_OAUTH_LOCK_DIR" 2>/dev/null || true
      continue
    fi
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 20 ]; then
      return 1
    fi
    sleep 0.5
  done

  KIMI_LOCK_HELD=1
  # refresh 可跨越多次 HTTP 重试；持续更新 mtime 防止上游把活跃 lock 当成 stale。
  (
    while [ -d "$KIMI_OAUTH_LOCK_DIR" ]; do
      touch "$KIMI_OAUTH_LOCK_DIR" 2>/dev/null || exit 0
      sleep 1
    done
  ) &
  KIMI_LOCK_HEARTBEAT_PID=$!
  # 无论正常退出还是中途失败，都释放精确的 OAuth lock 目录。
  trap '_kimi_release_refresh_lock' EXIT
  return 0
}

# 同目录临时文件加 rename 保证读者只会看到旧凭据或完整新凭据。
_kimi_persist_token_json() {
  local temporary

  temporary="$(mktemp "$KIMI_AUTH_FILE.tmp.XXXXXX")" || return 1
  if ! printf '%s\n' "$KIMI_TOKEN_JSON" > "$temporary"; then
    rm -f "$temporary"
    return 1
  fi
  chmod 600 "$temporary" 2>/dev/null || true
  if ! mv -f "$temporary" "$KIMI_AUTH_FILE"; then
    rm -f "$temporary"
    return 1
  fi
  return 0
}

_kimi_save_refreshed_token() {
  KIMI_TOKEN_JSON="$(printf '%s\n' "$KIMI_REFRESH_BODY" | jq --argjson now "$(date +%s)" '
    (.expires_in | tonumber?) as $expires_in
    | select(
        (.access_token | type) == "string" and (.access_token | length) > 0
        and (.refresh_token | type) == "string" and (.refresh_token | length) > 0
        and $expires_in != null and $expires_in > 0
      )
    | {
        access_token,
        refresh_token,
        expires_at: ($now + ($expires_in | floor)),
        scope: (if (.scope | type) == "string" then .scope else "" end),
        token_type: (if (.token_type | type) == "string" then .token_type else "Bearer" end),
        expires_in: ($expires_in | floor)
      }
  ' 2>/dev/null)"
  [ -n "$KIMI_TOKEN_JSON" ] && _kimi_persist_token_json
}

_kimi_save_revoked_token() {
  KIMI_TOKEN_JSON="$(jq '
    {
      access_token: "",
      refresh_token: "",
      expires_at: 0,
      scope: (if (.scope | type) == "string" then .scope else "" end),
      token_type: (if (.token_type | type) == "string" then .token_type else "Bearer" end),
      expires_in: 0
    }
  ' "$KIMI_AUTH_FILE" 2>/dev/null)"
  [ -n "$KIMI_TOKEN_JSON" ] && _kimi_persist_token_json
}

# 只对传输错误、429 和 5xx 做最多三次重试；401/403/invalid_grant 立即交给登录状态处理。
_kimi_refresh_access_token() {
  local attempt=1
  local form_body
  local response
  local curl_exit
  local refresh_status
  local refresh_error

  # refresh token 从 stdin 传给 jq，避免敏感值出现在进程参数列表。
  form_body="$(printf '%s' "$KIMI_REFRESH_TOKEN" | jq -Rrs \
    --arg client_id "$KIMI_OAUTH_CLIENT_ID" \
    '"client_id=" + ($client_id | @uri)
      + "&grant_type=refresh_token&refresh_token=" + (. | @uri)')"

  while [ "$attempt" -le 3 ]; do
    response="$(printf '%s' "$form_body" | curl \
      --silent \
      --show-error \
      --max-time 10 \
      --request POST \
      --header 'Content-Type: application/x-www-form-urlencoded' \
      --header 'Accept: application/json' \
      --user-agent 'sketchybar-ai-limits' \
      --data-binary @- \
      --write-out '\n%{http_code}' \
      "$KIMI_OAUTH_HOST_VALUE/api/oauth/token" 2>/dev/null)"
    curl_exit=$?
    if [ "$curl_exit" -ne 0 ]; then
      if [ "$attempt" -lt 3 ]; then
        sleep "$attempt"
        attempt=$((attempt + 1))
        continue
      fi
      return 1
    fi

    refresh_status="$(printf '%s\n' "$response" | tail -n 1)"
    KIMI_REFRESH_BODY="$(printf '%s\n' "$response" | sed '$d')"
    if [ "$refresh_status" = 200 ]; then
      _kimi_save_refreshed_token || return 3
      _kimi_read_credentials || return 3
      return 0
    fi

    refresh_error="$(printf '%s\n' "$KIMI_REFRESH_BODY" | jq -r '.error // empty' 2>/dev/null)"
    case "$refresh_status:$refresh_error" in
      401:*|403:*|*:invalid_grant)
        return 2
        ;;
      429:*|500:*|502:*|503:*|504:*)
        if [ "$attempt" -lt 3 ]; then
          sleep "$attempt"
          attempt=$((attempt + 1))
          continue
        fi
        return 1
        ;;
      *)
        return 3
        ;;
    esac
  done
  return 1
}

# @description 返回可用的 Kimi access token；近期过期时在跨进程 lock 内刷新并写回与上游相同的凭据文件。
# @set ACCESS_TOKEN string 可直接用于 managed usage 请求的 Bearer token
# @stdout 无
# @exitcode 0 获得可用 token
# @exitcode 1 凭据缺失、refresh token 被拒绝或需要重新登录
# @exitcode 2 lock、网络、服务端或凭据写入暂时不可用
kimi_ensure_fresh_access_token() {
  local refresh_exit
  local refresh_token_used

  if [ ! -r "$KIMI_AUTH_FILE" ] || ! _kimi_read_credentials || [ -z "$KIMI_REFRESH_TOKEN" ]; then
    return 1
  fi
  if ! _kimi_token_needs_refresh; then
    return 0
  fi
  if ! _kimi_acquire_refresh_lock; then
    return 2
  fi

  # 拿到跨进程 lock 后必须重读；Kimi Code 可能已在等待期间轮换了 refresh token。
  if ! _kimi_read_credentials || [ -z "$KIMI_REFRESH_TOKEN" ]; then
    _kimi_release_refresh_lock
    return 1
  fi
  if ! _kimi_token_needs_refresh; then
    _kimi_release_refresh_lock
    return 0
  fi

  refresh_token_used="$KIMI_REFRESH_TOKEN"
  if _kimi_refresh_access_token; then
    _kimi_release_refresh_lock
    return 0
  fi
  refresh_exit=$?

  if [ "$refresh_exit" -eq 2 ]; then
    # 拒绝可能来自旧 token 竞态；只有凭据未被同行转动时才按上游语义写入 revoked tombstone。
    _kimi_read_credentials || true
    if [ -n "$ACCESS_TOKEN" ] && [ "$KIMI_REFRESH_TOKEN" != "$refresh_token_used" ]; then
      _kimi_release_refresh_lock
      return 0
    fi
    _kimi_save_revoked_token || true
    _kimi_release_refresh_lock
    return 1
  fi

  _kimi_release_refresh_lock
  return 2
}
