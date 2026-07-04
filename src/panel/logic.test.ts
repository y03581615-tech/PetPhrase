import { describe, expect, test } from "vitest";
import { isShort, panelPosition, searchPhrases } from "./logic";
import type { PhraseData } from "../shared/types";

describe("isShort", () => {
  test("10 字以内且无换行为短句", () => {
    expect(isShort("收到马上处理好的呢")).toBe(true); // 9 字
    expect(isShort("一二三四五六七八九十")).toBe(true); // 10 字
  });
  test("11 字为长句", () => {
    expect(isShort("一二三四五六七八九十一")).toBe(false);
  });
  test("含换行即长句", () => {
    expect(isShort("短\n句")).toBe(false);
  });
});

const data: PhraseData = {
  groups: [
    {
      id: "g1",
      name: "工作",
      icon: null,
      phrases: [
        { id: "p1", text: "收到,马上处理" },
        { id: "p2", text: "会议纪要已同步" },
      ],
    },
    {
      id: "g2",
      name: "客服",
      icon: null,
      phrases: [{ id: "p3", text: "感谢您的反馈,我们马上排查" }],
    },
  ],
};

describe("searchPhrases", () => {
  test("跨组匹配并标注来源组", () => {
    const hits = searchPhrases(data, "马上");
    expect(hits.map((h) => h.phrase.id)).toEqual(["p1", "p3"]);
    expect(hits[1].groupName).toBe("客服");
  });
  test("空查询返回空", () => {
    expect(searchPhrases(data, "  ")).toEqual([]);
  });
  test("无命中返回空", () => {
    expect(searchPhrases(data, "不存在")).toEqual([]);
  });
});

describe("panelPosition", () => {
  const work = { x: 0, y: 0, w: 1920, h: 1040 };
  const panel = { w: 300, h: 400 };

  test("空间充足:面板在宠物上方,预览贴右", () => {
    const p = panelPosition({ x: 800, y: 600, w: 192, h: 208 }, panel, work);
    expect(p).toEqual({ x: 800, y: 600 - 400 - 12, side: "right" });
  });

  test("宠物贴顶:面板翻到下方", () => {
    const p = panelPosition({ x: 800, y: 10, w: 192, h: 208 }, panel, work);
    expect(p.y).toBe(10 + 208 + 12);
  });

  test("宠物贴右缘:面板右对齐且预览贴左", () => {
    const p = panelPosition({ x: 1700, y: 600, w: 192, h: 208 }, panel, work);
    expect(p.x + panel.w).toBeLessThanOrEqual(1920);
    expect(p.side).toBe("left");
  });

  test("永不越出工作区", () => {
    const p = panelPosition({ x: -50, y: -50, w: 192, h: 208 }, panel, work);
    expect(p.x).toBeGreaterThanOrEqual(0);
    expect(p.y).toBeGreaterThanOrEqual(0);
  });
});
