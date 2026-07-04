//! petdex 雪碧图帧驱动(移植自 TS 版 animator,行为一致)。

pub const FRAME_W: u32 = 192;
pub const FRAME_H: u32 = 208;
pub const FRAMES_PER_STATE: i32 = 6;
pub const LOOP_MS: u64 = 1100;
pub const FRAME_MS: u64 = LOOP_MS / FRAMES_PER_STATE as u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetState {
    Idle,
    Wave,
}

/// petdex 行序:idle, wave, run, failed, review, jump…
fn row_of(state: PetState, rows: i32) -> i32 {
    let want = match state {
        PetState::Idle => 0,
        PetState::Wave => 1,
    };
    if want < rows {
        want
    } else {
        0
    }
}

/// 网格按图片尺寸推算,不硬编码(文档口径与素材实测相反)
pub fn grid_from_image(w: u32, h: u32) -> (i32, i32) {
    (((h / FRAME_H).max(1)) as i32, ((w / FRAME_W).max(1)) as i32)
}

pub struct Animator {
    rows: i32,
    frames: i32,
    state: PetState,
    once: bool,
    frame: i32,
}

impl Animator {
    pub fn new(rows: i32, cols: i32) -> Self {
        Animator {
            rows,
            frames: FRAMES_PER_STATE.min(cols),
            state: PetState::Idle,
            once: false,
            frame: 0,
        }
    }

    pub fn play(&mut self, state: PetState, once: bool) {
        self.state = state;
        self.once = once && state != PetState::Idle;
        self.frame = 0;
    }

    /// 每 FRAME_MS 调一次,返回 (row, col)
    pub fn step(&mut self) -> (i32, i32) {
        let result = (row_of(self.state, self.rows), self.frame);
        self.frame += 1;
        if self.frame >= self.frames {
            self.frame = 0;
            if self.once {
                self.state = PetState::Idle;
                self.once = false;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_both_orientations() {
        assert_eq!(grid_from_image(1536, 1872), (9, 8));
        assert_eq!(grid_from_image(1728, 1664), (8, 9));
    }

    #[test]
    fn wave_once_returns_to_idle() {
        let mut a = Animator::new(9, 8);
        a.play(PetState::Wave, true);
        for i in 0..FRAMES_PER_STATE {
            assert_eq!(a.step(), (1, i));
        }
        assert_eq!(a.step(), (0, 0), "播完一轮回 idle");
    }

    #[test]
    fn missing_rows_fall_back_to_idle_row() {
        let mut a = Animator::new(1, 8);
        a.play(PetState::Wave, true);
        assert_eq!(a.step().0, 0);
    }
}
