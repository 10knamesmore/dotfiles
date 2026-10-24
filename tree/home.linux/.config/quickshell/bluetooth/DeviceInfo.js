function hasService(device, shortUuid) {
  return device.uuids.some(
    (uuid) =>
      uuid.toLowerCase() ===
      "0000" + shortUuid + "-0000-1000-8000-00805f9b34fb",
  );
}

function isAudioOutput(device) {
  return (
    hasService(device, "110b") ||
    hasService(device, "111e") ||
    hasService(device, "1108") ||
    hasService(device, "184e")
  );
}

function serviceNames(device) {
  const services = [
    ["110b", "音频输出（A2DP）"],
    ["110a", "音频来源"],
    ["111e", "免提通话"],
    ["1108", "耳机通话"],
    ["110e", "媒体遥控"],
    ["1124", "键盘 / 鼠标（HID）"],
    ["1812", "低功耗输入设备"],
    ["180f", "电池"],
    ["184e", "LE Audio"],
  ];
  return services
    .filter((entry) => hasService(device, entry[0]))
    .map((entry) => entry[1]);
}

function icon(device) {
  if (isAudioOutput(device)) return "󰋋";
  const icons = {
    "input-keyboard": "󰌌",
    "input-mouse": "󰍽",
    "input-gaming": "󰊗",
    phone: "󰏲",
    computer: "󰍹",
    "audio-card": "󰓃",
    "audio-headset": "󰋋",
    "audio-headphones": "󰋋",
  };
  return icons[device.icon] || "󰂱";
}

function operationText(operation) {
  if (!operation) return "";
  const names = {
    connect: "准备连接…",
    pairing: "配对中…",
    connecting: "连接中…",
    disconnect: "正在断开…",
    cancel: "正在取消…",
    forget: "正在忘记…",
  };
  return names[operation.stage] || operation.stage;
}
