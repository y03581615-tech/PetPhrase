# PetPhrase

Windows 桌宠常用语工具:一只 petdex 格式的桌宠静坐桌面,点击弹出常用语面板,点短语即复制到剪贴板。**单进程原生应用(Rust + Slint),常驻内存 ~23MB。**

## 功能

- **桌宠**:透明置顶,雪碧图待机动画,点击招手,可拖拽、位置记忆
- **常用语面板**:贴宠弹出;分组胶囊 Tab + 全组宫格;短句排成气泡流、长句 2 行卡片;搜索跨组过滤;点击复制 + ✓ 反馈 + 自动收起;失焦即隐
- **长文预览**:被截断的条目悬停 400ms,侧边浮出深色全文卡(不抢鼠标)
- **设置**:分组/短语增删改/排序、12 个图标、换宠、亚克力/实底主题、开机自启、导入导出 JSON
- **petdex 生态兼容**:扫描 `~/.codex/pets/`(`npx petdex install xxx` 的产物)与自定义目录,`pet.json + spritesheet.webp/png` 即插即用
- **标准安装/卸载**:NSIS 安装包,含 `uninstall.exe` 与控制面板「应用和功能」卸载入口;卸载可选保留数据

## 验收指标(release 实测,2026-07-04)

| 指标 | 目标 | 实测 |
|------|------|------|
| 安装包 | <10MB | **5.6MB** |
| 常驻内存(单进程私有) | <40MB | **22.7MB** |
| 冷启动 | <1s | **~0.3s** |
| 复制响应 | <100ms | 本地写入,无感 |

## 构建

```bash
cd app-slint
cargo test                 # 21 个单元测试
cargo build --release      # 产出 target/release/PetPhrase.exe
# 打包(NSIS,tauri CLI 缓存里自带 makensis):
%LOCALAPPDATA%/tauri/NSIS/makensis.exe installer.nsi
# 产出 target/PetPhrase_0.2.0_x64-setup.exe
```

数据文件:`%APPDATA%\PetPhrase\{phrases.json, settings.json, phrases.backup.json}`(原子写 + 启动备份自动恢复)。

## 历史

`main` 分支保留 Tauri v2 + WebView2 的完整实现(tag `v0.1.0-checkpoint`,内存 ~140-320MB),因内存目标改用 Slint 原生重写(本分支)。设计文档见 `docs/superpowers/specs/`,技术简报见 `docs/tech-brief.md`。
