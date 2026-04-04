// =====================================================================
// actions-preview.nvim 验证文件
// 用 nvim 打开这个文件，光标移到指定位置，按 <leader>ca 看 diff 预览
// =====================================================================

// ----- 测试 1：缺少 import -----
// 光标放在 readFileSync 上 → <leader>ca → 应该看到 "Add import from 'fs'" 的 diff
const data = readFileSync("/tmp/test.txt", "utf-8");

// ----- 测试 2：unused variable -----
// 光标放在 unusedVar 上 → <leader>ca → 可能看到 "Remove unused variable" 的 diff
const unusedVar = 42;
console.log("hello");

// ----- 测试 3：类型转换建议 -----
// 光标放在 parseInt 上 → <leader>ca
const num = parseInt("123");

// ----- 测试 4：可以简化的条件表达式 -----
// 光标放在 if 上 → <leader>ca → 可能提示简化写法
function isPositive(n: number): boolean {
    if (n > 0) {
        return true;
    } else {
        return false;
    }
}

// ----- 测试 5：快速修复拼写 -----
// 光标放在 consloe 上 → <leader>ca → 应该提示 "Did you mean console?"
consloe.log("typo");

// ----- 测试 6：提取变量/函数 -----
// 先 visual 选中 a * b + c * d 整个表达式 → <leader>ca → 看 "Extract to variable" 的 diff
function compute(a: number, b: number, c: number, d: number) {
    return a * b + c * d;
}

// ----- 测试 7：organize imports -----
// 光标在文件任意位置 → <leader>ca → 看 "Organize imports" 选项的 diff
import { useState, useEffect } from "react";
import { readFile } from "fs";
import { join } from "path";
