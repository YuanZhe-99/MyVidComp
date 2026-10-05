# 共享界面基础

GUI 固定 MyApps-UI v0.1.6，使用全宽分段选项和大字体回退。既有选项数量、下拉
策略和转换设置留在应用。

## P5 能力与授权

GUI 固定 MyApps-UI v0.1.5，在设置中显示 myapps_ui 源码及 GNU GPL v3 声明。
仅依赖 myapps_ui，不宣称使用资料或布局包。转换布局和 Rust 引擎仍由应用负责。

## 设置控件

GUI 固定 MyApps-UI v0.1.4。ChoiceField 委托 MyAppsSettingsChoice 显示，
保留公开构造器、标签、帮助和禁用行为。应用保留 340 像素、最多三个选项的分段
策略及 AppText 目录。Rust 设置及存储仍由应用负责。抽取已完成，库的正式
概念文档替代已完成的路线图。

MyApps-UI `v0.1.2` 作为子模块放在仓库根目录的 `packages/myapps_ui`，使用
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

## P2 导航与实际空间

应用现在把导航绘制交给 `MyAppsNavigationShell`。应用导航壳保留路由、
目的地过滤、选中位置持久化和提醒回调。页面把 `context` 传入宽度和底部留白
函数，只使用一次实际内容宽度；全窗口路由不再扣除侧栏。无上下文的兼容函数
保留原计算方式。固定内容位置在缩放、风格和侧栏方向切换时保留页面状态。
MyVidComp 保留经典导航、展开侧栏和审核角标。

P3 资料抽取已完成，数据格式不变。

## P3 资料与头像

MyVidComp 固定到 P3 共享版本，不新增资料依赖或界面。
应用继续只使用主题和导航。
