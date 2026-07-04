import { describe, expect, test } from "vitest";
import {
  createAnimator,
  frameOffset,
  gridFromImage,
  LOOP_MS,
  FRAMES_PER_STATE,
} from "./animator";

const FRAME_MS = LOOP_MS / FRAMES_PER_STATE;

describe("gridFromImage", () => {
  test("推算 8列×9行(1536×1872 实测口径)", () => {
    expect(gridFromImage(1536, 1872)).toEqual({ rows: 9, cols: 8 });
  });

  test("推算 9列×8行(文档口径)", () => {
    expect(gridFromImage(1728, 1664)).toEqual({ rows: 8, cols: 9 });
  });
});

describe("frameOffset", () => {
  test("负偏移定位帧", () => {
    expect(frameOffset(0, 0)).toEqual({ x: 0, y: 0 });
    expect(frameOffset(1, 2)).toEqual({ x: -384, y: -208 });
  });
});

describe("createAnimator", () => {
  test("idle 循环:6帧后回到第0帧", () => {
    const a = createAnimator(9, 8);
    expect(a.tick(0)).toEqual({ row: 0, col: 0 });
    expect(a.tick(FRAME_MS * 5 + 1)).toEqual({ row: 0, col: 5 });
    expect(a.tick(FRAME_MS * 6 + 1)).toEqual({ row: 0, col: 0 });
  });

  test("wave once 播完一轮自动回 idle", () => {
    const a = createAnimator(9, 8);
    a.play("wave", true);
    expect(a.tick(0)).toEqual({ row: 1, col: 0 });
    expect(a.tick(FRAME_MS * 5 + 1).row).toBe(1);
    const after = a.tick(FRAME_MS * 6 + 1);
    expect(after.row).toBe(0);
    expect(a.current()).toBe("idle");
  });

  test("行数不足的雪碧图回落 idle 行", () => {
    const a = createAnimator(1, 8);
    a.play("wave", true);
    expect(a.tick(0).row).toBe(0);
  });
});
