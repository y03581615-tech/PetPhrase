// 与 src-tauri/src/storage.rs、pet_loader.rs 结构镜像

export interface Phrase {
  id: string;
  text: string;
}

export interface Group {
  id: string;
  name: string;
  icon: string | null;
  phrases: Phrase[];
}

export interface PhraseData {
  groups: Group[];
}

export interface Settings {
  pet_id: string;
  theme: "acrylic" | "solid";
  pet_pos: [number, number] | null;
  last_group: string | null;
  custom_pet_dir: string | null;
}

export interface PetInfo {
  id: string;
  name: string;
  spritesheet: string;
  error: string | null;
}

/** 事件名常量 —— Rust 与各窗口共用 */
export const EVT = {
  dataChanged: "data-changed",
  settingsChanged: "settings-changed",
  phraseCopied: "phrase-copied",
  showPreview: "show-preview",
  hidePreview: "hide-preview",
  vibrancyFailed: "vibrancy-failed",
  togglePanel: "toggle-panel",
  previewReady: "preview-ready",
} as const;

/** show-preview 事件载荷 */
export interface PreviewPayload {
  text: string;
  /** 面板窗口外框(物理像素) */
  panelX: number;
  panelY: number;
  panelW: number;
  /** 浮层贴面板哪一侧 */
  side: "left" | "right";
  /** 触发条目相对面板顶部的 y(逻辑像素),用于纵向对齐 */
  anchorY: number;
  scale: number;
}
