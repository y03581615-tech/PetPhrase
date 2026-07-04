import { convertFileSrc } from "@tauri-apps/api/core";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emit, getSettings, listen, listPets, saveSettings } from "../shared/ipc";
import { EVT, type Settings } from "../shared/types";
import {
  createAnimator,
  frameOffset,
  gridFromImage,
  type Animator,
} from "./animator";
import "./pet.css";

const DRAG_THRESHOLD_PX = 4;
const POS_SAVE_DEBOUNCE_MS = 500;

const petEl = document.getElementById("pet") as HTMLDivElement;
const win = getCurrentWindow();

let settings: Settings | null = null;
let animator: Animator | null = null;

async function loadSprite(petId: string): Promise<void> {
  const pets = await listPets();
  const pet =
    pets.find((p) => p.id === petId && !p.error) ?? pets.find((p) => !p.error) ?? null;
  if (!pet) {
    // 无可用宠物:显示占位圆点,不崩
    petEl.style.background = "radial-gradient(circle at 50% 45%, #f97316 55%, transparent 56%)";
    return;
  }
  const url = convertFileSrc(pet.spritesheet);
  const img = new Image();
  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error(`雪碧图加载失败: ${pet.spritesheet}`));
    img.src = url;
  });
  const { rows, cols } = gridFromImage(img.naturalWidth, img.naturalHeight);
  animator = createAnimator(rows, cols);
  petEl.style.background = "";
  petEl.style.backgroundImage = `url("${url}")`;
}

function startLoop(): void {
  const render = (now: number) => {
    if (animator) {
      const { row, col } = animator.tick(now);
      const { x, y } = frameOffset(row, col);
      petEl.style.backgroundPosition = `${x}px ${y}px`;
    }
    requestAnimationFrame(render);
  };
  requestAnimationFrame(render);
}

/** 拖拽与单击分流:位移 >4px 交给系统拖窗,否则视为单击 */
function bindPointer(): void {
  let downAt: { x: number; y: number } | null = null;

  petEl.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    downAt = { x: e.screenX, y: e.screenY };
  });

  petEl.addEventListener("mousemove", (e) => {
    if (!downAt) return;
    const moved =
      Math.abs(e.screenX - downAt.x) > DRAG_THRESHOLD_PX ||
      Math.abs(e.screenY - downAt.y) > DRAG_THRESHOLD_PX;
    if (moved) {
      downAt = null;
      void win.startDragging();
    }
  });

  petEl.addEventListener("mouseup", (e) => {
    if (e.button !== 0 || !downAt) return;
    downAt = null;
    animator?.play("wave", true);
    void emit(EVT.togglePanel, null);
  });
}

function bindMoveSave(): void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  void win.onMoved(({ payload }) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      if (!settings) return;
      settings = { ...settings, pet_pos: [payload.x, payload.y] };
      void saveSettings(settings);
    }, POS_SAVE_DEBOUNCE_MS);
  });
}

async function main(): Promise<void> {
  settings = await getSettings();

  if (settings.pet_pos) {
    await win.setPosition(new PhysicalPosition(settings.pet_pos[0], settings.pet_pos[1]));
  }

  await loadSprite(settings.pet_id);
  startLoop();
  bindPointer();
  bindMoveSave();

  await listen(EVT.phraseCopied, () => animator?.play("wave", true));
  await listen<Settings>(EVT.settingsChanged, (e) => {
    const next = e.payload;
    const petChanged = settings?.pet_id !== next.pet_id;
    settings = next;
    if (petChanged) void loadSprite(next.pet_id);
  });
}

void main();
