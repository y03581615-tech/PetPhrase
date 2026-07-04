/** petdex 雪碧图行序(官方约定):idle, wave, run, failed, review, jump, extra1, extra2 */
export const STATE_ROWS = {
  idle: 0,
  wave: 1,
  run: 2,
  failed: 3,
  review: 4,
  jump: 5,
} as const;

export type PetState = keyof typeof STATE_ROWS;

export const FRAME_W = 192;
export const FRAME_H = 208;
export const FRAMES_PER_STATE = 6;
export const LOOP_MS = 1100;

/** 网格由图片实际尺寸推算,不硬编码 8×9(文档口径与素材实测有出入) */
export function gridFromImage(
  imgW: number,
  imgH: number,
  fw = FRAME_W,
  fh = FRAME_H,
): { rows: number; cols: number } {
  return { rows: Math.max(1, Math.floor(imgH / fh)), cols: Math.max(1, Math.floor(imgW / fw)) };
}

/** background-position 偏移(负值) */
export function frameOffset(
  row: number,
  col: number,
  fw = FRAME_W,
  fh = FRAME_H,
): { x: number; y: number } {
  // || 0 归一化 -0
  return { x: -col * fw || 0, y: -row * fh || 0 };
}

export interface Animator {
  play(state: PetState, once?: boolean): void;
  tick(nowMs: number): { row: number; col: number };
  current(): PetState;
}

/**
 * 帧驱动器:6帧/状态、1100ms/循环;once 播完一轮自动回 idle。
 * rows/cols = 雪碧图实际网格,状态行越界时回落 idle 行。
 */
export function createAnimator(rows: number, cols: number): Animator {
  const frameMs = LOOP_MS / FRAMES_PER_STATE;
  const frames = Math.min(FRAMES_PER_STATE, cols);
  let state: PetState = "idle";
  let once = false;
  let frame = 0;
  let last: number | null = null;

  const rowOf = (s: PetState): number => (STATE_ROWS[s] < rows ? STATE_ROWS[s] : STATE_ROWS.idle);

  return {
    play(s: PetState, playOnce = false) {
      state = rowOf(s) === STATE_ROWS[s] ? s : "idle";
      once = playOnce && state !== "idle";
      frame = 0;
      last = null;
    },
    tick(nowMs: number) {
      if (last === null) last = nowMs;
      while (nowMs - last >= frameMs) {
        last += frameMs;
        frame += 1;
        if (frame >= frames) {
          frame = 0;
          if (once) {
            state = "idle";
            once = false;
          }
        }
      }
      return { row: rowOf(state), col: frame };
    },
    current: () => state,
  };
}
