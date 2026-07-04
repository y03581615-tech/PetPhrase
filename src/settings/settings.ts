import { convertFileSrc } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import {
  exportPhrases,
  getPhrases,
  getSettings,
  importPhrases,
  listPets,
  savePhrases,
  saveSettings,
} from "../shared/ipc";
import { GROUP_ICONS, groupIcon, ICON } from "../shared/icons";
import type { Group, Phrase, PhraseData, Settings } from "../shared/types";
import "./settings.css";

let data: PhraseData = { groups: [] };
let settings: Settings | null = null;
let activeGroupId: string | null = null;

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;
const uid = (): string => crypto.randomUUID();

/* ---------- 应用内确认框(替代原生弹窗,风格与整体一致) ---------- */

function confirmDialog(title: string, message: string): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "modal-overlay";
    overlay.innerHTML = `
      <div class="modal" role="dialog" aria-modal="true">
        <div class="modal-icon">${ICON.trash}</div>
        <div class="modal-title"></div>
        <div class="modal-msg"></div>
        <div class="modal-actions">
          <button class="btn subtle" data-act="cancel">取消</button>
          <button class="btn danger" data-act="ok">删除</button>
        </div>
      </div>`;
    (overlay.querySelector(".modal-title") as HTMLElement).textContent = title;
    (overlay.querySelector(".modal-msg") as HTMLElement).textContent = message;

    const done = (ok: boolean) => {
      overlay.remove();
      document.removeEventListener("keydown", onKey);
      resolve(ok);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") done(false);
      if (e.key === "Enter") done(true);
    };
    overlay.onclick = (e) => {
      if (e.target === overlay) done(false);
    };
    (overlay.querySelector('[data-act="cancel"]') as HTMLButtonElement).onclick = () => done(false);
    (overlay.querySelector('[data-act="ok"]') as HTMLButtonElement).onclick = () => done(true);
    document.addEventListener("keydown", onKey);
    document.body.appendChild(overlay);
    (overlay.querySelector('[data-act="cancel"]') as HTMLButtonElement).focus();
  });
}

/* ---------- 持久化 ---------- */

async function persist(): Promise<void> {
  await savePhrases(data);
}

async function persistSettings(patch: Partial<Settings>): Promise<void> {
  if (!settings) return;
  settings = { ...settings, ...patch };
  await saveSettings(settings);
}

/* ---------- 分组列表 ---------- */

function activeGroup(): Group | null {
  return data.groups.find((g) => g.id === activeGroupId) ?? data.groups[0] ?? null;
}

function renderGroups(): void {
  const list = $("group-list");
  list.innerHTML = "";
  data.groups.forEach((g, idx) => {
    const row = document.createElement("div");
    row.className = "group-row" + (g.id === activeGroup()?.id ? " active" : "");
    row.draggable = true;

    const drag = document.createElement("span");
    drag.className = "drag";
    drag.innerHTML = ICON.gripVertical;
    const icon = document.createElement("span");
    icon.style.display = "inline-flex";
    icon.innerHTML = groupIcon(g.icon);
    const name = document.createElement("span");
    name.className = "name";
    name.textContent = g.name;
    const count = document.createElement("span");
    count.className = "count";
    count.textContent = String(g.phrases.length);

    row.append(drag, icon, name, count);
    row.onclick = () => {
      activeGroupId = g.id;
      renderAll();
    };

    bindReorder(row, idx, (from, to) => {
      const [moved] = data.groups.splice(from, 1);
      data.groups.splice(to, 0, moved);
      renderAll();
      void persist();
    });

    list.appendChild(row);
  });
}

/** HTML5 拖拽排序:dataTransfer 存源下标,drop 到目标下标 */
function bindReorder(
  row: HTMLElement,
  index: number,
  apply: (from: number, to: number) => void,
): void {
  row.addEventListener("dragstart", (e) => {
    e.dataTransfer?.setData("text/plain", String(index));
  });
  row.addEventListener("dragover", (e) => {
    e.preventDefault();
    row.classList.add("drag-over");
  });
  row.addEventListener("dragleave", () => row.classList.remove("drag-over"));
  row.addEventListener("drop", (e) => {
    e.preventDefault();
    row.classList.remove("drag-over");
    const from = Number(e.dataTransfer?.getData("text/plain"));
    if (!Number.isNaN(from) && from !== index) apply(from, index);
  });
}

/* ---------- 分组头:重命名/图标/删除 ---------- */

function renderGroupHeader(): void {
  const header = $("group-header");
  header.innerHTML = "";
  const g = activeGroup();
  if (!g) {
    header.innerHTML = `<span class="hint">还没有分组,点左下角新建一个</span>`;
    return;
  }

  const name = document.createElement("span");
  name.className = "g-name";
  name.contentEditable = "plaintext-only";
  name.textContent = g.name;
  name.title = "点击修改分组名";
  const commit = () => {
    const v = name.textContent?.trim();
    if (v && v !== g.name) {
      g.name = v;
      renderGroups();
      void persist();
    } else {
      name.textContent = g.name;
    }
  };
  name.addEventListener("blur", commit);
  name.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      name.blur();
    }
  });

  const picker = document.createElement("div");
  picker.className = "icon-picker";
  for (const [key, svg] of Object.entries(GROUP_ICONS)) {
    const b = document.createElement("button");
    b.innerHTML = svg;
    b.title = key;
    b.className = g.icon === key ? "active" : "";
    b.onclick = () => {
      g.icon = key;
      renderAll();
      void persist();
    };
    picker.appendChild(b);
  }

  const spacer = document.createElement("span");
  spacer.className = "spacer";

  const del = document.createElement("button");
  del.className = "btn subtle";
  del.innerHTML = `${ICON.trash} 删除分组`;
  del.onclick = async () => {
    const ok = await confirmDialog(
      "删除分组",
      `将删除「${g.name}」及其中 ${g.phrases.length} 条常用语,此操作不可撤销。`,
    );
    if (!ok) return;
    data.groups = data.groups.filter((x) => x.id !== g.id);
    activeGroupId = data.groups[0]?.id ?? null;
    renderAll();
    void persist();
  };

  header.append(name, picker, spacer, del);
}

/* ---------- 短语列表 ---------- */

function renderPhrases(): void {
  const list = $("phrase-list");
  list.innerHTML = "";
  const g = activeGroup();
  if (!g) return;
  if (!g.phrases.length) {
    list.innerHTML = `<span class="hint">空分组,在下方输入第一条常用语</span>`;
    return;
  }

  g.phrases.forEach((p, idx) => {
    const row = document.createElement("div");
    row.className = "phrase-row";
    row.draggable = true;

    const drag = document.createElement("span");
    drag.className = "drag";
    drag.innerHTML = ICON.gripVertical;

    const text = document.createElement("div");
    text.className = "p-text";
    text.textContent = p.text;
    text.title = "点击编辑";
    text.onclick = () => startEdit(row, text, p);

    const ops = document.createElement("div");
    ops.className = "ops";
    const edit = document.createElement("button");
    edit.innerHTML = ICON.pencil;
    edit.title = "编辑";
    edit.onclick = () => startEdit(row, text, p);
    const del = document.createElement("button");
    del.className = "danger";
    del.innerHTML = ICON.trash;
    del.title = "删除";
    del.onclick = () => {
      g.phrases = g.phrases.filter((x) => x.id !== p.id);
      renderAll();
      void persist();
    };
    ops.append(edit, del);

    row.append(drag, text, ops);
    bindReorder(row, idx, (from, to) => {
      const [moved] = g.phrases.splice(from, 1);
      g.phrases.splice(to, 0, moved);
      renderAll();
      void persist();
    });
    list.appendChild(row);
  });
}

function startEdit(row: HTMLElement, textEl: HTMLElement, p: Phrase): void {
  const ta = document.createElement("textarea");
  ta.className = "p-edit";
  ta.value = p.text;
  ta.rows = Math.min(6, p.text.split("\n").length + 1);
  row.replaceChild(ta, textEl);
  ta.focus();
  ta.setSelectionRange(ta.value.length, ta.value.length);

  const commit = () => {
    const v = ta.value.trim();
    if (v && v !== p.text) {
      p.text = v;
      void persist();
    }
    renderAll();
  };
  ta.addEventListener("blur", commit);
  ta.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      ta.value = p.text;
      ta.blur();
    }
    if (e.key === "Enter" && e.ctrlKey) ta.blur();
  });
}

/* ---------- 外观页 ---------- */

async function renderPets(): Promise<void> {
  const grid = $("pet-grid");
  grid.innerHTML = "";
  const pets = await listPets();
  if (!pets.length) {
    grid.innerHTML = `<span class="hint">未发现宠物包</span>`;
    return;
  }
  for (const pet of pets) {
    const card = document.createElement("div");
    card.className =
      "pet-card" +
      (pet.id === settings?.pet_id ? " active" : "") +
      (pet.error ? " broken" : "");

    const thumb = document.createElement("div");
    thumb.className = "thumb";
    if (!pet.error) {
      // 第一帧缩略:192x208 → 72x78(0.375 倍)
      thumb.style.backgroundImage = `url("${convertFileSrc(pet.spritesheet)}")`;
      thumb.style.backgroundSize = "auto";
      thumb.style.transform = "scale(0.375)";
      thumb.style.transformOrigin = "top left";
      thumb.style.width = "192px";
      thumb.style.height = "208px";
      const wrap = document.createElement("div");
      wrap.style.width = "72px";
      wrap.style.height = "78px";
      wrap.style.overflow = "hidden";
      wrap.appendChild(thumb);
      card.appendChild(wrap);
    }

    const name = document.createElement("div");
    name.className = "p-name";
    name.textContent = pet.name;
    card.appendChild(name);

    if (pet.error) {
      const err = document.createElement("div");
      err.className = "p-err";
      err.textContent = pet.error;
      card.appendChild(err);
    } else {
      card.onclick = () => {
        void persistSettings({ pet_id: pet.id }).then(renderPets);
      };
    }
    grid.appendChild(card);
  }
}

async function initAppearance(): Promise<void> {
  await renderPets();

  $("custom-dir").textContent = settings?.custom_pet_dir ?? "";
  document.querySelectorAll<HTMLInputElement>('input[name="theme"]').forEach((r) => {
    r.checked = r.value === settings?.theme;
    r.onchange = () => void persistSettings({ theme: r.value as Settings["theme"] });
  });

  const auto = $<HTMLInputElement>("autostart");
  auto.checked = await isEnabled().catch(() => false);
  auto.onchange = async () => {
    try {
      if (auto.checked) await enable();
      else await disable();
    } catch {
      auto.checked = !auto.checked;
    }
  };

  $("pick-pet-dir").onclick = async () => {
    const dir = await open({ directory: true, title: "选择宠物目录" });
    if (typeof dir === "string") {
      await persistSettings({ custom_pet_dir: dir });
      $("custom-dir").textContent = dir;
      await renderPets();
    }
  };

  $("export").onclick = async () => {
    const path = await save({
      title: "导出常用语",
      defaultPath: "petphrase-phrases.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await exportPhrases(path);
    $("data-msg").textContent = "已导出 ✓";
  };

  $("import").onclick = async () => {
    const path = await open({
      title: "导入常用语",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (typeof path !== "string") return;
    try {
      data = await importPhrases(path);
      activeGroupId = data.groups[0]?.id ?? null;
      renderAll();
      $("data-msg").textContent = "已导入 ✓";
    } catch (err) {
      $("data-msg").textContent = `导入失败:${String(err)}`;
    }
  };
}

/* ---------- 装配 ---------- */

function renderAll(): void {
  renderGroups();
  renderGroupHeader();
  renderPhrases();
}

async function main(): Promise<void> {
  settings = await getSettings();
  data = await getPhrases();
  activeGroupId = data.groups[0]?.id ?? null;

  // 页签切换
  document.querySelectorAll<HTMLButtonElement>(".page-tab").forEach((tab) => {
    tab.onclick = () => {
      document.querySelectorAll(".page-tab").forEach((t) => t.classList.remove("active"));
      tab.classList.add("active");
      $("page-phrases").hidden = tab.dataset.page !== "phrases";
      $("page-appearance").hidden = tab.dataset.page !== "appearance";
    };
  });

  $("add-group").innerHTML = `${ICON.plus} 新建分组`;
  $("add-group").onclick = () => {
    const g: Group = { id: uid(), name: "新分组", icon: "folder", phrases: [] };
    data.groups.push(g);
    activeGroupId = g.id;
    renderAll();
    void persist();
  };

  const newPhrase = $<HTMLTextAreaElement>("new-phrase");
  const addPhrase = () => {
    const text = newPhrase.value.trim();
    const g = activeGroup();
    if (!text || !g) return;
    g.phrases.push({ id: uid(), text });
    newPhrase.value = "";
    renderAll();
    void persist();
  };
  $("add-phrase").onclick = addPhrase;
  newPhrase.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && e.ctrlKey) addPhrase();
  });

  renderAll();
  await initAppearance();
}

void main();
