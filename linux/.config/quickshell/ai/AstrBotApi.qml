import QtQuick
import Quickshell.Io

// AstrBot HTTP API 封装 — 全部通过 curl Process 调用
Item {
    id: api

    property string server: ""    // "http://x.x.x.x:6185"
    property string apiKey: ""

    // ── 信号 ──
    signal chatSessionId(string sessionId)   // 服务端分配的会话 ID
    signal chatChunk(string content)
    signal chatToolCall(string name, string args)
    signal chatToolResult(string name, string result)
    signal chatComplete(string fullContent)
    signal chatStats(var stats)    // {token_usage, start_time, end_time, time_to_first_token}
    signal chatError(string message)
    signal configsLoaded(var configs)
    signal sessionsLoaded(var sessions)
    signal botsLoaded(var botIds)
    signal fileUploaded(string attachmentId, string filename)
    signal imMessageSent(bool success)

    // ── Chat SSE 流式 ──
    function sendChat(message, sessionId, configId, username) {
        if (chatProc.running)
            return;
        let body = {
            message: message,
            username: username || "qml-user",
            enable_streaming: true
        };
        if (sessionId)
            body.session_id = sessionId;
        if (configId)
            body.config_id = configId;
        chatProc._err = "";
        chatProc._fullContent = "";
        chatProc._gotData = false;
        let cmd = [
            "curl", "-N", "-s", "--no-buffer", "--max-time", "120",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-H", "X-API-Key: " + api.apiKey,
            "-d", JSON.stringify(body),
            api.server + "/api/v1/chat"
        ];
        chatProc.command = cmd;
        chatProc.running = true;
    }

    function abortChat() {
        chatProc.running = false;
    }

    // ── Configs ──
    function fetchConfigs() {
        if (configsProc.running)
            return;
        configsProc._buf = "";
        configsProc.command = [
            "curl", "-s", "--max-time", "10",
            "-H", "X-API-Key: " + api.apiKey,
            api.server + "/api/v1/configs"
        ];
        configsProc.running = true;
    }

    // ── Sessions ──
    function fetchSessions(username, page, pageSize) {
        if (sessionsProc.running)
            return;
        sessionsProc._buf = "";
        let url = api.server + "/api/v1/chat/sessions?username=" + encodeURIComponent(username || "qml-user");
        if (page)
            url += "&page=" + page;
        if (pageSize)
            url += "&page_size=" + pageSize;
        sessionsProc.command = [
            "curl", "-s", "--max-time", "10",
            "-H", "X-API-Key: " + api.apiKey,
            url
        ];
        sessionsProc.running = true;
    }

    // ── Bots ──
    function fetchBots() {
        if (botsProc.running)
            return;
        botsProc._buf = "";
        botsProc.command = [
            "curl", "-s", "--max-time", "10",
            "-H", "X-API-Key: " + api.apiKey,
            api.server + "/api/v1/im/bots"
        ];
        botsProc.running = true;
    }

    // ── File Upload ──
    function uploadFile(filePath) {
        if (uploadProc.running)
            return;
        uploadProc._buf = "";
        uploadProc.command = [
            "curl", "-s", "--max-time", "30",
            "-X", "POST",
            "-H", "X-API-Key: " + api.apiKey,
            "-F", "file=@" + filePath,
            api.server + "/api/v1/file"
        ];
        uploadProc.running = true;
    }

    // ── File Download ──
    function downloadFile(attachmentId, destPath) {
        if (downloadProc.running)
            return;
        downloadProc.command = [
            "curl", "-s", "--max-time", "30",
            "-H", "X-API-Key: " + api.apiKey,
            "-o", destPath,
            api.server + "/api/v1/file?attachment_id=" + encodeURIComponent(attachmentId)
        ];
        downloadProc.running = true;
    }

    // ── IM Message ──
    function sendImMessage(umo, message) {
        if (imProc.running)
            return;
        imProc._buf = "";
        let body = { umo: umo, message: message };
        imProc.command = [
            "curl", "-s", "--max-time", "10",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-H", "X-API-Key: " + api.apiKey,
            "-d", JSON.stringify(body),
            api.server + "/api/v1/im/message"
        ];
        imProc.running = true;
    }

    // ── Process: Chat SSE ──
    // AstrBot SSE 格式:
    //   data: {"type":"session_id","data":null,"session_id":"..."}
    //   data: {"type":"plain","data":"内容","streaming":true,...}
    //   data: {"type":"complete","data":"完整回复","reasoning":"...",...}
    //   data: {"type":"agent_stats","data":{...}}
    //   data: {"type":"message_saved","data":{...}}
    //   data: {"type":"end","data":"","streaming":false,...}
    Process {
        id: chatProc

        property string _err: ""
        property string _fullContent: ""
        property string _lastToolName: ""
        property bool _gotData: false

        stdout: SplitParser {
            onRead: data => {
                let line = data.trim();
                if (line.length === 0 || line.startsWith(":"))
                    return;
                // 剥离 "data: " 前缀
                if (line.startsWith("data: "))
                    line = line.substring(6);
                else if (line.startsWith("data:"))
                    line = line.substring(5);
                else
                    return;

                try {
                    let obj = JSON.parse(line);
                    chatProc._gotData = true;

                    switch (obj.type) {
                    case "session_id":
                        if (obj.session_id)
                            api.chatSessionId(obj.session_id);
                        break;
                    case "plain":
                        if (obj.chain_type === "tool_call") {
                            try {
                                let tc = JSON.parse(obj.data);
                                chatProc._lastToolName = tc.name || "";
                                api.chatToolCall(tc.name || "", JSON.stringify(tc.args || {}));
                            } catch (e2) {}
                        } else if (obj.chain_type === "tool_call_result") {
                            try {
                                let tr = JSON.parse(obj.data);
                                api.chatToolResult(chatProc._lastToolName, tr.result || obj.data);
                            } catch (e2) {
                                api.chatToolResult(chatProc._lastToolName, obj.data || "");
                            }
                        } else if (obj.data) {
                            api.chatChunk(obj.data);
                        }
                        break;
                    case "complete":
                        chatProc._fullContent = obj.data || "";
                        break;
                    case "agent_stats":
                        if (obj.data)
                            api.chatStats(obj.data);
                        break;
                    case "end":
                        api.chatComplete(chatProc._fullContent);
                        break;
                    }
                } catch (e) {}
            }
        }

        onExited: (exitCode, exitStatus) => {
            if (exitCode !== 0 && !_gotData) {
                api.chatError("连接失败 (exit " + exitCode + ")");
            } else if (!_gotData) {
                api.chatError("未收到响应");
            }
            // 如果收到过数据但没收到 end 事件（异常中断）
            else if (_fullContent === "" && _gotData) {
                api.chatComplete("");
            }
            _err = "";
            _fullContent = "";
            _gotData = false;
        }

        stderr: SplitParser {
            onRead: data => chatProc._err += data
        }
    }

    // ── Process: Configs ──
    Process {
        id: configsProc

        property string _buf: ""

        stdout: SplitParser {
            onRead: data => configsProc._buf += data
        }

        onExited: {
            try {
                let obj = JSON.parse(_buf);
                if (obj.status === "ok" && obj.data && obj.data.configs)
                    api.configsLoaded(obj.data.configs);
            } catch (e) {}
            _buf = "";
        }
    }

    // ── Process: Sessions ──
    Process {
        id: sessionsProc

        property string _buf: ""

        stdout: SplitParser {
            onRead: data => sessionsProc._buf += data
        }

        onExited: {
            try {
                let obj = JSON.parse(_buf);
                if (obj.status === "ok" && obj.data && obj.data.sessions)
                    api.sessionsLoaded(obj.data.sessions);
            } catch (e) {}
            _buf = "";
        }
    }

    // ── Process: Bots ──
    Process {
        id: botsProc

        property string _buf: ""

        stdout: SplitParser {
            onRead: data => botsProc._buf += data
        }

        onExited: {
            try {
                let obj = JSON.parse(_buf);
                if (obj.status === "ok" && obj.data && obj.data.bot_ids)
                    api.botsLoaded(obj.data.bot_ids);
            } catch (e) {}
            _buf = "";
        }
    }

    // ── Process: File Upload ──
    Process {
        id: uploadProc

        property string _buf: ""

        stdout: SplitParser {
            onRead: data => uploadProc._buf += data
        }

        onExited: {
            try {
                let obj = JSON.parse(_buf);
                if (obj.status === "ok" && obj.data)
                    api.fileUploaded(obj.data.attachment_id, obj.data.filename);
            } catch (e) {}
            _buf = "";
        }
    }

    // ── Process: File Download ──
    Process {
        id: downloadProc
    }

    // ── Process: IM Message ──
    Process {
        id: imProc

        property string _buf: ""

        stdout: SplitParser {
            onRead: data => imProc._buf += data
        }

        onExited: {
            try {
                let obj = JSON.parse(_buf);
                api.imMessageSent(obj.status === "ok");
            } catch (e) {
                api.imMessageSent(false);
            }
            _buf = "";
        }
    }
}
