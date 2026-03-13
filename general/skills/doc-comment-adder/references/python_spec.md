# Python 文档注释规范（Google 风格 Docstring）

## 1. 基本原则

* 使用三引号 `""" ... """`
* 紧贴声明下方
* 首行必须为**一句简洁功能描述（动词开头）**
* 第二行留空
* 后续为结构化段落
* 类型信息以 **类型注解为准**，docstring 不重复类型

---

## 2. 适用范围

必须添加文档注释：

* 公共函数
* 公共类
* 公共方法
* 核心业务逻辑函数
* 复杂算法函数
* 模块级说明（文件顶部）

可选：

* 私有方法（当逻辑复杂时）

---

## 3. 标准结构模板

```python
def function_name(...) -> ReturnType:
    """简要说明函数用途。

    可选：补充详细行为说明、业务语义或边界条件。

    Args:
        param1: 参数语义说明。
        param2: 参数语义说明。

    Returns:
        返回值语义说明。

    Raises:
        异常类型: 触发条件说明。

    Example: <可选>
        >>> function_name(...)
        结果示例
    """
```

---

## 4. 段落说明规范

### 4.1 首句描述

要求：

* 动词开头
* 说明“做什么”
* 不描述实现方式
* 不重复函数名

❌ 错误示例：

```python
"""过滤市场项目"""
```

✅ 正确示例：

```python
"""根据关键字与产品类型过滤金融产品列表。"""
```

---

### 4.2 Args

规则：

* 只说明语义，不重复类型
* 说明单位、范围、默认行为
* 必要时说明 None 的语义

```python
Args:
    timeout: 超时时间（秒），必须大于 0。
    keyword: 匹配关键字；若为 None 或空字符串，则忽略名称过滤。
```

---

### 4.3 Returns

规则：

* 说明返回值语义
* 说明空值情况
* 说明特殊值含义

```python
Returns:
    过滤后的金融产品列表；若无匹配项则返回空列表。
```

---

### 4.4 Raises

规则：

* 只写可能显式抛出的异常
* 必须写触发条件

```python
Raises:
    ValueError: 当 item_type 非法时。
    RuntimeError: 当数据源未初始化时。
```

---

### 4.5 Example (可选)

规则：

* 提供最小可运行示例
* 使用 doctest 风格
* 避免冗长

```python
Example:
    >>> filter_market_items_by_keyword(items, "ETF", item_type=FinItemDescType.FUND)
    [FinItemType(...)]
```

---

### 4.6 Notes（可选）

用于补充：

* 算法复杂度
* 副作用
* 性能说明
* 线程安全语义

---

## 5. 函数示例（完整示范）

```python
def filter_market_items_by_keyword(
    items: Sequence[FinItemType],
    keyword: Optional[str],
    *,
    item_type: FinItemDescType,
) -> List[FinItemType]:
    """根据关键字过滤指定类型的金融产品列表。

    当 keyword 为 None 或空字符串时，仅根据 item_type 进行过滤。

    Args:
        items: 待过滤的金融产品序列。
        keyword: 产品名称匹配关键字；None 或空字符串表示忽略关键字。
        item_type: 指定的金融产品类型约束。

    Returns:
        满足条件的金融产品列表。若无匹配项则返回空列表。

    Raises:
        ValueError: 当 item_type 不合法时。
    """
```

---

## 6. 类规范

```python
class MarketFilter:
    """封装金融产品筛选逻辑。"""

    def __init__(self, source: DataSource) -> None:
        """初始化筛选器。

        Args:
            source: 数据来源接口。
        """
        self._source = source

    def filter(self, keyword: str) -> List[FinItemType]:
        """根据关键字筛选产品。

        Args:
            keyword: 产品名称匹配关键字。

        Returns:
            匹配的产品列表。
        """
```

---

## 7. 模块级注释规范

文件顶部：

```python
"""市场数据过滤模块。

提供基于关键字与产品类型的筛选功能。
适用于交易前端展示与数据分析模块。
"""
```

---

## 8. 风格一致性规则

* 同一文件内保持一致结构
* 不混用 NumPy 风格与 Google 风格
* 不混用 @param 与 Args
* 不写无意义描述
* 不描述内部实现细节
* 不重复类型

---

## 9. 严格禁止

禁止：

* 空洞描述
* 重复函数名
* 重复类型
* 描述“如何实现”
* 与行为无关的背景介绍
