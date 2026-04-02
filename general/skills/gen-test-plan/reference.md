---
project: ShopEase 在线订单系统
version: "2.0"
author: 测试团队 - 李明
date: "2026-04-01"
status: Approved
module: 订单模块
test-type:
  - 集成测试
  - E2E测试
  - 回归测试
environment:
  os: Ubuntu 22.04 / macOS 14
  runtime: Node.js 20 LTS
  database: PostgreSQL 16
---

# ShopEase 在线订单系统 v2.0 测试计划

## 概述

### 测试目的

验证 ShopEase 在线订单系统 v2.0 的订单模块在以下方面的正确性和稳定性：

- 订单创建、查询、修改、取消的完整生命周期
- 库存扣减与回滚的一致性
- 支付回调处理的幂等性
- 多角色权限控制（管理员 / 商家 / 普通用户）

### 测试范围

**包含：**

- 订单 CRUD API（`/api/v2/orders`）
- 库存服务联动（扣减、锁定、释放）
- 支付网关回调（微信支付、支付宝）
- 订单状态机流转
- 管理后台订单管理页面

**排除：**

- 物流追踪模块（v2.1 交付）
- 发票系统（独立项目组负责）
- 性能压测（另有专项计划）

## 测试策略

### 测试类型

| 类型 | 目标 | 工具 |
|---|---|---|
| 集成测试 | 验证订单服务与库存/支付服务的交互 | Jest + Supertest |
| E2E 测试 | 模拟用户完整下单流程 | Playwright |
| 回归测试 | 确保 v1.x 已有功能未被破坏 | Jest + 自动化套件 |

### 测试层级

- **L1（冒烟测试）**: 部署后立即执行，覆盖 P0 用例，5 分钟内完成
- **L2（核心回归）**: 每日 CI 执行，覆盖 P0 + P1 用例
- **L3（全量回归）**: 发版前执行，覆盖全部用例

### 优先级定义

- **P0**: 阻断性缺陷，阻止发版。如：无法创建订单、支付后订单状态未更新
- **P1**: 核心功能缺陷。如：订单取消后库存未释放、权限校验绕过
- **P2**: 重要但非阻断。如：订单列表分页异常、筛选条件不生效
- **P3**: 边缘场景。如：超长备注截断、特殊字符显示异常

## 测试环境

| 项目 | 规格 |
|---|---|
| 操作系统 | Ubuntu 22.04 LTS（CI）/ macOS 14（本地开发） |
| 运行时 | Node.js 20.11 LTS |
| 数据库 | PostgreSQL 16.2（Docker） |
| 缓存 | Redis 7.2（Docker） |
| 消息队列 | RabbitMQ 3.13（Docker） |
| 支付网关 | Mock 服务（见 Fixtures） |
| 浏览器 | Chromium 121（Playwright 内置） |

启动命令：

```bash title="环境启动"
docker compose -f docker-compose.test.yml up -d
npm run db:migrate:test
npm run db:seed:test
```

## Fixtures

### 用户认证 fixture

用于所有需要认证的测试用例（TC-001 至 TC-010），通过 JWT 签发。

```json title="fixtures/auth-tokens.json"
{
  "admin": {
    "userId": 1,
    "username": "admin@shopease.com",
    "role": "admin",
    "token": "eyJhbGciOiJIUzI1NiJ9.eyJ1c2VySWQiOjEsInJvbGUiOiJhZG1pbiJ9.TEST_SIGNATURE"
  },
  "merchant": {
    "userId": 2,
    "username": "merchant@shopease.com",
    "role": "merchant",
    "shopId": 100,
    "token": "eyJhbGciOiJIUzI1NiJ9.eyJ1c2VySWQiOjIsInJvbGUiOiJtZXJjaGFudCJ9.TEST_SIGNATURE"
  },
  "customer": {
    "userId": 3,
    "username": "customer@test.com",
    "role": "customer",
    "token": "eyJhbGciOiJIUzI1NiJ9.eyJ1c2VySWQiOjMsInJvbGUiOiJjdXN0b21lciJ9.TEST_SIGNATURE"
  }
}
```

### 商品与库存 fixture

用于 TC-001、TC-002、TC-005，通过数据库种子脚本导入。

```sql title="fixtures/seed-products.sql"
INSERT INTO products (id, name, price, status) VALUES
  (1001, '无线蓝牙耳机 Pro', 299.00, 'active'),
  (1002, '机械键盘 K8', 499.00, 'active'),
  (1003, '已下架商品-测试', 99.00, 'inactive');

INSERT INTO inventory (product_id, stock, locked) VALUES
  (1001, 100, 0),
  (1002, 5, 0),
  (1003, 0, 0);
```

### 订单请求体 fixture

用于 TC-001（正常下单）和 TC-003（库存不足）。

```json title="fixtures/create-order-request.json"
{
  "normal": {
    "items": [
      { "productId": 1001, "quantity": 2 },
      { "productId": 1002, "quantity": 1 }
    ],
    "addressId": 501,
    "couponCode": null,
    "remark": "请尽快发货"
  },
  "outOfStock": {
    "items": [
      { "productId": 1002, "quantity": 999 }
    ],
    "addressId": 501,
    "couponCode": null,
    "remark": ""
  },
  "inactiveProduct": {
    "items": [
      { "productId": 1003, "quantity": 1 }
    ],
    "addressId": 501,
    "couponCode": null,
    "remark": ""
  }
}
```

### 支付回调 Mock fixture

用于 TC-004（支付成功回调）和 TC-006（重复回调幂等）。

```json title="fixtures/payment-callback-mock.json"
{
  "wechatPaySuccess": {
    "method": "POST",
    "path": "/api/v2/payment/callback/wechat",
    "body": {
      "appid": "wx1234567890",
      "mch_id": "1234567890",
      "out_trade_no": "{{orderId}}",
      "transaction_id": "4200001234202604010000001",
      "result_code": "SUCCESS",
      "total_fee": 109700,
      "sign": "MOCK_SIGN_VALUE"
    }
  },
  "alipaySuccess": {
    "method": "POST",
    "path": "/api/v2/payment/callback/alipay",
    "body": {
      "out_trade_no": "{{orderId}}",
      "trade_no": "2026040122001400001",
      "trade_status": "TRADE_SUCCESS",
      "total_amount": "1097.00",
      "sign": "MOCK_SIGN_VALUE"
    }
  }
}
```

### 环境变量模板

用于全部测试用例，测试启动前加载。

```bash title="fixtures/.env.test"
DATABASE_URL=postgresql://test:test@localhost:5433/shopease_test
REDIS_URL=redis://localhost:6380/0
RABBITMQ_URL=amqp://guest:guest@localhost:5673
JWT_SECRET=test-jwt-secret-key-do-not-use-in-production
WECHAT_PAY_MCH_ID=1234567890
ALIPAY_APP_ID=2026040100000001
PAYMENT_CALLBACK_BASE_URL=http://localhost:3001
```

## 测试用例

| ID | 模块 | 描述 | 前置条件 / Fixture | 步骤 | 预期结果 | 优先级 |
|---|---|---|---|---|---|---|
| TC-001 | 订单创建 | 正常创建订单 | auth-tokens (customer), seed-products, create-order-request (normal) | 1. 以 customer 身份调用 POST /api/v2/orders 2. 传入 normal 请求体 | 返回 201，订单状态为 pending_payment，库存扣减 | P0 |
| TC-002 | 订单查询 | 查询订单详情 | auth-tokens (customer), TC-001 已执行 | 1. 以 customer 身份调用 GET /api/v2/orders/:id | 返回 200，包含完整订单信息和商品明细 | P0 |
| TC-003 | 订单创建 | 库存不足时创建订单 | auth-tokens (customer), seed-products, create-order-request (outOfStock) | 1. 以 customer 身份调用 POST /api/v2/orders 2. 传入 outOfStock 请求体 | 返回 409，错误信息包含库存不足提示，库存不变 | P0 |
| TC-004 | 支付回调 | 微信支付成功回调 | payment-callback-mock (wechatPaySuccess), TC-001 已执行 | 1. 模拟微信支付回调 POST 2. 查询订单状态 | 订单状态变为 paid，支付记录已创建 | P0 |
| TC-005 | 订单取消 | 用户取消未支付订单 | auth-tokens (customer), TC-001 已执行 | 1. 以 customer 身份调用 PUT /api/v2/orders/:id/cancel | 返回 200，订单状态为 cancelled，库存已释放 | P1 |
| TC-006 | 支付回调 | 重复支付回调幂等性 | payment-callback-mock (wechatPaySuccess), TC-004 已执行 | 1. 再次发送相同的微信支付回调 | 返回 200，订单状态仍为 paid，不产生重复支付记录 | P1 |
| TC-007 | 权限控制 | 普通用户无法查看他人订单 | auth-tokens (customer + merchant) | 1. 以 customer 身份查询 merchant 的订单 | 返回 403 Forbidden | P1 |
| TC-008 | 权限控制 | 管理员可查看所有订单 | auth-tokens (admin) | 1. 以 admin 身份调用 GET /api/v2/admin/orders | 返回 200，包含分页订单列表 | P1 |
| TC-009 | 订单创建 | 购买已下架商品 | auth-tokens (customer), seed-products, create-order-request (inactiveProduct) | 1. 以 customer 身份下单已下架商品 | 返回 400，错误信息包含商品已下架 | P2 |
| TC-010 | 订单列表 | 订单列表分页与筛选 | auth-tokens (customer), 多条已有订单 | 1. 调用 GET /api/v2/orders?page=1&size=10&status=paid | 返回 200，分页信息正确，仅包含 paid 状态订单 | P2 |

## 进入/退出标准

### 进入标准

- 订单模块代码已完成并合并到 `release/2.0` 分支
- 测试环境已部署且所有依赖服务可用
- Fixtures 数据已导入，`npm run db:seed:test` 执行成功
- 冒烟测试（L1）全部通过

### 退出标准

- P0 用例通过率 100%
- P1 用例通过率 >= 95%
- P2 用例通过率 >= 80%
- 无未修复的 P0/P1 缺陷
- 测试报告已提交并经测试负责人审批

## 风险与缓解措施

| 风险 | 影响 | 可能性 | 缓解措施 |
|---|---|---|---|
| 支付网关 Mock 与真实行为不一致 | 支付回调测试结果不可靠 | 中 | 定期对比 Mock 与沙箱环境响应；预留支付沙箱测试窗口 |
| 测试数据库与生产 schema 不同步 | 测试通过但生产出错 | 低 | CI 流水线中加入 migration 一致性检查 |
| 并发场景下库存超卖 | 库存数据不一致 | 中 | 补充并发集成测试（10 并发下单同一商品） |
| 第三方服务限流 | E2E 测试不稳定 | 低 | 使用本地 Mock 服务替代；限流测试单独安排 |

## 时间安排与资源分配

| 阶段 | 时间 | 负责人 | 内容 |
|---|---|---|---|
| 测试准备 | 04-02 ~ 04-03 | 李明 | 环境搭建、fixture 数据准备、自动化脚本编写 |
| L1 冒烟测试 | 04-04 | 李明 | 部署后执行 P0 用例，确认基本功能可用 |
| L2 核心回归 | 04-05 ~ 04-08 | 李明、张伟 | 执行 P0 + P1 用例，提交缺陷 |
| 缺陷修复验证 | 04-09 ~ 04-10 | 李明 | 验证已修复缺陷，补充回归 |
| L3 全量回归 | 04-11 | 李明、张伟 | 执行全部用例，生成测试报告 |
| 报告与评审 | 04-12 | 李明 | 提交测试报告，评审会议 |
