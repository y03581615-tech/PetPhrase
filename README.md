# PetPhrase

一只桌宠 + 你的常用语面板。桌宠([petdex](https://petdex.dev/) 格式)静坐在桌面上,点它弹出常用语面板,点一条短语立即复制到剪贴板——客服回复、报销话术、代码片段,一次点击不打断手头工作。

**Rust + [Slint](https://slint.dev/) 单进程原生应用**:安装包 5.6MB,常驻内存 ~23MB,冷启动 ~0.3s。

> **平台支持:仅 Windows 10/11 x64。** 暂不支持 macOS 与 Linux。

## 功能

- **桌宠**:透明置顶、雪碧图待机动画,点击招手,可拖拽并记住位置
- **常用语面板**:贴宠弹出;分组胶囊 Tab + 全组宫格;短句排成气泡流、长句卡片;搜索跨组过滤;点击复制 + ✓ 反馈 + 自动收起;失焦即隐
- **设置**:分组/短语增删改、长按拖动排序、12 个分组图标、换宠、亚克力/实底主题、开机自启、导入导出 JSON
- **petdex 生态兼容**:自动扫描 `~/.codex/pets/`——用 [petdex](https://github.com/crafter-station/petdex) 安装的桌宠即插即用:

  ```bash
  npx petdex@latest install <pet-name>
  ```

  也支持在设置里指定自定义目录,`pet.json + spritesheet.webp/png` 即为一只宠
- **标准安装/卸载**:NSIS 安装包(装到 `%LOCALAPPDATA%\Programs\PetPhrase`,无需管理员权限),含 `uninstall.exe` 与「应用和功能」卸载入口,卸载时可选保留数据

## 安装

从 [Releases](../../releases) 下载 `PetPhrase_x.y.z_x64-setup.exe`,双击安装。

数据存放于 `%APPDATA%\PetPhrase\`(原子写入 + 启动备份自动恢复),卸载重装不丢数据。

## 从源码构建

需要 Rust stable 工具链(MSVC)。

```bash
cd app-slint
cargo test                # 21 个单元测试
cargo build --release     # 产出 target/release/PetPhrase.exe
# 打包安装程序(需 NSIS makensis):
makensis installer.nsi    # 产出 target/PetPhrase_0.2.0_x64-setup.exe
```

## 致谢

- 桌宠格式与生态来自 [petdex](https://petdex.dev/)([crafter-station/petdex](https://github.com/crafter-station/petdex))
- UI 框架 [Slint](https://slint.dev/)
- 图标 [Lucide](https://lucide.dev/)

## 许可

[MIT](LICENSE)
