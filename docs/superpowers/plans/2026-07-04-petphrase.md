# PetPhrase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Windows 桌面桌宠常用语工具:点宠 → 面板 → 点短语复制,petdex 格式换肤。

**Architecture:** Tauri v2 单进程 4 窗口(pet/panel/preview/settings)。Rust 侧三个模块:storage(phrases+settings, 原子写+备份)、pet_loader(扫描/校验 petdex 包)、lib.rs(窗口/托盘/插件装配)。前端 Vite 多页 + vanilla TS,纯函数(帧计算/长短分类/搜索/定位)全部可单测。

**Tech Stack:** Tauri 2.x, Rust (serde/serde_json), Vite + TypeScript(无框架), Vitest, 插件: clipboard-manager / autostart / single-instance / dialog, window-vibrancy crate。

## Global Constraints

- 性能验收:面板弹出 <200ms;复制 <100ms;常驻内存 <40MB;安装包 <10MB;冷启动 <1s
- 面板 300×400 逻辑像素;短句阈值 ≤10 字符且无换行;长句钳 2 行;悬停预览延迟 400ms
- petdex 格式:`pet.json` + `spritesheet.webp|png`,帧 192×208,网格由图片尺寸÷帧尺寸推算(不硬编码 8×9),行序 `idle, wave, run, failed, review, jump, extra1, extra2`,每状态 6 帧,循环 1100ms
- 数据目录 `%APPDATA%\PetPhrase\`:`phrases.json` / `settings.json` / `phrases.backup.json`
- 宠物扫描目录(序):应用资源内置 → `~/.codex/pets/` → settings.custom_pet_dir
- 主题 token 化(CSS 变量);默认 acrylic(window-vibrancy),失败自动加 `solid` class 退化实底
- 图标 SVG(Lucide 内联),不用 emoji 作功能图标;动效 150-300ms 仅 transform/opacity;对比度 4.5:1
- MVP 即全部范围,不加计划外功能

## File Structure

```
src-tauri/
  tauri.conf.json          4 窗口 + bundle(NSIS) + asset protocol scope
  capabilities/default.json 权限
  src/main.rs              入口(调 lib::run)
  src/lib.rs               装配:插件/托盘/vibrancy/commands 注册
  src/storage.rs           PhraseData/Settings 读写,原子写,备份恢复(含 #[test])
  src/pet_loader.rs        扫描+校验 petdex 包(含 #[test])
  pets/default/            内置占位宠(自产,零版权风险)
scripts/gen_default_pet.py 生成占位雪碧图(一次性)
index.html → 无(多页:pet.html/panel.html/preview.html/settings.html)
src/
  shared/types.ts          与 Rust 结构镜像
  shared/ipc.ts            invoke/事件薄封装
  shared/tokens.css        design tokens + acrylic/solid 两主题
  pet/pet.html|pet.ts|pet.css        雪碧图动画/拖拽/点击开面板
  pet/animator.ts          纯函数:帧序列/状态机(Vitest)
  panel/panel.html|panel.ts|panel.css 搜索/Tab/⊞宫格/混排列表/复制/预览触发
  panel/logic.ts           纯函数:classify/search/position(Vitest)
  preview/preview.html|preview.ts    深色全文卡(被动显示)
  settings/settings.html|settings.ts|settings.css CRUD/换宠/主题/自启/导入导出
```

---

### Task 1: 脚手架 + 4 窗口拉起

**Files:** Create 整个 Tauri 项目骨架(见上文件树);Modify `vite.config.ts`(多页)、`src-tauri/tauri.conf.json`

**Interfaces produces:** 窗口 label:`pet`(192×208 透明/置顶/无边框/skipTaskbar/可拖)、`panel`(300×400 无边框/置顶/隐藏)、`preview`(240×300 无边框/置顶/隐藏/不可聚焦)、`settings`(720×520 常规/隐藏)。

- [ ] `npm create tauri-app@latest petphrase-app -- --template vanilla-ts`,把生成内容合入项目根(或在根直接生成后整理)
- [ ] `vite.config.ts` 配 `build.rollupOptions.input` 四个 html;删默认 index.html
- [ ] `tauri.conf.json`:`app.windows` 四项;pet: `transparent/alwaysOnTop/decorations:false/skipTaskbar/shadow:false/resizable:false`;panel/preview 同透明置顶隐藏,preview 加 `focus:false`;`bundle.targets:["nsis"]`;identifier `com.petphrase.desktop`
- [ ] 每个 html 放最小占位内容,`npm run tauri dev` 验证 pet 窗出现、其余隐藏
- [ ] Commit `feat: scaffold tauri app with 4 windows`

### Task 2: storage.rs(TDD)

**Files:** Create `src-tauri/src/storage.rs`;Modify `lib.rs` 注册 commands

**Interfaces produces(前端依赖的确切签名):**
```rust
pub struct Phrase { pub id: String, pub text: String }
pub struct Group  { pub id: String, pub name: String, pub icon: Option<String>, pub phrases: Vec<Phrase> } // 顺序=Vec序
pub struct PhraseData { pub groups: Vec<Group> }
pub struct Settings { pub pet_id: String, pub theme: String, pub pet_pos: Option<(i32,i32)>,
                      pub last_group: Option<String>, pub custom_pet_dir: Option<String> } // autostart 由插件管
// commands: get_phrases()->PhraseData, save_phrases(data), get_settings()->Settings, save_settings(s),
//           export_phrases(path:String), import_phrases(path:String)->PhraseData
```
数据根:`std::env::var("APPDATA")/PetPhrase`,可测试性:所有 fn 接 `dir: &Path`,command 层再绑定真实目录。

- [ ] 写失败测试(tempfile crate):`save_then_load_roundtrip`、`load_missing_returns_default`(default 含「常用」示例组)、`corrupt_json_recovers_from_backup`、`atomic_write_leaves_no_tmp`
- [ ] `cargo test` 确认 FAIL
- [ ] 实现:load(路径不存在→default;parse 失败→读 backup 成功则覆写主文件);save(写 `.tmp` → `fs::rename`);启动时 load 成功后 copy 主→backup;import=读外部文件校验后存,export=copy
- [ ] `cargo test` PASS;Commit `feat: phrase/settings storage with atomic write and backup recovery`

### Task 3: pet_loader.rs(TDD)

**Files:** Create `src-tauri/src/pet_loader.rs`;Modify `lib.rs`

**Interfaces produces:**
```rust
pub struct PetInfo { pub id: String, pub name: String, pub spritesheet: String, // 绝对路径
                     pub error: Option<String> }
// command: list_pets(custom_dir: Option<String>) -> Vec<PetInfo>
```
校验只做:目录含 pet.json(可 parse,取 name/slug,缺则用目录名)+ spritesheet.webp|png 存在。尺寸/网格由前端加载图片时推算。`// ponytail: 尺寸校验在前端,Rust 不引 image crate`

- [ ] 失败测试:`scans_valid_pet`、`missing_spritesheet_yields_error`、`bad_json_yields_error_with_dirname_as_name`、`merges_three_dirs_in_order`(同 id 后者不覆盖前者)
- [ ] `cargo test` FAIL → 实现 → PASS
- [ ] Commit `feat: petdex package scanner with lenient validation`

### Task 4: lib.rs 装配(托盘/插件/vibrancy/asset scope)

**Files:** Modify `src-tauri/src/lib.rs`、`Cargo.toml`、`capabilities/default.json`

- [ ] 加依赖:`tauri-plugin-clipboard-manager`、`-autostart`、`-single-instance`、`-dialog`、`window-vibrancy`
- [ ] setup:对 `panel` 窗 `apply_acrylic(&win, Some((255,255,255,140)))`,Err 时 `win.emit("vibrancy-failed",())`;preview 窗 `set_ignore_cursor_events(true)`
- [ ] 托盘:菜单 显示/隐藏宠物、设置、退出;单实例回调=聚焦 pet 窗
- [ ] capabilities:各窗口 core:window 权限(show/hide/position/startDragging)、clipboard write、event、dialog
- [ ] `npm run tauri dev` 手验:托盘三项可用、面板窗手动 show 有磨砂
- [ ] Commit `feat: tray, plugins, acrylic wiring`

### Task 5: 前端 shared 层 + 主题 tokens

**Files:** Create `src/shared/types.ts`、`src/shared/ipc.ts`、`src/shared/tokens.css`

**Interfaces produces:** `ipc.getPhrases()/savePhrases()/getSettings()/saveSettings()/listPets()/copyText(t)`;事件名常量:`data-changed`、`settings-changed`、`phrase-copied`、`show-preview`、`hide-preview`、`vibrancy-failed`。tokens.css:spec §3.4 色板变量 + `body.solid` 覆盖组(实底浅色)。

- [ ] 照 Task 2/3 Rust 结构写 types;ipc 薄封装 `invoke`+`emit/listen`
- [ ] Commit `feat: shared ipc, types, theme tokens`

### Task 6: 宠物窗(animator TDD + 拖拽/点击)

**Files:** Create `src/pet/animator.ts`、`pet.ts`、`pet.html`、`pet.css`;Test `src/pet/animator.test.ts`

**Interfaces produces:**
```ts
// animator.ts 纯函数
const STATE_ROWS = { idle:0, wave:1, run:2, failed:3, review:4, jump:5 } as const;
gridFromImage(w:number,h:number,fw=192,fh=208): {rows:number,cols:number}
frameOffset(row:number,col:number,fw=192,fh=208): {x:number,y:number}  // background-position 负值
createAnimator(opts): { play(state:'idle'|'wave', once?:boolean), tick(nowMs:number):{row,col} }
// 6帧/状态、1100ms/循环;once 播完自动回 idle
```

- [ ] Vitest 失败测试:grid 推算(1536×1872→8×9 或 1728×1664→9×8 都对)、frameOffset 负偏移、wave once 播 6 帧回 idle 序列
- [ ] `npx vitest run` FAIL → 实现 → PASS
- [ ] pet.ts:convertFileSrc 加载当前宠雪碧图为 div background;setInterval(1100/6) 驱动 tick;mousedown+移动>4px → `getCurrentWindow().startDragging()`,未移动的 mouseup=单击 → wave(once)+ 通知 panel 显示;监听窗口 moved(去抖 500ms)存 pet_pos;监听 `phrase-copied` 播 wave;启动时按 settings.pet_pos 落位
- [ ] 手验:动画流畅、拖拽、单击;Commit `feat: pet window with spritesheet animator and drag`

### Task 7: 面板窗(logic TDD + 完整 UI)

**Files:** Create `src/panel/logic.ts`、`panel.ts`、`panel.html`、`panel.css`;Test `src/panel/logic.test.ts`

**Interfaces produces:**
```ts
isShort(text:string): boolean                       // ≤10字符 && 无\n
searchPhrases(data:PhraseData, q:string): {phrase:Phrase, groupName:string}[]
panelPosition(pet:{x,y,w,h}, panel:{w,h}, work:{x,y,w,h}): {x,y, side:'left'|'right'}
// side 供 preview 窗决定贴哪边
```

- [ ] Vitest 失败测试:isShort 边界(10字/11字/含换行);search 跨组+标注组名+空串返回空;panelPosition 贴边翻转与钳制
- [ ] FAIL → 实现 → PASS
- [ ] UI 按 spec §3.1-3.3:搜索框/齿轮、胶囊 Tab 横滚(wheel→scrollLeft)+右缘渐隐、⊞ 宫格覆层(≤4 组自动藏 ⊞)、混排列表(短句 chip 流/长句 2 行卡);点击条目→`copyText`→条目 300ms「✓」→200ms 后 hide 面板+emit `phrase-copied`;blur→hide;记住 last_group;监听 `data-changed` 重载;`vibrancy-failed`→body.solid
- [ ] 悬停预览:仅 scrollHeight>clientHeight 的卡,400ms 定时 emit `show-preview` {text, panelRect, side},mouseleave/Esc emit `hide-preview`
- [ ] pet 单击链路:pet.ts 计算 `panelPosition` → panel setPosition+show+setFocus
- [ ] 手验全交互;Commit `feat: phrase panel with tabs, grid overlay, mixed list, copy flow`

### Task 8: 预览窗

**Files:** Create `src/preview/preview.html`、`preview.ts`

- [ ] 深色圆角卡(#1e293b 底/浅字/12.5px/1.6 行高),listen `show-preview`:填文本、按 panelRect+side 定位(左右各让 8px)、show;`hide-preview`→hide
- [ ] 手验:悬停长句浮出、不抢焦点不挡点击、贴屏边换侧;Commit `feat: hover full-text preview window`

### Task 9: 设置窗

**Files:** Create `src/settings/settings.html`、`settings.ts`、`settings.css`

- [ ] 左侧组列表(选中高亮/新增/重命名/删除确认/HTML5 拖拽排序/图标选择=固定 12 个 Lucide);右侧该组短语列表(新增 textarea/编辑/删除/拖拽排序)
- [ ] 「外观」区:宠物卡片网格(listPets,error 宠标灰+原因,点选切换→save settings→pet 窗监听 `settings-changed` 换图);主题 acrylic/solid 切换;开机自启 toggle(autostart 插件);自定义宠物目录选择(dialog)
- [ ] 导入导出:dialog save/open + `export_phrases`/`import_phrases`,导入成功 emit `data-changed`
- [ ] 保存路径全走 `save_phrases`(每次操作即存,无「保存」按钮)
- [ ] 手验 CRUD 全链路 + 面板实时刷新;Commit `feat: settings window with CRUD, pet picker, theme, autostart`

### Task 10: 内置默认宠 + 打包 + 验收

**Files:** Create `scripts/gen_default_pet.py`、`src-tauri/pets/default/{pet.json,spritesheet.png}`

- [ ] PIL 脚本生成 1536×1872(8列×9行)占位宠:圆润橘色史莱姆,idle=上下呼吸浮动 6 帧,wave=左右摆 6 帧,其余行复用 idle;`pip install pillow` 若缺
- [ ] pet.json:`{"name":"Mochi","slug":"default"}`;bundle resources 打进安装包;pet_loader 内置目录=resource dir
- [ ] `npm run tauri build`(NSIS);装机手验清单:托盘/拖拽/复制/换组/搜索/预览/换宠/导入导出/自启/透明效果关闭降级/150% DPI
- [ ] 实测四指标(任务管理器内存、安装包体积、启动秒表、弹出手感),记录进 README
- [ ] Commit `feat: bundled default pet, NSIS packaging`;`chore: acceptance results`

## Self-Review

- Spec 覆盖:§1指标→T10;§2交互流→T6/T7;§3.1→T7;§3.2→T7;§3.3→T7+T8;§3.4→T4/T5/T7;§4/§5→T1-T5;§6 错误处理→T2(备份恢复)/T3(宠物标灰)/T7(复制失败重试文案在 copy catch 中)/T4(vibrancy 降级);§7 测试→T2/T3/T6/T7 单测+T10 手验清单 ✓
- 偏差记录:尺寸校验从 Rust 移到前端(避免 image crate,理由见 T3);全文预览由「面板外浮层」实现为独立 preview 窗(WebView 画不出窗界)✓ 已在会话中向用户说明
- 类型一致性:PetInfo/PhraseData/Settings/事件名在 T2/T3/T5 定义,T6-T9 仅引用 ✓
