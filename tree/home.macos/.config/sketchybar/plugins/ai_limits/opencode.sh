#!/bin/sh
# 使用 OPENCODE_API_KEY 直接获取 OpenCode Go 的 5h、weekly 和 monthly 额度。
# Usage: opencode.sh   （stdout 输出供 item.sh 消费的单行 JSON）
# 依赖：curl, jq
# 读取环境变量：OPENCODE_API_KEY
# 写入环境变量：无

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
. "$SCRIPT_DIR/common.sh"

if [ -z "${OPENCODE_API_KEY:-}" ]; then
  emit_status "auth_required" "Auth required"
  exit 0
fi

if ! http_get_bearer_json \
  "https://opencode.ai/zen/go/v1/usage" \
  "$OPENCODE_API_KEY" \
  "" \
  "sketchybar-ai-limits"; then
  emit_status "unavailable" "Network unavailable"
  exit 0
fi

case "$HTTP_STATUS" in
  200) ;;
  401)
    emit_status "auth_required" "Auth required"
    exit 0
    ;;
  403)
    emit_status "plan_required" "Go plan required"
    exit 0
    ;;
  *)
    emit_status "unavailable" "Usage unavailable"
    exit 0
    ;;
esac

OUTPUT="$(printf '%s\n' "$HTTP_BODY" | jq -c '
  def clamp_percent:
    (tonumber? // 0)
    | if . < 0 then 0 elif . > 100 then 100 else round end;

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

  def usage_row($name; $row):
    ($row.percent | clamp_percent) as $used
    | {
        used: $used,
        text: "\($name) · \($used)% used · \(reset_text($row.resetsAt))"
      };

  ([
    usage_row("5h"; .usage.rolling),
    usage_row("Weekly"; .usage.weekly),
    usage_row("Monthly"; .usage.monthly)
  ]) as $rows
  | if ($rows | length) != 3 then error("missing usage windows")
    else (.usage.weekly.percent | clamp_percent) as $weekly_used
    | (.usage.rolling.percent | clamp_percent) as $five_hour_used
    | {
        status: "ok",
        main: "W \(100 - $weekly_used)% 5h \(100 - $five_hour_used)%",
        details: ($rows | map(.text))
      }
    end
' 2>/dev/null)"

if [ -z "$OUTPUT" ]; then
  emit_status "unavailable" "Invalid usage response"
  exit 0
fi

printf '%s\n' "$OUTPUT"
