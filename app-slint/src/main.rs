//! PetPhrase Slint 原生版 —— 单进程装配:
//! 宠物/面板/预览/设置四窗口、托盘、剪贴板、自启、单实例。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod anim;
mod logic;
mod pet_loader;
mod storage;

use anim::{Animator, PetState};
use logic::LaidItem;
use pet_loader::PetInfo;
use slint::winit_030::{winit, WinitWindowAccessor};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

slint::include_modules!();

/// 分组图标固定集,顺序与 common.slint GroupIcon 分支一致
const ICON_KEYS: [&str; 12] = [
    "star", "briefcase", "headphones", "code", "mail", "message-circle", "smile", "heart",
    "fish", "map-pin", "credit-card", "folder",
];

fn icon_idx(icon: &Option<String>) -> i32 {
    icon.as_deref()
        .and_then(|k| ICON_KEYS.iter().position(|x| *x == k))
        .unwrap_or(11) as i32
}

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").expect("APPDATA 环境变量缺失")).join("PetPhrase")
}

fn pet_roots(custom: &Option<String>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.join("pets"));
        }
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        roots.push(PathBuf::from(home).join(".codex").join("pets"));
    }
    if let Some(c) = custom {
        roots.push(PathBuf::from(c));
    }
    roots
}

struct State {
    data: storage::PhraseData,
    settings: storage::Settings,
    pets: Vec<PetInfo>,
    active_group: usize,
    items: Vec<LaidItem>,
    animator: Animator,
    clipboard: Option<arboard::Clipboard>,
    thumb_cache: HashMap<String, slint::Image>,
    panel_native_ready: bool,
    /// show 后是否真正拿到过焦点 —— 防初始 Focused(false) 误隐藏
    panel_got_focus: bool,
}

struct App {
    pet: PetWindow,
    panel: PanelWindow,
    settings_win: SettingsWindow,
    state: RefCell<State>,
    hover_timer: slint::Timer,
    hide_timer: slint::Timer,
    move_timer: slint::Timer,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = single_instance::SingleInstance::new("petphrase-slint")?;
    if !instance.is_single() {
        return Ok(());
    }

    // 软件渲染:实测常驻 ~15MB(GPU 渲染 ~78MB),雪碧图 6fps 动画绰绰有余
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .with_winit_window_attributes_hook(|attrs| attrs.with_transparent(true))
        .select()?;

    let dir = data_dir();
    storage::backup_phrases(&dir);
    let data = storage::load_phrases(&dir);
    let settings = storage::load_settings(&dir);
    let roots = pet_roots(&settings.custom_pet_dir);
    let refs: Vec<&std::path::Path> = roots.iter().map(|p| p.as_path()).collect();
    let pets = pet_loader::scan_pets(&refs);

    let active_group = settings
        .last_group
        .as_ref()
        .and_then(|id| data.groups.iter().position(|g| &g.id == id))
        .unwrap_or(0);

    let app = Rc::new(App {
        pet: PetWindow::new()?,
        panel: PanelWindow::new()?,
        settings_win: SettingsWindow::new()?,
        state: RefCell::new(State {
            data,
            settings,
            pets,
            active_group,
            items: Vec::new(),
            animator: Animator::new(1, 1),
            clipboard: arboard::Clipboard::new().ok(),
            thumb_cache: HashMap::new(),
            panel_native_ready: false,
            panel_got_focus: false,
        }),
        hover_timer: slint::Timer::default(),
        hide_timer: slint::Timer::default(),
        move_timer: slint::Timer::default(),
    });

    let solid = app.state.borrow().settings.theme == "solid";
    set_theme(&app, solid);

    wire_pet(&app);
    wire_panel(&app);
    wire_settings(&app);
    setup_frame_timer(&app);
    let _tray = setup_tray(&app)?;

    refresh_pet_sprite(&app);
    refresh_panel(&app);
    // refresh_settings 延迟到设置窗打开时:缩略图解码占 10+MB/宠,不该常驻

    // 设置窗关闭 → 释放缩略图缓存与模型
    {
        let a = app.clone();
        app.settings_win.window().on_close_requested(move || {
            a.state.borrow_mut().thumb_cache.clear();
            a.settings_win.set_pets(ModelRc::new(VecModel::from(Vec::<PetCardUi>::new())));
            slint::CloseRequestResponse::HideWindow
        });
    }

    app.pet.show()?;
    // 落位 + 原生属性(跳任务栏)
    {
        let st = app.state.borrow();
        if let Some((x, y)) = st.settings.pet_pos {
            app.pet.window().set_position(slint::PhysicalPosition::new(x, y));
        }
    }
    app.pet.window().with_winit_window(|w: &winit::window::Window| {
        use winit::platform::windows::WindowExtWindows;
        w.set_skip_taskbar(true);
    });

    slint::run_event_loop_until_quit()?;
    Ok(())
}

fn set_theme(app: &Rc<App>, solid: bool) {
    app.panel.global::<Theme>().set_solid(solid);
    app.settings_win.global::<Theme>().set_solid(solid);
    app.settings_win.set_solid_theme(solid);
}

/* ================= 宠物窗 ================= */

fn wire_pet(app: &Rc<App>) {
    let a = app.clone();
    app.pet.on_pet_clicked(move || {
        dbg_log("pet clicked");
        a.state.borrow_mut().animator.play(PetState::Wave, true);
        toggle_panel(&a);
    });

    let a = app.clone();
    app.pet.on_drag_start(move || {
        a.pet.window().with_winit_window(|w: &winit::window::Window| {
            let _ = w.drag_window();
        });
    });

    // 拖完保存位置(去抖 500ms)
    let a = app.clone();
    app.pet.window().on_winit_window_event(move |_, event| {
        if let winit::event::WindowEvent::Moved(pos) = event {
            let (x, y) = (pos.x, pos.y);
            let a2 = a.clone();
            a.move_timer.start(
                slint::TimerMode::SingleShot,
                Duration::from_millis(500),
                move || {
                    let mut st = a2.state.borrow_mut();
                    st.settings.pet_pos = Some((x, y));
                    let _ = storage::save_settings(&data_dir(), &st.settings);
                },
            );
        }
        slint::winit_030::EventResult::Propagate
    });
}

fn refresh_pet_sprite(app: &Rc<App>) {
    let (sheet_path, ok) = {
        let st = app.state.borrow();
        let pet = st
            .pets
            .iter()
            .find(|p| p.id == st.settings.pet_id && p.error.is_none())
            .or_else(|| st.pets.iter().find(|p| p.error.is_none()));
        match pet {
            Some(p) => (p.spritesheet.clone(), true),
            None => (String::new(), false),
        }
    };
    if !ok {
        return;
    }
    if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&sheet_path)) {
        let size = img.size();
        let (rows, cols) = anim::grid_from_image(size.width, size.height);
        app.state.borrow_mut().animator = Animator::new(rows, cols);
        app.pet.set_sheet(img);
    }
}

fn setup_frame_timer(app: &Rc<App>) {
    let a = app.clone();
    let timer = Box::leak(Box::new(slint::Timer::default()));
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(anim::FRAME_MS),
        move || {
            let (row, col) = a.state.borrow_mut().animator.step();
            a.pet.set_frame_row(row);
            a.pet.set_frame_col(col);
        },
    );
}

/* ================= 面板窗 ================= */

const LIST_AVAIL_W: f32 = logic::PANEL_W - logic::LIST_PAD * 2.0;

fn refresh_panel(app: &Rc<App>) {
    let (tabs, items, content_h, show_grid) = {
        let st = app.state.borrow();
        let query = app.panel.get_search_text().to_string();
        let items = if query.trim().is_empty() {
            logic::layout_group(&st.data, st.active_group, LIST_AVAIL_W)
        } else {
            logic::layout_search(&st.data, &query, LIST_AVAIL_W)
        };
        let tabs: Vec<TabUi> = st
            .data
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| TabUi {
                id: g.id.clone().into(),
                name: g.name.clone().into(),
                icon_idx: icon_idx(&g.icon),
                active: i == st.active_group,
            })
            .collect();
        let h = logic::content_height(&items);
        let show_grid = st.data.groups.len() > 4;
        (tabs, items, h, show_grid)
    };

    let ui_items: Vec<PanelItemUi> = items
        .iter()
        .map(|it| PanelItemUi {
            text: it.text.clone().into(),
            badge: it.badge.clone().into(),
            x: it.x,
            y: it.y,
            w: it.w,
            h: it.h,
            is_chip: it.is_chip,
            truncated: it.truncated,
        })
        .collect();

    app.state.borrow_mut().items = items;
    app.panel.set_tabs(ModelRc::new(VecModel::from(tabs)));
    app.panel.set_items(ModelRc::new(VecModel::from(ui_items)));
    app.panel.set_content_h(content_h);
    app.panel.set_show_grid_btn(show_grid);
    app.panel.set_copied_idx(-1);
    app.panel.set_failed_idx(-1);
}

fn wire_panel(app: &Rc<App>) {
    let a = app.clone();
    app.panel.on_tab_clicked(move |i| {
        select_group(&a, i as usize);
    });

    let a = app.clone();
    app.panel.on_search_changed(move |_| refresh_panel(&a));

    let a = app.clone();
    app.panel.on_item_clicked(move |i| copy_item(&a, i));

    let a = app.clone();
    app.panel.on_gear_clicked(move || {
        open_settings(&a);
        hide_panel(&a);
    });

    let a = app.clone();
    app.panel.on_escape_pressed(move || hide_panel(&a));

    // 悬停 400ms → 面板内全文预览浮层
    let a = app.clone();
    app.panel.on_item_hovered(move |i| {
        let text = match a.state.borrow().items.get(i as usize) {
            Some(it) if it.truncated => it.text.clone(),
            _ => return,
        };
        let a2 = a.clone();
        a.hover_timer.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(400),
            move || {
                a2.panel.set_preview_text(text.clone().into());
                a2.panel.set_preview_visible(true);
            },
        );
    });

    let a = app.clone();
    app.panel.on_item_unhovered(move || a.hover_timer.stop());

    // 预览浮层单击 = 复制全文并收面板
    let a = app.clone();
    app.panel.on_preview_clicked(move || {
        let text = a.panel.get_preview_text().to_string();
        let ok = {
            let mut st = a.state.borrow_mut();
            match st.clipboard.as_mut() {
                Some(cb) => cb.set_text(text).is_ok(),
                None => false,
            }
        };
        if ok {
            a.state.borrow_mut().animator.play(PetState::Wave, true);
            hide_panel(&a);
        }
    });
}

fn select_group(app: &Rc<App>, idx: usize) {
    {
        let mut st = app.state.borrow_mut();
        if idx >= st.data.groups.len() {
            return;
        }
        st.active_group = idx;
        st.settings.last_group = Some(st.data.groups[idx].id.clone());
        let _ = storage::save_settings(&data_dir(), &st.settings);
    }
    app.panel.set_search_text("".into());
    refresh_panel(app);
    refresh_settings(app);
}

fn copy_item(app: &Rc<App>, i: i32) {
    let text = match app.state.borrow().items.get(i as usize) {
        Some(it) => it.text.clone(),
        None => return,
    };
    let ok = {
        let mut st = app.state.borrow_mut();
        match st.clipboard.as_mut() {
            Some(cb) => cb.set_text(text).is_ok(),
            None => false,
        }
    };
    if ok {
        app.panel.set_copied_idx(i);
        app.state.borrow_mut().animator.play(PetState::Wave, true);
        let a = app.clone();
        app.hide_timer.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(200),
            move || hide_panel(&a),
        );
    } else {
        app.panel.set_failed_idx(i);
    }
}

fn ensure_panel_native(app: &Rc<App>) {
    if app.state.borrow().panel_native_ready {
        return;
    }
    let solid_pref = app.state.borrow().settings.theme == "solid";
    let mut acrylic_ok = false;
    app.panel.window().with_winit_window(|w: &winit::window::Window| {
        use winit::platform::windows::WindowExtWindows;
        w.set_skip_taskbar(true);
        acrylic_ok = window_vibrancy::apply_acrylic(w, Some((255, 255, 255, 170))).is_ok();
    });
    // acrylic 失败或用户选实底 → solid
    set_theme(app, solid_pref || !acrylic_ok);
    app.state.borrow_mut().panel_native_ready = true;
}

fn toggle_panel(app: &Rc<App>) {
    if app.panel.window().is_visible() {
        hide_panel(app);
        return;
    }
    // 贴宠定位(物理像素)
    let scale = app.pet.window().scale_factor();
    let placement = app.pet.window().with_winit_window(|w: &winit::window::Window| {
        let pos = w.outer_position().unwrap_or_default();
        let size = w.outer_size();
        let (mx, my, mw, mh) = match w.current_monitor() {
            Some(m) => {
                let p = m.position();
                let s = m.size();
                (p.x as f32, p.y as f32, s.width as f32, s.height as f32)
            }
            None => (0.0, 0.0, 1920.0, 1080.0),
        };
        logic::panel_position(
            logic::Rect { x: pos.x as f32, y: pos.y as f32, w: size.width as f32, h: size.height as f32 },
            logic::PANEL_W * scale,
            logic::PANEL_H * scale,
            logic::Rect { x: mx, y: my, w: mw, h: mh },
        )
    });
    let Some(placement) = placement else {
        dbg_log("toggle_panel: no placement (pet native window missing?)");
        return;
    };
    app.state.borrow_mut().panel_got_focus = false;

    app.panel.set_search_text("".into());
    refresh_panel(app);
    dbg_log(&format!("toggle_panel: show at {},{}", placement.x, placement.y));
    if let Err(e) = app.panel.show() {
        dbg_log(&format!("panel.show err: {e}"));
        return;
    }
    app.panel
        .window()
        .set_position(slint::PhysicalPosition::new(placement.x as i32, placement.y as i32));
    ensure_panel_native(app);

    // 失焦即隐:仅在拿到过焦点后才生效,防 show 初期的 Focused(false)
    let a = app.clone();
    app.panel.window().on_winit_window_event(move |_, event| {
        match event {
            winit::event::WindowEvent::Focused(true) => {
                a.state.borrow_mut().panel_got_focus = true;
            }
            winit::event::WindowEvent::Focused(false) => {
                if a.state.borrow().panel_got_focus {
                    hide_panel(&a);
                }
            }
            _ => {}
        }
        slint::winit_030::EventResult::Propagate
    });
    app.panel.window().with_winit_window(|w: &winit::window::Window| {
        w.focus_window();
    });
}

fn dbg_log(msg: &str) {
    #[cfg(debug_assertions)]
    eprintln!("[petphrase] {msg}");
    #[cfg(not(debug_assertions))]
    let _ = msg;
}

fn hide_panel(app: &Rc<App>) {
    app.hover_timer.stop();
    app.panel.set_grid_open(false);
    app.panel.set_preview_visible(false);
    let _ = app.panel.window().hide();
}

/* ================= 设置窗 ================= */

fn open_settings(app: &Rc<App>) {
    refresh_pets(app);
    refresh_settings(app);
    let _ = app.settings_win.show();
    app.settings_win.window().with_winit_window(|w: &winit::window::Window| {
        w.focus_window();
    });
}

fn refresh_pets(app: &Rc<App>) {
    let mut st = app.state.borrow_mut();
    let roots = pet_roots(&st.settings.custom_pet_dir);
    let refs: Vec<&std::path::Path> = roots.iter().map(|p| p.as_path()).collect();
    st.pets = pet_loader::scan_pets(&refs);
}

fn refresh_settings(app: &Rc<App>) {
    let editing = app.settings_win.get_editing_idx();
    let mut st = app.state.borrow_mut();

    let groups: Vec<GroupRowUi> = st
        .data
        .groups
        .iter()
        .enumerate()
        .map(|(i, g)| GroupRowUi {
            name: g.name.clone().into(),
            icon_idx: icon_idx(&g.icon),
            count: g.phrases.len() as i32,
            selected: i == st.active_group,
        })
        .collect();

    let active = st.active_group.min(st.data.groups.len().saturating_sub(1));
    st.active_group = active;
    let (name, gicon, phrases): (SharedString, i32, Vec<PhraseRowUi>) = match st.data.groups.get(active) {
        Some(g) => (
            g.name.clone().into(),
            icon_idx(&g.icon),
            g.phrases
                .iter()
                .enumerate()
                .map(|(i, p)| PhraseRowUi {
                    text: p.text.clone().into(),
                    display: p.text.replace('\n', " ").into(),
                    editing: i as i32 == editing,
                })
                .collect(),
        ),
        None => ("".into(), 11, Vec::new()),
    };

    // 宠物卡(缩略图缓存)
    let pets = st.pets.clone();
    let selected_pet = st.settings.pet_id.clone();
    let mut cards = Vec::new();
    for p in &pets {
        let thumb = if p.error.is_none() {
            let cache = &mut st.thumb_cache;
            cache
                .entry(p.spritesheet.clone())
                .or_insert_with(|| {
                    slint::Image::load_from_path(std::path::Path::new(&p.spritesheet))
                        .unwrap_or_default()
                })
                .clone()
        } else {
            slint::Image::default()
        };
        cards.push(PetCardUi {
            name: p.name.clone().into(),
            err: p.error.clone().unwrap_or_default().into(),
            selected: p.id == selected_pet,
            thumb,
        });
    }

    let custom_dir: SharedString = st.settings.custom_pet_dir.clone().unwrap_or_default().into();
    let has_group = !st.data.groups.is_empty();
    drop(st);

    app.settings_win.set_groups(ModelRc::new(VecModel::from(groups)));
    app.settings_win.set_phrases(ModelRc::new(VecModel::from(phrases)));
    app.settings_win.set_pets(ModelRc::new(VecModel::from(cards)));
    app.settings_win.set_group_name(name);
    app.settings_win.set_group_icon_idx(gicon);
    app.settings_win.set_has_group(has_group);
    app.settings_win.set_custom_dir(custom_dir);
}

fn persist_data(app: &Rc<App>) {
    let st = app.state.borrow();
    let _ = storage::save_phrases(&data_dir(), &st.data);
}

fn uid() -> String {
    // 时间戳+计数足够本地唯一,免拉 uuid 依赖
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("p-{t:x}-{}", N.fetch_add(1, Ordering::Relaxed))
}

fn wire_settings(app: &Rc<App>) {
    let a = app.clone();
    app.settings_win.on_group_selected(move |i| select_group(&a, i as usize));

    let a = app.clone();
    app.settings_win.on_group_add(move || {
        {
            let mut st = a.state.borrow_mut();
            st.data.groups.push(storage::Group {
                id: uid(),
                name: "新分组".into(),
                icon: Some("folder".into()),
                phrases: Vec::new(),
            });
            st.active_group = st.data.groups.len() - 1;
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_group_renamed(move |name| {
        let name = name.trim().to_string();
        if name.is_empty() {
            refresh_settings(&a);
            return;
        }
        {
            let mut st = a.state.borrow_mut();
            let idx = st.active_group;
            if let Some(g) = st.data.groups.get_mut(idx) {
                g.name = name;
            }
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_group_icon_set(move |i| {
        {
            let mut st = a.state.borrow_mut();
            let idx = st.active_group;
            if let Some(g) = st.data.groups.get_mut(idx) {
                g.icon = Some(ICON_KEYS[i.clamp(0, 11) as usize].to_string());
            }
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    // 删除分组走应用内确认框
    let a = app.clone();
    app.settings_win.on_group_delete(move || {
        let (title, msg) = {
            let st = a.state.borrow();
            match st.data.groups.get(st.active_group) {
                Some(g) => (
                    "删除分组".to_string(),
                    format!("将删除「{}」及其中 {} 条常用语,此操作不可撤销。", g.name, g.phrases.len()),
                ),
                None => return,
            }
        };
        a.settings_win.set_confirm_title(title.into());
        a.settings_win.set_confirm_msg(msg.into());
        a.settings_win.set_confirm_visible(true);
    });

    let a = app.clone();
    app.settings_win.on_confirm_ok(move || {
        {
            let mut st = a.state.borrow_mut();
            let idx = st.active_group;
            if idx < st.data.groups.len() {
                st.data.groups.remove(idx);
            }
            st.active_group = 0;
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_group_dropped(move |from, to| {
        {
            let mut st = a.state.borrow_mut();
            let len = st.data.groups.len() as i32;
            let from = from.clamp(0, len - 1) as usize;
            let to = to.clamp(0, len - 1) as usize;
            if from == to {
                return;
            }
            let g = st.data.groups.remove(from);
            st.data.groups.insert(to, g);
            st.active_group = to;
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_phrase_add(move |text| {
        let text = text.trim().to_string();
        if text.is_empty() {
            return;
        }
        {
            let mut st = a.state.borrow_mut();
            let idx = st.active_group;
            if let Some(g) = st.data.groups.get_mut(idx) {
                g.phrases.push(storage::Phrase { id: uid(), text });
            }
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_phrase_edited(move |i, text| {
        let text = text.trim().to_string();
        {
            let mut st = a.state.borrow_mut();
            let idx = st.active_group;
            if let Some(p) = st.data.groups.get_mut(idx).and_then(|g| g.phrases.get_mut(i as usize)) {
                if !text.is_empty() {
                    p.text = text;
                }
            }
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_phrase_delete(move |i| {
        {
            let mut st = a.state.borrow_mut();
            let idx = st.active_group;
            if let Some(g) = st.data.groups.get_mut(idx) {
                if (i as usize) < g.phrases.len() {
                    g.phrases.remove(i as usize);
                }
            }
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_phrase_dropped(move |from, to| {
        {
            let mut st = a.state.borrow_mut();
            let idx = st.active_group;
            let Some(g) = st.data.groups.get_mut(idx) else { return };
            let len = g.phrases.len() as i32;
            if len == 0 {
                return;
            }
            let from = from.clamp(0, len - 1) as usize;
            let to = to.clamp(0, len - 1) as usize;
            if from == to {
                return;
            }
            let p = g.phrases.remove(from);
            g.phrases.insert(to, p);
        }
        persist_data(&a);
        refresh_settings(&a);
        refresh_panel(&a);
    });

    let a = app.clone();
    app.settings_win.on_pet_selected(move |i| {
        {
            let mut st = a.state.borrow_mut();
            let Some(p) = st.pets.get(i as usize) else { return };
            if p.error.is_some() {
                return;
            }
            st.settings.pet_id = p.id.clone();
            let _ = storage::save_settings(&data_dir(), &st.settings);
        }
        refresh_pet_sprite(&a);
        refresh_settings(&a);
    });

    let a = app.clone();
    app.settings_win.on_theme_toggled(move |solid| {
        {
            let mut st = a.state.borrow_mut();
            st.settings.theme = if solid { "solid".into() } else { "acrylic".into() };
            let _ = storage::save_settings(&data_dir(), &st.settings);
        }
        set_theme(&a, solid);
    });

    let a = app.clone();
    app.settings_win.on_autostart_toggled(move |on| {
        let result = autostart_handle().and_then(|al| {
            if on {
                al.enable().map_err(|e| e.to_string())
            } else {
                al.disable().map_err(|e| e.to_string())
            }
        });
        if result.is_err() {
            a.settings_win.set_autostart_on(!on);
        }
    });

    let a = app.clone();
    app.settings_win.on_pick_dir(move || {
        if let Some(dir) = rfd::FileDialog::new().set_title("选择宠物目录").pick_folder() {
            {
                let mut st = a.state.borrow_mut();
                st.settings.custom_pet_dir = Some(dir.to_string_lossy().to_string());
                let _ = storage::save_settings(&data_dir(), &st.settings);
            }
            refresh_pets(&a);
            refresh_settings(&a);
        }
    });

    let a = app.clone();
    app.settings_win.on_do_export(move || {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("导出常用语")
            .set_file_name("petphrase-phrases.json")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            let msg = match storage::export_phrases(&data_dir(), &path) {
                Ok(_) => "已导出 ✓".to_string(),
                Err(e) => format!("导出失败:{e}"),
            };
            a.settings_win.set_data_msg(msg.into());
        }
    });

    let a = app.clone();
    app.settings_win.on_do_import(move || {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("导入常用语")
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match storage::import_phrases(&data_dir(), &path) {
                Ok(data) => {
                    {
                        let mut st = a.state.borrow_mut();
                        st.data = data;
                        st.active_group = 0;
                    }
                    a.settings_win.set_data_msg("已导入 ✓".into());
                    refresh_settings(&a);
                    refresh_panel(&a);
                }
                Err(e) => a.settings_win.set_data_msg(format!("导入失败:{e}").into()),
            }
        }
    });

    // 初始自启状态
    if let Ok(al) = autostart_handle() {
        app.settings_win.set_autostart_on(al.is_enabled().unwrap_or(false));
    }
}

fn autostart_handle() -> Result<auto_launch::AutoLaunch, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("PetPhrase")
        .set_app_path(&exe.to_string_lossy())
        .build()
        .map_err(|e| e.to_string())
}

/* ================= 托盘 ================= */

fn setup_tray(app: &Rc<App>) -> Result<tray_icon::TrayIcon, Box<dyn std::error::Error>> {
    use tray_icon::menu::{Menu, MenuItem};

    let icon_png = include_bytes!("../assets/icon.png");
    let rgba = image::load_from_memory(icon_png)?.into_rgba8();
    let (w, h) = rgba.dimensions();
    let icon = tray_icon::Icon::from_rgba(rgba.into_raw(), w, h)?;

    let toggle = MenuItem::new("显示/隐藏宠物", true, None);
    let settings_item = MenuItem::new("设置", true, None);
    let quit = MenuItem::new("退出", true, None);
    let menu = Menu::new();
    menu.append(&toggle)?;
    menu.append(&settings_item)?;
    menu.append(&quit)?;

    let tray = tray_icon::TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("PetPhrase")
        .with_menu(Box::new(menu))
        .build()?;

    let (toggle_id, settings_id, quit_id) = (toggle.id().clone(), settings_item.id().clone(), quit.id().clone());
    let a = app.clone();
    let poll = Box::leak(Box::new(slint::Timer::default()));
    poll.start(slint::TimerMode::Repeated, Duration::from_millis(150), move || {
        while let Ok(ev) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            if ev.id == toggle_id {
                if a.pet.window().is_visible() {
                    let _ = a.pet.window().hide();
                    hide_panel(&a);
                } else {
                    let _ = a.pet.show();
                }
            } else if ev.id == settings_id {
                open_settings(&a);
            } else if ev.id == quit_id {
                let _ = slint::quit_event_loop();
            }
        }
    });

    Ok(tray)
}
