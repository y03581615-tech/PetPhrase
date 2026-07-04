# PetPhrase

Windows 桌宠常用语工具:petdex 格式桌宠 + 点击弹出常用语面板 + 点短语复制。

## 分支与版本

- **`slint-rewrite`(当前主开发线)**:Rust + Slint 单进程原生版,~23MB 常驻。产品代码在 `app-slint/`
- `main`:旧 Tauri v2 + WebView2 版(tag `v0.1.0-checkpoint`),因内存(140-320MB)被 Slint 版取代,仅存档
- `src-tauri/`、`src/`、`*.html` 均属旧版,改 bug 只改 `app-slint/`

## 常用命令(都在 app-slint/ 下执行)

```bash
cargo test                # 21 个单元测试(storage/pet_loader/logic/anim)
cargo build --release     # 产出 target/release/PetPhrase.exe
%LOCALAPPDATA%/tauri/NSIS/makensis.exe installer.nsi   # NSIS 安装包(含 uninstall.exe)
./target/PetPhrase_0.2.0_x64-setup.exe //S             # 静默安装到 %LOCALAPPDATA%\Programs\PetPhrase
```

installer.nsi 必须保持 **UTF-8 带 BOM**,否则 makensis 报 Bad text encoding。

## 架构(app-slint/)

- `src/main.rs` — 全部装配:窗口接线、托盘(tray-icon+定时轮询)、剪贴板(arboard)、自启(auto-launch)、单实例、文件对话框(rfd)
- `src/storage.rs` — phrases/settings JSON,原子写+启动备份恢复;数据在 `%APPDATA%\PetPhrase\`
- `src/pet_loader.rs` — petdex 包扫描(exe/pets → ~/.codex/pets → 自定义目录),宽松校验
- `src/logic.rs` — **布局全在 Rust 算**(气泡流换行/卡片/贴宠定位),Slint 只按 x/y/w/h 绝对定位渲染;文本宽度是估算(CJK=字号,ASCII=0.55×)
- `src/anim.rs` — 雪碧图帧驱动;网格按图片尺寸推算,**不硬编码 8×9**(petdex 文档与实际素材相反);行序 idle,wave,run,failed,review,jump
- `ui/*.slint` — pet(透明桌宠)/ panel(亚克力面板)/ settings(常规窗)+ common(tokens/通用控件)

## 关键坑(改动前必读)

1. **窗口 show/hide 会引发激活/焦点事件链**。面板「失焦即隐」+ 任何新窗口弹出 = 面板自隐 bug。预览因此做成**面板内浮层**而非独立窗口,别改回去
2. 软件渲染器(默认,为了 ~15MB)最怕:每帧重算 drop-shadow、opacity 图层、大区域重绘。拖拽排序用「插入指示线+幽灵条」而非整行跟手,就是为此
3. petdex 真实素材字段是 `displayName`(非文档的 name);雪碧图 1536×1872 = 8列×9行
4. Windows 上同步上下文创建窗口会死锁(旧版教训);Slint 单线程事件循环没这问题,但 `slint::Timer` 被 drop 即停,常驻 timer 用 `Box::leak`
5. Slint 多窗口时 `global<Theme>` 是**每组件实例一份**,set_theme 必须逐窗口设置
6. 设置窗列表行是固定行高 + 单行 display 文本(`\n` 折叠);直接塞多行原文会溢出重叠
7. 中文必须显式 `default-font-family: "Microsoft YaHei UI"`,默认字体回退会丢字形
8. tsc/村规里的 web 规则不适用本分支;Rust 规则适用

## 验收基线(release 实测)

安装包 5.6MB / 常驻 ~23MB / 冷启动 ~0.3s。改动后若明显劣化需说明原因。
