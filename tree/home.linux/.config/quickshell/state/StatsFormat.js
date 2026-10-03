.pragma library

/**
 * @param {number} value 字节数。
 * @returns {string} 一位小数的 GiB 数值，不含单位。
 */
function gib(value) {
    return (value / 1073741824).toFixed(1);
}

/**
 * @param {number} value 字节数。
 * @returns {string} 含二进制容量单位的数值。
 */
function bytes(value) {
    if (value < 1024)
        return value.toFixed(0) + " B";
    if (value < 1048576)
        return (value / 1024).toFixed(1) + " KiB";
    if (value < 1073741824)
        return (value / 1048576).toFixed(1) + " MiB";
    return (value / 1073741824).toFixed(2) + " GiB";
}

/**
 * @param {number} value 字节/秒。
 * @returns {string} 含速率单位的数值。
 */
function speed(value) {
    return bytes(value) + "/s";
}
