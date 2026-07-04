import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "../shared/ipc";
import { EVT, type PreviewPayload } from "../shared/types";
import "./preview.css";

const GAP_PX = 8;

const win = getCurrentWindow();
const card = document.getElementById("card") as HTMLDivElement;
card.hidden = true;

async function show(p: PreviewPayload): Promise<void> {
  card.textContent = p.text;
  card.hidden = false;

  const mySize = await win.outerSize();
  const x =
    p.side === "right"
      ? p.panelX + p.panelW + GAP_PX * p.scale
      : p.panelX - mySize.width - GAP_PX * p.scale;
  // 纵向对齐触发条目,微调避免顶出屏
  const y = Math.max(0, p.panelY + Math.round(p.anchorY * p.scale) - 8 * p.scale);

  await win.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
  await win.show();
}

async function main(): Promise<void> {
  await listen<PreviewPayload>(EVT.showPreview, (e) => void show(e.payload));
  await listen(EVT.hidePreview, () => {
    card.hidden = true;
    void win.hide();
  });
}

void main();
