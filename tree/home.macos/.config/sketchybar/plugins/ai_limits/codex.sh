#!/bin/sh
# 读取现有 Codex ChatGPT 登录凭据，并直接获取所有远端额度窗口。
# Usage: codex.sh   （stdout 输出供 item.sh 消费的单行 JSON）
# 依赖：curl, jq
# 读取环境变量：CODEX_HOME（缺省为 ~/.codex）
# 写入环境变量：无；不会刷新或改写 Codex 凭据

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
. "$SCRIPT_DIR/common.sh"

AUTH_FILE="${CODEX_HOME:-$HOME/.codex}/auth.json"
if [ ! -r "$AUTH_FILE" ]; then
  emit_status "auth_required" "Auth required"
  exit 0
fi

ACCESS_TOKEN="$(jq -r 'select(.auth_mode == "chatgpt") | .tokens.access_token // empty' "$AUTH_FILE" 2>/dev/null)"
ACCOUNT_ID="$(jq -r 'select(.auth_mode == "chatgpt") | .tokens.account_id // empty' "$AUTH_FILE" 2>/dev/null)"
if [ -z "$ACCESS_TOKEN" ] || [ -z "$ACCOUNT_ID" ]; then
  emit_status "auth_required" "Auth required"
  exit 0
fi

if ! http_get_bearer_json \
  "https://chatgpt.com/backend-api/wham/usage" \
  "$ACCESS_TOKEN" \
  "$ACCOUNT_ID" \
  "codex-cli"; then
  emit_status "unavailable" "Network unavailable"
  exit 0
fi

case "$HTTP_STATUS" in
  200) ;;
  401|403)
    emit_status "auth_required" "Auth required"
    exit 0
    ;;
  *)
    emit_status "unavailable" "Usage unavailable"
    exit 0
    ;;
esac

# 主标签和 popup 都只按固定顺序显示标准 Codex 的 weekly 和 5h 窗口。
OUTPUT="$(printf '%s\n' "$HTTP_BODY" | jq -c '
  def clamp_percent:
    (tonumber? // 0)
    | if . < 0 then 0 elif . > 100 then 100 else round end;

  def window_name($seconds):
    ($seconds | tonumber? // 0) as $s
    | if $s <= 0 then "Window"
      elif ($s % 604800) == 0 then "\($s / 604800 | floor)w"
      elif ($s % 86400) == 0 then "\($s / 86400 | floor)d"
      elif ($s % 3600) == 0 then "\($s / 3600 | floor)h"
      else "\($s / 60 | floor)m"
      end;

  def reset_text($epoch):
    ($epoch | tonumber?) as $at
    | if $at == null then "reset unknown"
      else (($at - now) | floor) as $left
      | if $left <= 0 then "reset due"
        elif $left >= 86400 then "resets in \($left / 86400 | floor)d \(($left % 86400) / 3600 | floor)h \(($left % 3600) / 60 | floor)m"
        elif $left >= 3600 then "resets in \($left / 3600 | floor)h \(($left % 3600) / 60 | floor)m"
        else "resets in \($left / 60 | floor)m"
        end
      end;

  def usage_row($name; $window):
    if $window == null then empty
    else ($window.used_percent | clamp_percent) as $used
    | {
        used: $used,
        text: "\($name) · \(window_name($window.limit_window_seconds)) · \($used)% used · \(reset_text($window.reset_at))"
      }
    end;

  def standard_window($rate_limit; $seconds):
    (first(
      [$rate_limit.primary_window, $rate_limit.secondary_window][]
      | select(. != null and ((.limit_window_seconds | tonumber?) == $seconds))
    ) // null);

  def main_window($rate_limit; $seconds; $label):
    (standard_window($rate_limit; $seconds)) as $window
    | if $window == null then null
      else ($window.used_percent | clamp_percent) as $used
      | "\($label) \(100 - $used)%"
      end;

  ([
    usage_row("Codex"; standard_window(.rate_limit; 604800)),
    usage_row("Codex"; standard_window(.rate_limit; 18000))
  ]) as $rows
  | ([
      main_window(.rate_limit; 604800; "W"),
      main_window(.rate_limit; 18000; "5h")
    ] | map(select(. != null))) as $main_windows
  | if ($rows | length) == 0 then error("missing rate-limit windows")
    elif ($main_windows | length) == 0 then error("missing standard Codex windows")
    else {
        status: "ok",
        main: ($main_windows | join(" ")),
        details: ($rows | map(.text))
      }
    end
' 2>/dev/null)"

if [ -z "$OUTPUT" ]; then
  emit_status "unavailable" "Invalid usage response"
  exit 0
fi

printf '%s\n' "$OUTPUT"
