#!/bin/sh
# 复用 Kimi Code OAuth 凭据获取计划额度，并按上游 ensureFresh 规则按需刷新短期 access token。
# Usage: kimi.sh   （stdout 输出供 item.sh 消费的单行 JSON）
# 依赖：curl, jq
# 读取环境变量：KIMI_CODE_HOME（缺省为 ~/.kimi-code）、KIMI_CODE_OAUTH_HOST、KIMI_OAUTH_HOST
# 写入环境变量：无；刷新成功时会原子替换 Kimi Code 的 OAuth 凭据文件

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
. "$SCRIPT_DIR/common.sh"
. "$SCRIPT_DIR/kimi_oauth.sh"

if kimi_ensure_fresh_access_token; then
  :
else
  kimi_auth_exit=$?
  if [ "$kimi_auth_exit" -eq 1 ]; then
    emit_status "auth_required" "Auth required"
  else
    emit_status "unavailable" "OAuth refresh unavailable"
  fi
  exit 0
fi

AUTH_FILE="${KIMI_CODE_HOME:-$HOME/.kimi-code}/credentials/kimi-code.json"
if [ ! -r "$AUTH_FILE" ]; then
  emit_status "auth_required" "Auth required"
  exit 0
fi

ACCESS_TOKEN="$(jq -r '.access_token // empty' "$AUTH_FILE" 2>/dev/null)"
EXPIRES_AT="$(jq -r '.expires_at // 0' "$AUTH_FILE" 2>/dev/null)"
case "$EXPIRES_AT" in
  ''|*[!0-9]*) EXPIRES_AT=0 ;;
esac

# ensureFresh 返回后仍校验落盘结果，避免损坏或空凭据被当成可用 token。
if [ -z "$ACCESS_TOKEN" ] || [ "$EXPIRES_AT" -le "$(date +%s)" ]; then
  emit_status "auth_required" "Auth required"
  exit 0
fi

if ! http_get_bearer_json \
  "https://api.kimi.com/coding/v1/usages" \
  "$ACCESS_TOKEN" \
  "" \
  "sketchybar-ai-limits"; then
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

# Kimi 的 summary 表示周额度，limits 数组保留服务端声明的短窗口。
OUTPUT="$(printf '%s\n' "$HTTP_BODY" | jq -c '
  def number: tonumber? // 0;

  def clamp_percent:
    if . < 0 then 0 elif . > 100 then 100 else round end;

  def reset_epoch($value):
    if ($value | type) != "string" then null
    else ($value | sub("\\.[0-9]+Z$"; "Z") | fromdateiso8601?)
    end;

  def reset_text($value):
    (reset_epoch($value)) as $at
    | if $at == null then "reset unknown"
      else (($at - now) | floor) as $left
      | if $left <= 0 then "reset due"
        elif $left >= 86400 then "resets in \($left / 86400 | floor)d \(($left % 86400) / 3600 | floor)h \(($left % 3600) / 60 | floor)m"
        elif $left >= 3600 then "resets in \($left / 3600 | floor)h \(($left % 3600) / 60 | floor)m"
        else "resets in \($left / 60 | floor)m"
        end
      end;

  def limit_name($window):
    ($window.duration | number) as $duration
    | ($window.timeUnit // "") as $unit
    | if $unit == "TIME_UNIT_MINUTE" and $duration >= 60 and ($duration % 60) == 0 then "\($duration / 60 | floor)h"
      elif $unit == "TIME_UNIT_MINUTE" then "\($duration)m"
      elif $unit == "TIME_UNIT_HOUR" then "\($duration)h"
      elif $unit == "TIME_UNIT_DAY" then "\($duration)d"
      elif $unit == "TIME_UNIT_WEEK" then "\($duration)w"
      else "Limit"
      end;

  def usage_row($name; $row):
    ($row.limit | number) as $limit
    | ($row.used | number) as $used_value
    | if $limit <= 0 then empty
      else (($used_value / $limit * 100) | clamp_percent) as $used
      | {
          used: $used,
          text: "\($name) · \($used)% used · \(reset_text($row.resetTime))"
      }
      end;

  def remaining_label($label; $row):
    ($row.limit | number) as $limit
    | ($row.used | number) as $used_value
    | if $limit <= 0 then null
      else (($used_value / $limit * 100) | clamp_percent) as $used
      | "\($label) \(100 - $used)%"
      end;

  (first(
    .limits[]?
    | select(
        (.window.duration | number) == 300
        and .window.timeUnit == "TIME_UNIT_MINUTE"
      )
    | .detail
  ) // null) as $five_hour
  |
  ([
    usage_row("Weekly"; .usage),
    (.limits[]? as $limit | usage_row(limit_name($limit.window); $limit.detail))
  ]) as $rows
  | ([
      remaining_label("W"; .usage),
      remaining_label("5h"; $five_hour)
    ] | map(select(. != null))) as $main_windows
  | if ($rows | length) == 0 then error("missing usage windows")
    elif ($main_windows | length) == 0 then error("missing standard Kimi windows")
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
