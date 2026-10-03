/** 为顶栏与展开面板生成额度状态文案，不展示后端错误正文。 */

/**
 * 压缩顶栏中的非成功状态。
 * @param {string} status 采集状态码。
 * @returns {string}
 */
function statusText(status) {
    switch (status) {
    case "loading": return "…";
    case "missing_credentials": return "未配置";
    case "auth_required": return "凭据失效";
    case "no_weekly_limit": return "无周额度";
    default: return "不可用";
    }
}

/**
 * 根据提供商与状态说明下一步操作。
 * @param {string} provider 提供商标识 codex 或 deepseek。
 * @param {string} status 采集状态码。
 * @returns {string}
 */
function statusDetails(provider, status) {
    switch (status) {
    case "loading": return "正在读取…";
    case "missing_credentials": return provider === "codex" ? "请先使用 codex login 登录订阅账号" : "请为桌面进程配置 DEEPSEEK_API_KEY";
    case "auth_required": return provider === "codex" ? "请使用 codex login 更新登录凭据" : "请检查 DEEPSEEK_API_KEY 是否有效";
    case "no_weekly_limit": return "接口未返回标准 Codex 周额度";
    case "network_error": return "网络连接失败，下一分钟自动重试";
    case "rate_limited": return "请求受限，下一分钟自动重试";
    case "invalid_response": return "暂时无法读取额度数据";
    default: return "服务暂不可用，下一分钟自动重试";
    }
}

/**
 * 按分钟向上取整显示剩余时长；过期后显示等待刷新，不产生负倒计时。
 * @param {number} resetAt 重置时刻，Unix 秒。
 * @param {number} now 当前时刻，Unix 毫秒。
 * @returns {string}
 */
function resetIn(resetAt, now) {
    const minutesLeft = Math.ceil((resetAt * 1000 - now) / 60000);
    if (minutesLeft <= 0)
        return "reset due";
    const days = Math.floor(minutesLeft / 1440);
    const hours = Math.floor(minutesLeft % 1440 / 60);
    const minutes = minutesLeft % 60;
    return "reset in " + (days > 0 ? days + "d " : "")
        + (days > 0 || hours > 0 ? hours + "h " : "") + minutes + "m";
}
