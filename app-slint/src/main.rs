//! S1 spike:验证 Slint 能否做桌宠窗口——
//! 透明 + 无边框 + 置顶 + 跳任务栏 + 雪碧图逐帧动画 + 拖拽。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::winit_030::{winit, WinitWindowAccessor};
use std::rc::Rc;
use std::time::Duration;

slint::slint! {
export component PetWindow inherits Window {
    in property <image> sheet;
    in property <int> frame-col;
    in property <int> frame-row;
    callback pet-clicked;
    callback drag-start;

    no-frame: true;
    always-on-top: true;
    background: transparent;
    width: 192px;
    height: 208px;
    title: "PetPhrase";

    Image {
        width: 192px;
        height: 208px;
        source: root.sheet;
        image-fit: preserve;
        source-clip-x: root.frame-col * 192;
        source-clip-y: root.frame-row * 208;
        source-clip-width: 192;
        source-clip-height: 208;
    }

    TouchArea {
        moved => {
            if (self.pressed && (abs(self.mouse-x - self.pressed-x) > 4px || abs(self.mouse-y - self.pressed-y) > 4px)) {
                root.drag-start();
            }
        }
        clicked => { root.pet-clicked(); }
    }
}
}

const FRAMES_PER_STATE: i32 = 6;
const FRAME_MS: u64 = 1100 / 6;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .with_winit_window_attributes_hook(|attrs| {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs.with_transparent(true).with_skip_taskbar(true)
        })
        .select()?;

    let app = PetWindow::new()?;

    // 直接用已装的 kun-like 验证真实素材
    let home = std::env::var("USERPROFILE")?;
    let sheet_path = format!("{home}/.codex/pets/kun-like/spritesheet.webp");
    let sheet = slint::Image::load_from_path(std::path::Path::new(&sheet_path))?;
    app.set_sheet(sheet);

    // 帧驱动:idle 行循环;点击播 wave 行一轮
    let wave_left = Rc::new(std::cell::Cell::new(0i32));
    {
        let app_weak = app.as_weak();
        let wave = wave_left.clone();
        let timer = Box::leak(Box::new(slint::Timer::default()));
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(FRAME_MS),
            move || {
                let Some(app) = app_weak.upgrade() else { return };
                let next = (app.get_frame_col() + 1) % FRAMES_PER_STATE;
                app.set_frame_col(next);
                if wave.get() > 0 {
                    wave.set(wave.get() - 1);
                    app.set_frame_row(1);
                } else {
                    app.set_frame_row(0);
                }
            },
        );
    }

    {
        let wave = wave_left.clone();
        app.on_pet_clicked(move || {
            wave.set(FRAMES_PER_STATE); // 播一轮 wave
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_drag_start(move || {
            let Some(app) = app_weak.upgrade() else { return };
            app.window().with_winit_window(|w: &winit::window::Window| {
                let _ = w.drag_window();
            });
        });
    }

    app.run()?;
    Ok(())
}
