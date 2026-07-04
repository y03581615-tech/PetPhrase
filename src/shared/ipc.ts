import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { PetInfo, PhraseData, Settings } from "./types";

export const getPhrases = (): Promise<PhraseData> => invoke("get_phrases");
export const savePhrases = (data: PhraseData): Promise<void> => invoke("save_phrases", { data });
export const getSettings = (): Promise<Settings> => invoke("get_settings");
export const saveSettings = (settings: Settings): Promise<void> =>
  invoke("save_settings", { settings });
export const listPets = (): Promise<PetInfo[]> => invoke("list_pets");
export const exportPhrases = (path: string): Promise<void> => invoke("export_phrases", { path });
export const importPhrases = (path: string): Promise<PhraseData> =>
  invoke("import_phrases", { path });

export const copyText = (text: string): Promise<void> => writeText(text);

export { emit, listen };
export type { UnlistenFn };
