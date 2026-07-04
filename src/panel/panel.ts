import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import {
  copyText,
  emit,
  getPhrases,
  getSettings,
  listen,
  saveSettings,
} from "../shared/ipc";
import { groupIcon, ICON } from "../shared/icons";
import {
  EVT,
  type Group,
  type Phrase,
  type PhraseData,
  type PreviewPayload,
  type Settings,
} from "../shared/types";
import { isShort, panelPosition, searchPhrases } from "./logic";
import "./panel.css";

const HOVER_PREVIEW_DELAY_MS = 400;
const COPY_FLASH_MS = 300;
const HIDE_AFTER_COPY_MS = 200;
const GRID_HIDE_THRESHOLD = 4;

const win = getCurrentWindow();
const tabsEl = document.getElementById("tabs") as HTMLDivElement;
const gridBtn = document.getElementById("grid-btn") as HTMLButtonElement;
const gridSheet = document.getElementById("grid-sheet") as HTMLDivElement;
const listEl = document.getElementById("list") as HTMLDivElement;
const searchInput = document.getElementById("search") as HTMLInputElement;

let data: PhraseData = { groups: [] };
let settings: Settings | null = null;
let activeGroupId: string | null = null;
let previewSide: "left" | "right" = "right";
let hoverTimer: ReturnType<typeof setTimeout> | null = null;

/* ---------- 渲染 ---------- */

function activeGroup(): Group | null {
  return data.groups.find((g) => g.id === activeGroupId) ?? data.groups[0] ?? null;
}

function renderTabs(): void {
  tabsEl.innerHTML = "";
  for (const g of data.groups) {
    const btn = document.createElement("button");
    btn.className = "tab" + (g.id === activeGroup()?.id ? " active" : "");
    btn.textContent = g.name;
    btn.onclick = () => selectGroup(g.id);
    tabsEl.appendChild(btn);
  }
  gridBtn.style.display = data.groups.length <= GRID_HIDE_THRESHOLD ? "none" : "";
  tabsEl.querySelector(".active")?.scrollIntoView({ inline: "nearest", block: "nearest" });
}

function renderGridSheet(): void {
  gridSheet.innerHTML = "";
  for (const g of data.groups) {
    const cell = document.createElement("button");
    cell.className = "grid-cell" + (g.id === activeGroup()?.id ? " active" : "");
    cell.innerHTML = groupIcon(g.icon);
    const label = document.createElement("span");
    label.textContent = g.name;
    cell.appendChild(label);
    cell.onclick = () => {
      gridSheet.hidden = true;
      selectGroup(g.id);
    };
    gridSheet.appendChild(cell);
  }
}

function makeItem(p: Phrase, groupName?: string): HTMLElement {
  const short = isShort(p.text);
  const el = document.createElement("button");
  el.className = short ? "chip" : "card";
  if (short) {
    el.textContent = p.text;
  } else {
    const text = document.createElement("div");
    text.className = "text";
    text.textContent = p.text;
    el.appendChild(text);
    if (groupName) {
      const badge = document.createElement("div");
      badge.className = "group-badge";
      badge.textContent = `来自「${groupName}」`;
      el.appendChild(badge);
    }
    bindHoverPreview(el, text, p.text);
  }
  el.onclick = () => void copyPhrase(el, p.text);
  return el;
}

/** 连续短句合并进一个气泡流容器,长句独占卡片 */
function renderList(): void {
  listEl.innerHTML = "";
  listEl.scrollTop = 0;

  const q = searchInput.value.trim();
  if (q) {
    const hits = searchPhrases(data, q);
    if (!hits.length) {
      listEl.innerHTML = `<div class="empty">没有匹配「${escapeHtml(q)}」的常用语</div>`;
      return;
    }
    for (const h of hits) listEl.appendChild(makeItem(h.phrase, h.groupName));
    return;
  }

  const group = activeGroup();
  if (!group || !group.phrases.length) {
    listEl.innerHTML = `<div class="empty">这个分组还没有常用语<br/>点右上角 ⚙ 去添加</div>`;
    return;
  }

  let flow: HTMLDivElement | null = null;
  for (const p of group.phrases) {
    if (isShort(p.text)) {
      if (!flow) {
        flow = document.createElement("div");
        flow.className = "chip-flow";
        listEl.appendChild(flow);
      }
      flow.appendChild(makeItem(p));
    } else {
      flow = null;
      listEl.appendChild(makeItem(p));
    }
  }
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

function renderAll(): void {
  renderTabs();
  renderGridSheet();
  renderList();
}

/* ---------- 交互 ---------- */

function selectGroup(id: string): void {
  activeGroupId = id;
  searchInput.value = "";
  renderAll();
  if (settings && settings.last_group !== id) {
    settings = { ...settings, last_group: id };
    void saveSettings(settings);
  }
}

async function copyPhrase(el: HTMLElement, text: string): Promise<void> {
  try {
    await copyText(text);
  } catch {
    el.classList.add("copy-failed");
    setTimeout(() => el.classList.remove("copy-failed"), 2000);
    return;
  }
  el.classList.add("copied");
  const mark = document.createElement("span");
  mark.className = "copied-mark";
  mark.innerHTML = ICON.check;
  el.appendChild(mark);
  void emit(EVT.phraseCopied, null);
  setTimeout(() => {
    el.classList.remove("copied");
    mark.remove();
  }, COPY_FLASH_MS);
  setTimeout(() => void hidePanel(), HIDE_AFTER_COPY_MS);
}

function bindHoverPreview(el: HTMLElement, textEl: HTMLElement, fullText: string): void {
  el.addEventListener("mouseenter", () => {
    // 仅被截断的条目触发
    if (textEl.scrollHeight <= textEl.clientHeight + 1) return;
    hoverTimer = setTimeout(() => void showPreview(el, fullText), HOVER_PREVIEW_DELAY_MS);
  });
  el.addEventListener("mouseleave", () => {
    if (hoverTimer) clearTimeout(hoverTimer);
    hoverTimer = null;
    void emit(EVT.hidePreview, null);
  });
}

async function showPreview(el: HTMLElement, text: string): Promise<void> {
  const pos = await win.outerPosition();
  const size = await win.outerSize();
  const payload: PreviewPayload = {
    text,
    panelX: pos.x,
    panelY: pos.y,
    panelW: size.width,
    side: previewSide,
    anchorY: el.getBoundingClientRect().top,
    scale: window.devicePixelRatio,
  };
  await emit(EVT.showPreview, payload);
}

async function hidePanel(): Promise<void> {
  if (hoverTimer) clearTimeout(hoverTimer);
  await emit(EVT.hidePreview, null);
  gridSheet.hidden = true;
  await win.hide();
}

async function togglePanel(): Promise<void> {
  if (await win.isVisible()) {
    await hidePanel();
    return;
  }
  const pet = await WebviewWindow.getByLabel("pet");
  const monitor = await currentMonitor();
  if (pet && monitor) {
    const pPos = await pet.outerPosition();
    const pSize = await pet.outerSize();
    const mySize = await win.outerSize();
    const placement = panelPosition(
      { x: pPos.x, y: pPos.y, w: pSize.width, h: pSize.height },
      { w: mySize.width, h: mySize.height },
      {
        x: monitor.position.x,
        y: monitor.position.y,
        w: monitor.size.width,
        h: monitor.size.height,
      },
    );
    previewSide = placement.side;
    await win.setPosition(new PhysicalPosition(placement.x, placement.y));
  }
  searchInput.value = "";
  renderList();
  await win.show();
  await win.setFocus();
  searchInput.focus();
}

/* ---------- 装配 ---------- */

async function main(): Promise<void> {
  (document.getElementById("search-icon") as HTMLSpanElement).innerHTML = ICON.search;
  (document.getElementById("gear") as HTMLButtonElement).innerHTML = ICON.settings;
  gridBtn.innerHTML = ICON.layoutGrid;

  settings = await getSettings();
  if (settings.theme === "solid") document.body.classList.add("solid");
  data = await getPhrases();
  activeGroupId = settings.last_group ?? data.groups[0]?.id ?? null;
  renderAll();

  searchInput.addEventListener("input", renderList);

  // 滚轮映射 Tab 横滚
  tabsEl.addEventListener("wheel", (e) => {
    e.preventDefault();
    tabsEl.scrollLeft += e.deltaY;
  });

  gridBtn.onclick = () => {
    gridSheet.hidden = !gridSheet.hidden;
  };

  (document.getElementById("gear") as HTMLButtonElement).onclick = async () => {
    const settingsWin = await WebviewWindow.getByLabel("settings");
    await settingsWin?.show();
    await settingsWin?.setFocus();
    await hidePanel();
  };

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      void emit(EVT.hidePreview, null);
      if (!gridSheet.hidden) gridSheet.hidden = true;
      else void hidePanel();
    }
  });

  await win.onFocusChanged(({ payload: focused }) => {
    if (!focused) void hidePanel();
  });

  await listen(EVT.togglePanel, () => void togglePanel());
  await listen(EVT.dataChanged, async () => {
    data = await getPhrases();
    renderAll();
  });
  await listen<Settings>(EVT.settingsChanged, (e) => {
    const prev = settings;
    settings = e.payload;
    document.body.classList.toggle("solid", settings.theme === "solid");
    if (prev?.last_group !== settings.last_group) {
      activeGroupId = settings.last_group ?? activeGroupId;
    }
  });
  await listen(EVT.vibrancyFailed, () => document.body.classList.add("solid"));
}

void main();
