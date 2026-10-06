# 共享 UI 基础

MyVidComp 使用 `packages/myapps_ui` 中的 MyApps-UI v0.1.7。Flutter 依赖路径为
`../packages/myapps_ui/packages/myapps_ui`；克隆后需递归初始化子模块。
Rust 引擎不依赖 UI 包。

## 主题与导航

`AppTheme.build` 向 `MyAppsTheme.build` 传入蓝色品牌种子。
卡片、输入框、文字和 Expressive 细节采用共享默认值。已有风格名称和默认
Expressive 风格保持兼容。
`HomeShell.build` 向 `MyAppsNavigationShell` 传入当前风格，采用共享
Expressive 浮动底栏和默认紧凑侧栏尺寸。窗口宽度达到 600 像素时使用侧栏；
导航目标、复核角标和页面状态仍由应用管理。

## 设置与选项

`SettingsPage.build` 使用 `MyAppsSettingsSection`，并为外观、风格和语言采用完整的
`MyAppsSettingsSegmentRow` 控件。
实际内容宽度达到 900 像素时，`MyAppsPaneBody` 将常用设置与高级控件分开，
两侧最小宽度均为 400 像素。
窄屏在主滚动列表中包含高级控件。稳定的键在布局变化时保留文本输入和展开状态。
`ChoiceField.build` 保留应用接口，并委托 `MyAppsSettingsChoice`，
取消按宽度或选项数量切换下拉框的限制。共享分段标签最多换行两行，
必要时回退为纵向选项，包括放大字体的情况。
`SectionCard.build` 在采用主题样式的卡片中使用共享分组标题。
`LabelledField.build` 继承共享输入框密度和边框样式。
可用宽度低于 480 像素乘以文字缩放比例时，尾部操作移到输入框下方，
确保桌面端较长的文件夹操作文字完整显示。
`CountTile.build` 使用主题卡片形状和语义表面颜色。
`PageBody.build` 提供可读的滚动宽度和统一的 16 像素页面间距。
工具状态、复核卡片和进度日志采用相同的 16 像素内容间距。
指定编码器以本地化的只读设置行展示。
转换设置、本地化、持久化和复核语义仍由应用管理。

## 归属与更新

设置页展示 myapps_ui 的源代码地址和 GNU GPL v3 声明。无需个人资料依赖。
升级共享库时，先发布到两个远端，再在此固定带标签的提交。
验证 Flutter 和 Rust 检查，并同步更新两种语言的文档。
