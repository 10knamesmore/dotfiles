# 交互类型

使用以下规范化类型，选择最接近的一项并保持一致。

- primary_action: 核心业务动作（create/submit/save/order/pay）。
- secondary_action: 辅助动作（duplicate/export/preview）。
- state_change: 状态或生命周期变更（pause/resume/cancel/enable/disable）。
- navigation: 路由跳转、打开页面、外链。
- dialog_open: 打开 modal/drawer/dialog。
- dialog_confirm: 在弹窗内确认。
- dialog_cancel: 在弹窗内取消/关闭。
- selection: 带业务含义的选择/筛选变更。
- bulk_action: 对多项执行动作。
- import: 上传或导入数据。
- export: 下载或导出数据。
- data_refresh: 用户主动刷新/重载。
- auto_action: 自动触发且有业务影响（on mount/interval）。

说明：
- 如果提交后伴随跳转，类型仍为 `primary_action`，并在 result.ui 写明跳转。
- 若行为以跳转为主要可观察结果，优先用 `navigation`。
