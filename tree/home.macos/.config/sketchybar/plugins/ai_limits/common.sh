#!/bin/sh
# 为 AI 额度采集器提供只读 Bearer HTTP 请求和统一错误结果。
# Usage: . "$SCRIPT_DIR/common.sh"
# 依赖：curl, jq
# 读取环境变量：无
# 写入环境变量：无；HTTP_BODY 和 HTTP_STATUS 仅在当前 shell 中更新

# @description 发送一次只读 GET 请求；非 2xx 仍返回响应，网络或 TLS 失败返回 1。
# @arg $1 string 完整请求 URL
# @arg $2 string Bearer token
# @arg $3 string 可选的 ChatGPT account ID；仅 Codex 请求需要
# @arg $4 string 可选的 User-Agent
# @set HTTP_BODY string 响应体；传输失败时为空
# @set HTTP_STATUS string 三位 HTTP 状态码；传输失败时为 000
# @stdout 无
# @exitcode 0 服务端返回了 HTTP 响应
# @exitcode 1 请求未到达可返回 HTTP 响应的阶段
http_get_bearer_json() {
  local url="$1"
  local token="$2"
  local account_id="$3"
  local user_agent="$4"
  local response
  local curl_exit

  set -- --silent --show-error --max-time 10 \
    --request GET \
    --header "Authorization: Bearer $token" \
    --header "Accept: application/json"

  if [ -n "$account_id" ]; then
    set -- "$@" --header "ChatGPT-Account-Id: $account_id"
  fi
  if [ -n "$user_agent" ]; then
    set -- "$@" --user-agent "$user_agent"
  fi

  response="$(curl "$@" --write-out '\n%{http_code}' "$url" 2>/dev/null)"
  curl_exit=$?
  if [ "$curl_exit" -ne 0 ]; then
    HTTP_BODY=""
    HTTP_STATUS="000"
    return 1
  fi

  # curl 把状态码追加为最后一行，响应体因此可以保持原始 JSON 结构。
  HTTP_STATUS="$(printf '%s\n' "$response" | tail -n 1)"
  HTTP_BODY="$(printf '%s\n' "$response" | sed '$d')"
  return 0
}

# @description 输出采集失败的稳定 JSON；不透传可能包含账户信息的服务端错误正文。
# @arg $1 string 状态标识，供展示层选择固定文案
# @arg $2 string 面向用户的简短错误说明
# @stdout 单行 JSON，字段为 status、message、main 和 details
# @exitcode 0 始终成功输出
emit_status() {
  jq -cn \
    --arg status "$1" \
    --arg message "$2" \
    '{status: $status, message: $message, main: "--", details: []}'
}
