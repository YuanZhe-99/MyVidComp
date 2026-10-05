# 共享界面基础

MyApps-UI `v0.1.0` 作为子模块放在仓库根目录的 `packages/myapps_ui`，使用
相对地址 `../MyApps-UI.git`。Flutter 应用在 `gui/pubspec.yaml` 中依赖
`../packages/myapps_ui/packages/myapps_ui`。
克隆后递归初始化子模块。

`gui/lib/app_theme.dart` 重新导出 `AppUiStyle`，保留原主题包装、品牌色、
零高度卡片和紧凑输入框。`MyAppsTheme.applyStyle` 提供 Expressive 覆盖层；
Material 3 基础主题仍由应用负责。
风格存储值、默认值、导航和转换行为不变。
Rust 引擎不依赖界面包。

## 升级

先将库发布到两个远程，再更新应用指针。固定到标签
并验证 Flutter 和 Rust 检查。库文档维护公共主题声明；
本仓库描述基础主题和应用行为。
