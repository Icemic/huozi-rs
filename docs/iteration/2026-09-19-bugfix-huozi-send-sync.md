# 2026-09-19 恢复 Huozi 的 Send 与 Sync

> 状态：已完成
>
> 分类：bugfix

## 目标

恢复 `Huozi` 的 `Send + Sync` 能力。引入 tiqian 字体后端后，`Huozi` 持有的
`ParagraphLayoutEngine` 内部 trait object 缺少线程安全约束，导致 Rust 无法自动推导
`Huozi` 的线程安全实现。

## 范围

- 将 tiqian 段落引擎注入依赖的线程安全要求写入类型定义；
- 为 `Huozi` 添加编译期 `Send + Sync` 回归测试；
- 更新架构文档中的并发边界说明。

不改变字体选择、shaping、段落布局、SDF 图集或缓存淘汰行为；不使用 `unsafe impl`，也不在
`Huozi` 内部新增锁。

## 设计

`Huozi` 的字体记录、图集和缓存已由可自动推导线程安全的字段组成。问题仅来自 tiqian 段落引擎
保存的可注入依赖。由 tiqian 在 `FontBackend`、profile resolver、metrics normalizer、font role
classifier、line breaker、kinsoku rule、hyphenator 和 annotation cache 的 trait 边界要求
`Send + Sync`，使引擎和 `Huozi` 自动获得同一能力。

`Huozi` 的布局和图集更新仍需 `&mut self`。`Send + Sync` 只允许实例跨线程转移或放入外部同步容器；
同一实例的并发可变访问仍由调用方负责串行化。

## 验证与回滚

- 运行 `cargo test huozi_is_send_and_sync`；
- 运行完整 Huozi 测试及 `git diff --check`；
- 编译期断言失败时，回退本迭代，不通过 `unsafe impl` 绕过依赖的线程安全问题。

## 完成记录

- `cargo test huozi_is_send_and_sync` 通过；
- `cargo test` 通过：58 个库测试、13 个集成测试与 3 个 doctest 通过，1 个 doctest 按既有配置忽略；
- `git diff --check` 与本次中文文档风格检查通过；
- `Huozi` 的 `Send + Sync` 由编译器自动推导，未使用不安全实现。
