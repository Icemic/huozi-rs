# SDF 基础边缘平滑

## 目标与范围

保留已人工验证的 fill 阈值 `0.735357`，恢复轻微的基础边缘平滑，减轻额外 gamma 归零后的毛刺。只调整文字顶点参数及官方 render 示例 shader。

## 设计

- fill 阈值为 `1.0 - CUTOFF - FILL_THRESHOLD_BIAS`。补偿量为 `0.014643`，单位为归一化 SDF 值，保持当前 `0.735357` 的阈值效果；正值使笔画外扩。
- `EDGE_SMOOTHING_HALF_WIDTH` 表示额外平滑半宽，单位为逻辑像素，暂定为 `0.30`；完整过渡带额外增加 `0.60` 逻辑像素。
- 换算系数 `m = x_scale / (RADIUS * scale_ratio)`；fill 和 stroke 的顶点 gamma 为 `0.30 * m`，shadow 为 `(0.30 + blur) * m`。
- 保留 shader 的导数抗锯齿，最终半宽为导数半宽加顶点 gamma。
- stroke 内外缘使用同一最终半宽；fill 和 shadow 的 `fill_buffer = 2.0` 表示无内缘，不计算内缘 coverage。
- 不改变描边宽度、阴影 spread、offset、仿粗或顶点 ABI。外部 renderer 需要同步内缘计算规则。

## 验证与回滚

运行库测试及 render 示例编译检查。人工在 24 逻辑字号比较宋体细横、斜线和曲线，并检查描边及阴影；`0.30` 为用户暂定的校准值。

将常量设为 `0.0` 可关闭额外平滑。完整回滚只撤销本次平滑参数与 shader 改动，保留用户已调整的 fill 阈值。