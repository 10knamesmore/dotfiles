# FileScan（证券增强文件单）页面 data-testid 覆盖文档

目标：帮助 Agent 通过自然语言定位元素，并能将页面组件组合成 Playwright PageObject（含示例）。

## 0. 页面功能概览

**页面名称**：证券增强文件单 / FileScan

**核心功能**：
- 配置并启动/停止文件扫描任务，从指定路径读取增强文件单（JSON格式）
- 实时显示扫描统计数据（母单/子单委托数、失败数、确认数等）
- 查看待确认、已确认、已移除的母单列表，支持确认或移除操作
- 查看运行日志，支持按内容和类型筛选

**主要交互流程**：
1. 设置扫描路径、扫描频率、导出频率
2. 点击"启动"按钮开始扫描
3. 系统实时更新统计数据和待确认母单列表
4. 用户可选择母单进行确认或移除操作
5. 查看运行日志了解扫描详情

---

## 1. 页面组件树（组合化）

- FileScan
  - wrapper: `filescan.page`
  - Setting Header: `filescan.setting-header`
  - FileScanSetting: `filescan.setting`
    - 运行状态显示
    - 启动/停止按钮
    - 文件路径选择
    - 下载模板按钮
    - 扫描频率设置
    - 导出频率设置
    - 自动启动复选框
  - Statistics: `filescan.statistics`
    - 10个统计项（母单委托数、母单失败数等）
  - OrderTable: `filescan.order-table`（仅全商版显示）
    - 待确认母单表格
    - 已确认母单表格
    - 已移除母单表格
    - 自动确认复选框
    - 确认/移除按钮
  - Log Header: `filescan.log-header`
  - LogTable: `filescan.log-table`
    - 日志目录显示
    - 内容搜索框
    - 类型筛选下拉框
    - 日志列表表格

---

## 2. testid 前缀与规则

- FileScan 页面根：`filescan.page`
- FileScanSetting 组件：`filescan.setting.{field}.{type}`
- Statistics 组件：`filescan.statistics.{title}.{label|value}`
- OrderTable 组件：`filescan.order-table.{field}.{type}`
- OrderTable 表格列：`filescan.order-table.table-cell.{field}`
- LogTable 组件：`filescan.log-table.{field}.{type}`
- LogTable 表格列：`filescan.log-table.table-cell.{field}`

---

## 3. 组件级覆盖明细（自然语言 -> testid）

### 3.1 主页面结构

| 自然语言描述 | testid | 元素类型 | 交互方式 |
|------------|--------|---------|---------|
| 页面根容器 | `filescan.page` | div | 容器 |
| 运行设置标题 | `filescan.setting-header` | div | 文本 |
| 运行日志标题 | `filescan.log-header` | div | 文本 |
| 日志限制说明 | `filescan.log-limit.text` | div | 文本 |

### 3.2 FileScanSetting（运行设置）

| 自然语言描述 | testid | 元素类型 | 交互方式 |
|------------|--------|---------|---------|
| 设置容器 | `filescan.setting` | div | 容器 |
| 运行状态文本 | `filescan.setting.status.text` | div | 文本 |
| 启动/停止按钮 | `filescan.setting.start-stop.button` | Button | 点击 |
| 文件路径输入框 | `filescan.setting.order-path.input` | FilePicker | 输入/选择 |
| 下载模板按钮 | `filescan.setting.download-template.button` | Button | 点击 |
| 扫描频率输入框 | `filescan.setting.scan-interval.input` | InputNumber | 输入 |
| 导出频率输入框 | `filescan.setting.report-interval.input` | InputNumber | 输入 |
| 登录自动启动复选框 | `filescan.setting.auto-start.checkbox` | Checkbox | 勾选 |

**说明**：
- `start-stop.button` 按钮文本会根据状态变化："启动" / "启动中" / "停止" / "停止中"
- `status.text` 显示 "运行中" 或 "未启动"

### 3.3 Statistics（数据统计）

| 自然语言描述 | testid | 元素类型 | 交互方式 |
|------------|--------|---------|---------|
| 统计容器 | `filescan.statistics` | div | 容器 |
| 母单委托数标签 | `filescan.statistics.母单委托数.label` | div | 文本 |
| 母单委托数值 | `filescan.statistics.母单委托数.value` | span | 文本 |
| 母单失败数标签 | `filescan.statistics.母单失败数.label` | div | 文本 |
| 母单失败数值 | `filescan.statistics.母单失败数.value` | span | 文本 |
| 母单确认数标签 | `filescan.statistics.母单确认数.label` | div | 文本 |
| 母单确认数值 | `filescan.statistics.母单确认数.value` | span | 文本 |
| 母单移除数标签 | `filescan.statistics.母单移除数.label` | div | 文本 |
| 母单移除数值 | `filescan.statistics.母单移除数.value` | span | 文本 |
| 交易账户数标签 | `filescan.statistics.交易账户数.label` | div | 文本 |
| 交易账户数值 | `filescan.statistics.交易账户数.value` | span | 文本 |
| 子单委托数标签 | `filescan.statistics.子单委托数.label` | div | 文本 |
| 子单委托数值 | `filescan.statistics.子单委托数.value` | span | 文本 |
| 子单错废单数标签 | `filescan.statistics.子单错废单数.label` | div | 文本 |
| 子单错废单数值 | `filescan.statistics.子单错废单数.value` | span | 文本 |
| 子单撤单数标签 | `filescan.statistics.子单撤单数.label` | div | 文本 |
| 子单撤单数值 | `filescan.statistics.子单撤单数.value` | span | 文本 |
| 子单错废单率标签 | `filescan.statistics.子单错废单率.label` | div | 文本 |
| 子单错废单率值 | `filescan.statistics.子单错废单率.value` | span | 文本 |
| 子单撤单率标签 | `filescan.statistics.子单撤单率.label` | div | 文本 |
| 子单撤单率值 | `filescan.statistics.子单撤单率.value` | span | 文本 |

**说明**：
- 统计项的 testid 使用中文标题作为标识，因为标题是业务语义的最佳体现
- 数值每2秒自动更新

### 3.4 OrderTable（母单表格）

#### 3.4.1 表格容器与操作

| 自然语言描述 | testid | 元素类型 | 交互方式 |
|------------|--------|---------|---------|
| 订单表格容器 | `filescan.order-table` | CustomTabs | 容器 |
| 待确认表格 | `filescan.order-table.waiting-confirm` | FtTable | 表格 |
| 已确认表格 | `filescan.order-table.confirmed` | FtTable | 表格 |
| 已移除表格 | `filescan.order-table.removed` | FtTable | 表格 |
| 自动确认复选框 | `filescan.order-table.auto-confirm.checkbox` | Checkbox | 勾选 |
| 确认按钮 | `filescan.order-table.confirm.button` | Button | 点击 |
| 移除按钮 | `filescan.order-table.remove.button` | Button | 点击 |

**说明**：
- 订单表格仅在全商版（`is_quanshang`）显示
- 确认/移除按钮仅在"待确认母单"标签页激活

#### 3.4.2 表格列（Table Cells）

所有表格列均遵循：`filescan.order-table.table-cell.{field}`

| 字段名 | testid | 表头 | 说明 |
|-------|--------|------|------|
| external_id | `filescan.order-table.table-cell.external-id` | 自定义委托编号 | 文本 |
| trade_acc | `filescan.order-table.table-cell.trade-acc` | 账户名称 | 文本 |
| stock_code | `filescan.order-table.table-cell.stock-code` | 证券代码 | 文本 |
| exchange_id | `filescan.order-table.table-cell.exchange-id` | 交易所 | 显示中文名称 |
| bs_flag | `filescan.order-table.table-cell.bs-flag` | 买卖标识 | TradeTag组件 |
| order_vol | `filescan.order-table.table-cell.order-vol` | 委托数量 | 格式化数字 |
| begin_tm | `filescan.order-table.table-cell.begin-tm` | 开始时间 | 时间戳格式化 |
| end_tm | `filescan.order-table.table-cell.end-tm` | 结束时间 | 时间戳格式化 |
| basket_name | `filescan.order-table.table-cell.basket-name` | 篮子名称 | 文本 |
| algo_type | `filescan.order-table.table-cell.algo-type` | 算法类型 | TradeTag组件 |
| strategy_param | `filescan.order-table.table-cell.strategy-param` | 算法参数 | 文本 |

### 3.5 LogTable（运行日志）

#### 3.5.1 日志控制

| 自然语言描述 | testid | 元素类型 | 交互方式 |
|------------|--------|---------|---------|
| 日志表格容器 | `filescan.log-table` | div | 容器 |
| 日志目录输入框 | `filescan.log-table.log-dir.input` | FilePicker | 只读 |
| 内容搜索框 | `filescan.log-table.search-content.input` | Input | 输入 |
| 类型筛选下拉框 | `filescan.log-table.filter-level.select` | Select | 选择 |

**说明**：
- 日志目录不可编辑，固定为客户端日志目录下的scan文件夹
- 类型筛选选项：INFO、WARNING、ERROR

#### 3.5.2 日志表格列（Table Cells）

所有日志表格列均遵循：`filescan.log-table.table-cell.{field}`

| 字段名 | testid | 表头 | 说明 |
|-------|--------|------|------|
| time | `filescan.log-table.table-cell.time` | 时间 | 时间戳格式化 |
| level | `filescan.log-table.table-cell.level` | 类型 | TradeTag组件（不同颜色） |
| msg | `filescan.log-table.table-cell.msg` | 内容 | 文本，ERROR类型显示红色 |

---

## 4. 覆盖缺失说明

### 4.1 已覆盖的元素

- ✅ 所有可交互元素（按钮、输入框、复选框、下拉框）
- ✅ 所有可断言的数据（状态文本、统计数值）
- ✅ 所有表格列（母单表格、日志表格）

### 4.2 不需要 testid 的元素

- 纯装饰性图标（iconfont）
- 分隔线（Divider）
- 静态布局容器（无业务语义）

### 4.3 兜底定位策略

如遇特殊情况需要定位未添加 testid 的元素，可使用：
- 文本内容定位：`page.getByText('运行设置')`
- 角色定位：`page.getByRole('button', { name: '启动' })`
- Placeholder定位：`page.getByPlaceholder('内容')`

---

## 5. Playwright PageObject 示例

### 5.1 页面根类

```typescript
import { Page, Locator } from '@playwright/test';

export class FileScanPage {
  readonly page: Page;
  readonly pageRoot: Locator;
  readonly settingHeader: Locator;
  readonly logHeader: Locator;
  
  // 子组件
  readonly setting: FileScanSettingComponent;
  readonly statistics: StatisticsComponent;
  readonly orderTable: OrderTableComponent;
  readonly logTable: LogTableComponent;

  constructor(page: Page) {
    this.page = page;
    this.pageRoot = page.locator('[data-testid="filescan.page"]');
    this.settingHeader = page.locator('[data-testid="filescan.setting-header"]');
    this.logHeader = page.locator('[data-testid="filescan.log-header"]');
    
    this.setting = new FileScanSettingComponent(page);
    this.statistics = new StatisticsComponent(page);
    this.orderTable = new OrderTableComponent(page);
    this.logTable = new LogTableComponent(page);
  }

  async goto() {
    // 假设从导航菜单进入
    await this.page.click('[data-testid="nav.filescan"]');
    await this.pageRoot.waitFor();
  }
}
```

### 5.2 FileScanSetting 组件类

```typescript
export class FileScanSettingComponent {
  readonly page: Page;
  readonly container: Locator;
  readonly statusText: Locator;
  readonly startStopButton: Locator;
  readonly orderPathInput: Locator;
  readonly downloadTemplateButton: Locator;
  readonly scanIntervalInput: Locator;
  readonly reportIntervalInput: Locator;
  readonly autoStartCheckbox: Locator;

  constructor(page: Page) {
    this.page = page;
    this.container = page.locator('[data-testid="filescan.setting"]');
    this.statusText = page.locator('[data-testid="filescan.setting.status.text"]');
    this.startStopButton = page.locator('[data-testid="filescan.setting.start-stop.button"]');
    this.orderPathInput = page.locator('[data-testid="filescan.setting.order-path.input"]');
    this.downloadTemplateButton = page.locator('[data-testid="filescan.setting.download-template.button"]');
    this.scanIntervalInput = page.locator('[data-testid="filescan.setting.scan-interval.input"]');
    this.reportIntervalInput = page.locator('[data-testid="filescan.setting.report-interval.input"]');
    this.autoStartCheckbox = page.locator('[data-testid="filescan.setting.auto-start.checkbox"]');
  }

  async getStatus(): Promise<string> {
    return await this.statusText.textContent() || '';
  }

  async startScan() {
    const statusBefore = await this.getStatus();
    if (statusBefore === '未启动') {
      await this.startStopButton.click();
      await this.page.waitForTimeout(500); // 等待状态切换
    }
  }

  async stopScan() {
    const statusBefore = await this.getStatus();
    if (statusBefore === '运行中') {
      await this.startStopButton.click();
      await this.page.waitForTimeout(500);
    }
  }

  async setOrderPath(path: string) {
    await this.orderPathInput.fill(path);
  }

  async setScanInterval(interval: number) {
    await this.scanIntervalInput.fill(interval.toString());
  }

  async setReportInterval(interval: number) {
    await this.reportIntervalInput.fill(interval.toString());
  }

  async enableAutoStart(enable: boolean) {
    const isChecked = await this.autoStartCheckbox.isChecked();
    if (isChecked !== enable) {
      await this.autoStartCheckbox.click();
    }
  }

  async downloadTemplate() {
    await this.downloadTemplateButton.click();
  }
}
```

### 5.3 Statistics 组件类

```typescript
export class StatisticsComponent {
  readonly page: Page;
  readonly container: Locator;

  constructor(page: Page) {
    this.page = page;
    this.container = page.locator('[data-testid="filescan.statistics"]');
  }

  async getStatValue(title: string): Promise<string> {
    const locator = this.page.locator(`[data-testid="filescan.statistics.${title}.value"]`);
    return await locator.textContent() || '0';
  }

  async getStrategyOrderScanned(): Promise<number> {
    const value = await this.getStatValue('母单委托数');
    return parseInt(value.replace(/,/g, ''));
  }

  async getStrategyOrderFailed(): Promise<number> {
    const value = await this.getStatValue('母单失败数');
    return parseInt(value.replace(/,/g, ''));
  }

  async getStrategyOrderConfirmed(): Promise<number> {
    const value = await this.getStatValue('母单确认数');
    return parseInt(value.replace(/,/g, ''));
  }

  async getSuborderFailedRate(): Promise<number> {
    const value = await this.getStatValue('子单错废单率');
    return parseFloat(value.replace('%', ''));
  }
}
```

### 5.4 OrderTable 组件类

```typescript
export class OrderTableComponent {
  readonly page: Page;
  readonly container: Locator;
  readonly waitingConfirmTable: Locator;
  readonly confirmedTable: Locator;
  readonly removedTable: Locator;
  readonly autoConfirmCheckbox: Locator;
  readonly confirmButton: Locator;
  readonly removeButton: Locator;

  constructor(page: Page) {
    this.page = page;
    this.container = page.locator('[data-testid="filescan.order-table"]');
    this.waitingConfirmTable = page.locator('[data-testid="filescan.order-table.waiting-confirm"]');
    this.confirmedTable = page.locator('[data-testid="filescan.order-table.confirmed"]');
    this.removedTable = page.locator('[data-testid="filescan.order-table.removed"]');
    this.autoConfirmCheckbox = page.locator('[data-testid="filescan.order-table.auto-confirm.checkbox"]');
    this.confirmButton = page.locator('[data-testid="filescan.order-table.confirm.button"]');
    this.removeButton = page.locator('[data-testid="filescan.order-table.remove.button"]');
  }

  async switchToWaitingConfirm() {
    await this.page.click('text=待确认母单');
    await this.waitingConfirmTable.waitFor();
  }

  async switchToConfirmed() {
    await this.page.click('text=已确认母单');
    await this.confirmedTable.waitFor();
  }

  async switchToRemoved() {
    await this.page.click('text=已移除母单');
    await this.removedTable.waitFor();
  }

  async enableAutoConfirm(enable: boolean) {
    const isChecked = await this.autoConfirmCheckbox.isChecked();
    if (isChecked !== enable) {
      await this.autoConfirmCheckbox.click();
    }
  }

  async confirmSelectedOrders() {
    await this.confirmButton.click();
  }

  async removeSelectedOrders() {
    await this.removeButton.click();
  }

  async getOrderCellValue(row: number, field: string): Promise<string> {
    const cellLocator = this.page.locator(
      `[data-testid="filescan.order-table.waiting-confirm"] ` +
      `[data-testid="filescan.order-table.table-cell.${field}"]`
    ).nth(row);
    return await cellLocator.textContent() || '';
  }
}
```

### 5.5 LogTable 组件类

```typescript
export class LogTableComponent {
  readonly page: Page;
  readonly container: Locator;
  readonly logDirInput: Locator;
  readonly searchContentInput: Locator;
  readonly filterLevelSelect: Locator;

  constructor(page: Page) {
    this.page = page;
    this.container = page.locator('[data-testid="filescan.log-table"]');
    this.logDirInput = page.locator('[data-testid="filescan.log-table.log-dir.input"]');
    this.searchContentInput = page.locator('[data-testid="filescan.log-table.search-content.input"]');
    this.filterLevelSelect = page.locator('[data-testid="filescan.log-table.filter-level.select"]');
  }

  async searchByContent(content: string) {
    await this.searchContentInput.fill(content);
    await this.page.waitForTimeout(300); // 等待筛选生效
  }

  async filterByLevel(level: 'INFO' | 'WARNING' | 'ERROR' | null) {
    if (level === null) {
      await this.filterLevelSelect.click();
      await this.page.click('.ant-select-clear');
    } else {
      await this.filterLevelSelect.click();
      await this.page.click(`text=${level}`);
    }
  }

  async getLogCellValue(row: number, field: 'time' | 'level' | 'msg'): Promise<string> {
    const cellLocator = this.page.locator(
      `[data-testid="filescan.log-table.table-cell.${field}"]`
    ).nth(row);
    return await cellLocator.textContent() || '';
  }

  async getLogCount(): Promise<number> {
    const rows = await this.page.locator('[data-testid="filescan.log-table.table-cell.time"]').count();
    return rows;
  }
}
```

---

## 6. 样例用例片段

### 6.1 配置并启动文件扫描

```typescript
import { test, expect } from '@playwright/test';
import { FileScanPage } from './pages/FileScanPage';

test('配置并启动文件扫描', async ({ page }) => {
  const fileScan = new FileScanPage(page);
  
  // 进入页面
  await fileScan.goto();
  
  // 配置扫描设置
  await fileScan.setting.setOrderPath('/path/to/orders');
  await fileScan.setting.setScanInterval(100);
  await fileScan.setting.setReportInterval(2000);
  await fileScan.setting.enableAutoStart(true);
  
  // 启动扫描
  await fileScan.setting.startScan();
  
  // 验证状态
  const status = await fileScan.setting.getStatus();
  expect(status).toBe('运行中');
  
  // 等待统计数据更新
  await page.waitForTimeout(3000);
  const scannedCount = await fileScan.statistics.getStrategyOrderScanned();
  expect(scannedCount).toBeGreaterThan(0);
});
```

### 6.2 确认待确认母单

```typescript
test('确认待确认母单', async ({ page }) => {
  const fileScan = new FileScanPage(page);
  await fileScan.goto();
  
  // 切换到待确认母单标签
  await fileScan.orderTable.switchToWaitingConfirm();
  
  // 选择第一个订单（假设表格支持行选择）
  await page.click('[data-testid="filescan.order-table.waiting-confirm"] tbody tr:first-child');
  
  // 点击确认按钮
  await fileScan.orderTable.confirmSelectedOrders();
  
  // 验证订单已移至已确认列表
  await fileScan.orderTable.switchToConfirmed();
  const confirmedCount = await page.locator('[data-testid="filescan.order-table.confirmed"] tbody tr').count();
  expect(confirmedCount).toBeGreaterThan(0);
});
```

### 6.3 筛选运行日志

```typescript
test('筛选运行日志', async ({ page }) => {
  const fileScan = new FileScanPage(page);
  await fileScan.goto();
  
  // 按类型筛选ERROR日志
  await fileScan.logTable.filterByLevel('ERROR');
  
  // 验证所有日志都是ERROR类型
  const logCount = await fileScan.logTable.getLogCount();
  for (let i = 0; i < logCount; i++) {
    const level = await fileScan.logTable.getLogCellValue(i, 'level');
    expect(level).toBe('ERROR');
  }
  
  // 按内容搜索
  await fileScan.logTable.searchByContent('扫描失败');
  
  // 验证日志内容包含搜索关键词
  const firstLogMsg = await fileScan.logTable.getLogCellValue(0, 'msg');
  expect(firstLogMsg).toContain('扫描失败');
});
```

### 6.4 验证统计数据实时更新

```typescript
test('验证统计数据实时更新', async ({ page }) => {
  const fileScan = new FileScanPage(page);
  await fileScan.goto();
  
  // 启动扫描
  await fileScan.setting.startScan();
  
  // 获取初始统计值
  const initialScanned = await fileScan.statistics.getStrategyOrderScanned();
  
  // 等待2秒（统计数据每2秒更新）
  await page.waitForTimeout(2500);
  
  // 获取更新后的统计值
  const updatedScanned = await fileScan.statistics.getStrategyOrderScanned();
  
  // 验证数据已更新（如果有新文件）
  expect(updatedScanned).toBeGreaterThanOrEqual(initialScanned);
});
```

---

## 7. 注意事项

### 7.1 动态内容

- **统计数据**：每2秒自动刷新，测试时需考虑时间延迟
- **日志列表**：每2秒自动刷新，最多显示100条
- **订单列表**：每1秒轮询更新（使用useKeepAlivePollingFn）

### 7.2 条件显示

- **OrderTable**：仅在 `is_quanshang === true` 时显示
- **日志目录**：仅在客户端环境（`isClient()`）显示

### 7.3 状态依赖

- **确认/移除按钮**：仅在"待确认母单"标签页且有选中项时可用
- **自动确认复选框**：当后台配置为禁止自动确认时禁用

### 7.4 表格 testid 说明

- 表格列的 testid 通过 `meta.testid` 字段配置
- FtTable 组件需要支持将 `meta.testid` 渲染到对应的 cell 上
- 如果 FtTable 尚不支持，需要在表格组件层面添加支持

---

**文档版本**：v1.0  
**最后更新**：2026-02-02  
**维护者**：AI Agent (testid-adder skill)
