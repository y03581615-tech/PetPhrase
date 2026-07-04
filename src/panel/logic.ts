import type { Phrase, PhraseData } from "../shared/types";

/** 短句阈值:≤10 字符且无换行 → 气泡流;否则 2 行卡片 */
export const SHORT_MAX_CHARS = 10;

export function isShort(text: string): boolean {
  return !text.includes("\n") && [...text].length <= SHORT_MAX_CHARS;
}

export interface SearchHit {
  phrase: Phrase;
  groupId: string;
  groupName: string;
}

/** 跨组子串过滤(大小写不敏感),空查询返回空 */
export function searchPhrases(data: PhraseData, query: string): SearchHit[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return data.groups.flatMap((g) =>
    g.phrases
      .filter((p) => p.text.toLowerCase().includes(q))
      .map((p) => ({ phrase: p, groupId: g.id, groupName: g.name })),
  );
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface PanelPlacement {
  x: number;
  y: number;
  /** 预览浮层应贴的一侧(面板相对屏幕的余量决定) */
  side: "left" | "right";
}

const GAP = 12;

/**
 * 面板贴宠定位:优先宠物上方右对齐宠物左缘,越界时翻转/钳制。
 * 所有量为同一坐标系(物理像素)。
 */
export function panelPosition(pet: Rect, panel: { w: number; h: number }, work: Rect): PanelPlacement {
  // 水平:面板左缘对齐宠物左缘;右侧放不下则右对齐宠物右缘
  let x = pet.x;
  if (x + panel.w > work.x + work.w) x = pet.x + pet.w - panel.w;
  x = Math.max(work.x, Math.min(x, work.x + work.w - panel.w));

  // 垂直:优先上方;放不下翻到下方;再不行钳制
  let y = pet.y - panel.h - GAP;
  if (y < work.y) y = pet.y + pet.h + GAP;
  y = Math.max(work.y, Math.min(y, work.y + work.h - panel.h));

  // 预览侧:面板右侧余量够就贴右,否则贴左
  const rightRoom = work.x + work.w - (x + panel.w);
  const side: "left" | "right" = rightRoom >= 260 ? "right" : "left";

  return { x, y, side };
}
