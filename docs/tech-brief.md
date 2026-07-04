# PetPhrase 技术现状简报

## 产品
Windows 桌面常驻小工具:petdex 格式桌宠(雪碧图动画)静坐桌面,点击弹出常用语面板,点短语即复制。定位「轻量、美观、快」。

## 技术栈
- Tauri v2(Rust 后端 + WebView2 前端),Vite + 原生 TypeScript(无框架)
- Rust:serde JSON 存储(原子写+启动备份)、petdex 包扫描、托盘、单实例
- 插件:clipboard-manager / autostart / single-instance / dialog + window-vibrancy(DWM 亚克力)
- 测试:Rust 13 用例 + Vitest 16 用例

## 窗口架构
| 窗口 | 形态 | 生命周期 |
|------|------|------|
| pet 192×208 | 透明/置顶/无边框,雪碧图 6fps | 常驻 |
| panel 300×400 | 无边框+亚克力,失焦即隐 | 常驻隐藏(<200ms 弹出) |
| preview 240×300 | 深色全文浮层,忽略鼠标 | 按需创建,关面板即销毁 |
| settings 760×540 | 常规窗口 | 按需创建,关即销毁 |

## 指标(release 实测)
- 安装包 NSIS:2.3MB ✓;冷启动 0.8-1.0s ✓;宿主进程 15.6MB
- 全进程私有内存:默认 ~320MB;旗标优化后 ~140MB(WebView2 地板)
  - 旗标:--disable-gpu --renderer-process-limit=1 --disable-features=SpareRendererForSitePerProcess
  - 构成:浏览器进程 53 + 渲染 34 + GPU stub 15 + 工具 25 + 宿主 16

## 踩坑记录(已解)
1. Node 24 localhost→::1 与 Tauri CLI IPv4 轮询冲突;全改 127.0.0.1
2. CSS display:grid 覆盖 hidden 属性;[hidden]{display:none} 兜底
3. Windows 同步 command 主线程建窗死锁(空白+无法关闭);改 async + async_runtime::spawn
4. 设置窗点 X 销毁后无法再开;改按需创建根治
5. CSS backdrop-filter 模糊不了窗外桌面;真亚克力走 window-vibrancy
6. petdex 素材字段 displayName(非文档 name);网格 8列×9行与文档相反,按图片尺寸推算

## 待决策
- A:接受 ~140MB,旗标进配置直接发布(零成本)
- B:UI 换 Slint 重写(Rust 全复用),预期 25-45MB;代价数天重写+手接 DWM+内嵌中文字体
- C:egui/WPF/C++ 均不推荐(可爱系 UI 难做/不熟/成本极端)
