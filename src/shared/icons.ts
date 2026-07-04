// 统一图标出口:UI 功能图标 + 分组可选图标(固定集,设置窗与面板宫格共用)
import briefcase from "lucide-static/icons/briefcase.svg?raw";
import check from "lucide-static/icons/check.svg?raw";
import code from "lucide-static/icons/code.svg?raw";
import creditCard from "lucide-static/icons/credit-card.svg?raw";
import download from "lucide-static/icons/download.svg?raw";
import fish from "lucide-static/icons/fish.svg?raw";
import folder from "lucide-static/icons/folder.svg?raw";
import gripVertical from "lucide-static/icons/grip-vertical.svg?raw";
import headphones from "lucide-static/icons/headphones.svg?raw";
import heart from "lucide-static/icons/heart.svg?raw";
import layoutGrid from "lucide-static/icons/layout-grid.svg?raw";
import mail from "lucide-static/icons/mail.svg?raw";
import mapPin from "lucide-static/icons/map-pin.svg?raw";
import messageCircle from "lucide-static/icons/message-circle.svg?raw";
import pencil from "lucide-static/icons/pencil.svg?raw";
import plus from "lucide-static/icons/plus.svg?raw";
import search from "lucide-static/icons/search.svg?raw";
import settings from "lucide-static/icons/settings.svg?raw";
import smile from "lucide-static/icons/smile.svg?raw";
import star from "lucide-static/icons/star.svg?raw";
import trash from "lucide-static/icons/trash-2.svg?raw";
import upload from "lucide-static/icons/upload.svg?raw";
import x from "lucide-static/icons/x.svg?raw";

/** UI 功能图标 */
export const ICON = {
  search,
  settings,
  layoutGrid,
  check,
  x,
  plus,
  trash,
  pencil,
  download,
  upload,
  gripVertical,
  folder,
} as const;

/** 分组可选图标(设置里的固定选择集) */
export const GROUP_ICONS: Record<string, string> = {
  star,
  briefcase,
  headphones,
  code,
  mail,
  "message-circle": messageCircle,
  smile,
  heart,
  fish,
  "map-pin": mapPin,
  "credit-card": creditCard,
  folder,
};

export function groupIcon(name: string | null): string {
  return (name && GROUP_ICONS[name]) || GROUP_ICONS.folder;
}
