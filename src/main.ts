import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface Settings {
  microphone: string;
  engine: string;
  whisperModel: string;
  groqApiKey: string;
  recordingMode: string;
  hotkey: string;
  language: string;
}

interface Language {
  code: string;
  name: string;
  flag: string;
}

const LANGUAGES: Language[] = [
  { code: "auto", name: "Auto-detect", flag: "🌐" },
  { code: "af", name: "Afrikaans", flag: "🇿🇦" },
  { code: "ar", name: "Arabic", flag: "🇸🇦" },
  { code: "hy", name: "Armenian", flag: "🇦🇲" },
  { code: "az", name: "Azerbaijani", flag: "🇦🇿" },
  { code: "be", name: "Belarusian", flag: "🇧🇾" },
  { code: "bs", name: "Bosnian", flag: "🇧🇦" },
  { code: "bg", name: "Bulgarian", flag: "🇧🇬" },
  { code: "ca", name: "Catalan", flag: "🇪🇸" },
  { code: "zh", name: "Chinese", flag: "🇨🇳" },
  { code: "hr", name: "Croatian", flag: "🇭🇷" },
  { code: "cs", name: "Czech", flag: "🇨🇿" },
  { code: "da", name: "Danish", flag: "🇩🇰" },
  { code: "nl", name: "Dutch", flag: "🇳🇱" },
  { code: "en", name: "English", flag: "🇬🇧" },
  { code: "et", name: "Estonian", flag: "🇪🇪" },
  { code: "fi", name: "Finnish", flag: "🇫🇮" },
  { code: "fr", name: "French", flag: "🇫🇷" },
  { code: "gl", name: "Galician", flag: "🇪🇸" },
  { code: "de", name: "German", flag: "🇩🇪" },
  { code: "el", name: "Greek", flag: "🇬🇷" },
  { code: "he", name: "Hebrew", flag: "🇮🇱" },
  { code: "hi", name: "Hindi", flag: "🇮🇳" },
  { code: "hu", name: "Hungarian", flag: "🇭🇺" },
  { code: "is", name: "Icelandic", flag: "🇮🇸" },
  { code: "id", name: "Indonesian", flag: "🇮🇩" },
  { code: "it", name: "Italian", flag: "🇮🇹" },
  { code: "ja", name: "Japanese", flag: "🇯🇵" },
  { code: "kk", name: "Kazakh", flag: "🇰🇿" },
  { code: "ko", name: "Korean", flag: "🇰🇷" },
  { code: "lv", name: "Latvian", flag: "🇱🇻" },
  { code: "lt", name: "Lithuanian", flag: "🇱🇹" },
  { code: "mk", name: "Macedonian", flag: "🇲🇰" },
  { code: "ms", name: "Malay", flag: "🇲🇾" },
  { code: "mi", name: "Māori", flag: "🇳🇿" },
  { code: "mr", name: "Marathi", flag: "🇮🇳" },
  { code: "ne", name: "Nepali", flag: "🇳🇵" },
  { code: "no", name: "Norwegian", flag: "🇳🇴" },
  { code: "fa", name: "Persian", flag: "🇮🇷" },
  { code: "pl", name: "Polish", flag: "🇵🇱" },
  { code: "pt", name: "Portuguese", flag: "🇵🇹" },
  { code: "ro", name: "Romanian", flag: "🇷🇴" },
  { code: "ru", name: "Russian", flag: "🇷🇺" },
  { code: "sr", name: "Serbian", flag: "🇷🇸" },
  { code: "sk", name: "Slovak", flag: "🇸🇰" },
  { code: "sl", name: "Slovenian", flag: "🇸🇮" },
  { code: "es", name: "Spanish", flag: "🇪🇸" },
  { code: "sw", name: "Swahili", flag: "🇰🇪" },
  { code: "sv", name: "Swedish", flag: "🇸🇪" },
  { code: "tl", name: "Filipino", flag: "🇵🇭" },
  { code: "ta", name: "Tamil", flag: "🇮🇳" },
  { code: "th", name: "Thai", flag: "🇹🇭" },
  { code: "tr", name: "Turkish", flag: "🇹🇷" },
  { code: "uk", name: "Ukrainian", flag: "🇺🇦" },
  { code: "ur", name: "Urdu", flag: "🇵🇰" },
  { code: "vi", name: "Vietnamese", flag: "🇻🇳" },
  { code: "cy", name: "Welsh", flag: "🏴" },
];

interface MicDevice {
  name: string;
  is_default: boolean;
}

interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

// DOM elements
const languageSelect = document.getElementById("language-select") as HTMLSelectElement;
const languageCombobox = document.getElementById("language-combobox") as HTMLDivElement;
const languageTrigger = document.getElementById("language-trigger") as HTMLButtonElement;
const languageDropdown = document.getElementById("language-dropdown") as HTMLDivElement;
const languageSearch = document.getElementById("language-search") as HTMLInputElement;
const languageList = document.getElementById("language-list") as HTMLUListElement;
const languageFlag = document.getElementById("language-flag") as HTMLSpanElement;
const languageName = document.getElementById("language-name") as HTMLSpanElement;

LANGUAGES.forEach((lang) => {
  const option = document.createElement("option");
  option.value = lang.code;
  option.textContent = lang.name;
  languageSelect.appendChild(option);
});

const languageByCode = new Map(LANGUAGES.map((lang) => [lang.code, lang]));
let filteredLanguages = [...LANGUAGES];
let highlightedLanguageCode = "auto";

function normalizeLanguageText(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

function setLanguageTrigger(code: string) {
  const selected = languageByCode.get(code) ?? LANGUAGES[0];
  languageFlag.textContent = selected.flag;
  languageName.textContent = selected.name;
}

function openLanguageDropdown() {
  languageCombobox.classList.add("open");
  languageCombobox.setAttribute("aria-expanded", "true");
  languageDropdown.classList.remove("hidden");
  languageSearch.value = "";
  filteredLanguages = [...LANGUAGES];
  highlightedLanguageCode = languageSelect.value || "auto";
  renderLanguageOptions();
  languageSearch.focus();
}

function closeLanguageDropdown() {
  languageCombobox.classList.remove("open");
  languageCombobox.setAttribute("aria-expanded", "false");
  languageDropdown.classList.add("hidden");
}

function renderLanguageOptions() {
  const selectedCode = languageSelect.value || "auto";
  languageList.innerHTML = "";

  if (filteredLanguages.length === 0) {
    const empty = document.createElement("li");
    empty.className = "combobox-empty";
    empty.textContent = "No matching languages";
    languageList.appendChild(empty);
    return;
  }

  filteredLanguages.forEach((lang) => {
    const item = document.createElement("li");
    const button = document.createElement("button");
    button.type = "button";
    button.className = "combobox-option";
    button.setAttribute("role", "option");
    button.dataset.code = lang.code;

    if (lang.code === selectedCode) {
      button.classList.add("selected");
      button.setAttribute("aria-selected", "true");
    }

    if (lang.code === highlightedLanguageCode) {
      button.classList.add("highlighted");
    }

    const flag = document.createElement("span");
    flag.className = "lang-flag";
    flag.textContent = lang.flag;

    const name = document.createElement("span");
    name.className = "lang-name";
    name.textContent = lang.name;

    button.append(flag, name);
    button.addEventListener("click", () => {
      setLanguageSelection(lang.code, true);
      closeLanguageDropdown();
      languageTrigger.focus();
    });

    item.appendChild(button);
    languageList.appendChild(item);
  });
}

function updateLanguageFilter(query: string) {
  const normalizedQuery = normalizeLanguageText(query.trim());
  filteredLanguages = LANGUAGES.filter((lang) => {
    if (!normalizedQuery) return true;
    return normalizeLanguageText(lang.name).includes(normalizedQuery);
  });

  if (!filteredLanguages.some((lang) => lang.code === highlightedLanguageCode)) {
    highlightedLanguageCode = filteredLanguages[0]?.code ?? "auto";
  }

  renderLanguageOptions();
}

function setLanguageSelection(code: string, persist: boolean) {
  languageSelect.value = code;
  setLanguageTrigger(code);
  highlightedLanguageCode = code;

  if (persist) {
    saveSettings();
  }
}

function moveLanguageHighlight(delta: number) {
  if (filteredLanguages.length === 0) return;

  const currentIndex = filteredLanguages.findIndex((lang) => lang.code === highlightedLanguageCode);
  const safeIndex = currentIndex < 0 ? 0 : currentIndex;
  const nextIndex = (safeIndex + delta + filteredLanguages.length) % filteredLanguages.length;
  highlightedLanguageCode = filteredLanguages[nextIndex].code;
  renderLanguageOptions();

  const highlighted = languageList.querySelector(".combobox-option.highlighted") as HTMLButtonElement | null;
  highlighted?.scrollIntoView({ block: "nearest" });
}

function selectByInitialLetter(letter: string) {
  const normalizedLetter = normalizeLanguageText(letter);
  const match = LANGUAGES.find((lang) => normalizeLanguageText(lang.name).startsWith(normalizedLetter));
  if (match) {
    setLanguageSelection(match.code, true);
  }
}

const statusDot = document.getElementById("status-dot")!;
const statusText = document.getElementById("status-text")!;
const micSelect = document.getElementById("mic-select") as HTMLSelectElement;
const engineLocal = document.getElementById("engine-local")!;
const engineCloud = document.getElementById("engine-cloud")!;
const localSettings = document.getElementById("local-settings")!;
const cloudSettings = document.getElementById("cloud-settings")!;
const modelSelect = document.getElementById("model-select") as HTMLSelectElement;
const downloadBtn = document.getElementById("download-btn")!;
const downloadProgress = document.getElementById("download-progress")!;
const progressFill = document.getElementById("progress-fill")!;
const groqKey = document.getElementById("groq-key") as HTMLInputElement;
const modeToggle = document.getElementById("mode-toggle")!;
const modePtt = document.getElementById("mode-ptt")!;
const hotkeyText = document.getElementById("hotkey-text")!;
const hotkeyChangeBtn = document.getElementById("hotkey-change-btn")!;

// Section navigation
const navItems = document.querySelectorAll(".nav-item");
const sections = document.querySelectorAll(".content-section");

navItems.forEach((item) => {
  item.addEventListener("click", () => {
    const target = item.getAttribute("data-section");
    navItems.forEach((n) => n.classList.remove("active"));
    sections.forEach((s) => s.classList.remove("active"));
    item.classList.add("active");
    document.getElementById(`section-${target}`)?.classList.add("active");
  });
});

// Window drag — titlebar and sidebar empty space
const titlebar = document.getElementById("titlebar")!;
const sidebar = document.getElementById("sidebar")!;
const appWindow = getCurrentWindow();

titlebar.addEventListener("mousedown", (e) => {
  if ((e.target as HTMLElement).closest("button, select, input, a, .nav-item")) return;
  appWindow.startDragging();
});

sidebar.addEventListener("mousedown", (e) => {
  if ((e.target as HTMLElement).closest("button, select, input, a, .nav-item")) return;
  appWindow.startDragging();
});

let currentSettings: Settings;

async function loadSettings() {
  currentSettings = await invoke<Settings>("get_settings");

  // Populate mic dropdown
  const mics = await invoke<MicDevice[]>("list_microphones");
  micSelect.innerHTML = "";
  mics.forEach((mic) => {
    const option = document.createElement("option");
    option.value = mic.name;
    option.textContent = mic.name + (mic.is_default ? " (default)" : "");
    micSelect.appendChild(option);
  });
  micSelect.value = currentSettings.microphone;

  // Engine
  setEngine(currentSettings.engine);

  // Model
  modelSelect.value = currentSettings.whisperModel;
  await checkModelStatus();

  // Groq key
  groqKey.value = currentSettings.groqApiKey;

  // Language
  const initialLanguage = currentSettings.language || "auto";
  languageSelect.value = languageByCode.has(initialLanguage) ? initialLanguage : "auto";
  setLanguageTrigger(languageSelect.value);

  // Recording mode
  setRecordingMode(currentSettings.recordingMode);

  // Hotkey
  hotkeyText.textContent = formatHotkey(currentSettings.hotkey);
}

function setEngine(engine: string) {
  currentSettings.engine = engine;
  engineLocal.classList.toggle("active", engine === "local");
  engineCloud.classList.toggle("active", engine === "cloud");
  localSettings.classList.toggle("hidden", engine !== "local");
  cloudSettings.classList.toggle("hidden", engine !== "cloud");
}

function setRecordingMode(mode: string) {
  currentSettings.recordingMode = mode;
  modeToggle.classList.toggle("active", mode === "toggle");
  modePtt.classList.toggle("active", mode === "push-to-talk");
}

async function checkModelStatus() {
  const downloaded = await invoke<boolean>("check_model_downloaded", {
    modelSize: modelSelect.value,
  });
  downloadBtn.textContent = downloaded ? "\u2713" : "Download";
  (downloadBtn as HTMLButtonElement).disabled = downloaded;
}

async function saveSettings() {
  currentSettings.microphone = micSelect.value;
  currentSettings.whisperModel = modelSelect.value;
  currentSettings.groqApiKey = groqKey.value;
  currentSettings.language = languageSelect.value;
  await invoke("save_settings", { settings: currentSettings });
}

// Event listeners
engineLocal.addEventListener("click", () => {
  setEngine("local");
  saveSettings();
});

engineCloud.addEventListener("click", () => {
  setEngine("cloud");
  saveSettings();
});

micSelect.addEventListener("change", () => saveSettings());

modelSelect.addEventListener("change", async () => {
  await checkModelStatus();
  saveSettings();
});

downloadBtn.addEventListener("click", async () => {
  (downloadBtn as HTMLButtonElement).disabled = true;
  downloadProgress.classList.remove("hidden");
  progressFill.style.width = "0%";

  try {
    await invoke("download_model", { modelSize: modelSelect.value });
    downloadBtn.textContent = "\u2713";
  } catch (e) {
    downloadBtn.textContent = "Retry";
    (downloadBtn as HTMLButtonElement).disabled = false;
    console.error("Download failed:", e);
  }
  downloadProgress.classList.add("hidden");
});

groqKey.addEventListener("change", () => saveSettings());

modeToggle.addEventListener("click", () => {
  setRecordingMode("toggle");
  saveSettings();
});

modePtt.addEventListener("click", () => {
  setRecordingMode("push-to-talk");
  saveSettings();
});

languageTrigger.addEventListener("click", () => {
  if (languageCombobox.classList.contains("open")) {
    closeLanguageDropdown();
  } else {
    openLanguageDropdown();
  }
});

languageSearch.addEventListener("input", () => {
  updateLanguageFilter(languageSearch.value);
});

languageSearch.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    moveLanguageHighlight(1);
    return;
  }

  if (e.key === "ArrowUp") {
    e.preventDefault();
    moveLanguageHighlight(-1);
    return;
  }

  if (e.key === "Enter") {
    e.preventDefault();
    const selected = filteredLanguages.find((lang) => lang.code === highlightedLanguageCode);
    if (selected) {
      setLanguageSelection(selected.code, true);
      closeLanguageDropdown();
      languageTrigger.focus();
    }
    return;
  }

  if (e.key === "Escape") {
    e.preventDefault();
    closeLanguageDropdown();
    languageTrigger.focus();
  }
});

languageTrigger.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    if (!languageCombobox.classList.contains("open")) {
      openLanguageDropdown();
    } else {
      moveLanguageHighlight(1);
    }
    return;
  }

  if (e.key === "ArrowUp") {
    e.preventDefault();
    if (!languageCombobox.classList.contains("open")) {
      openLanguageDropdown();
    } else {
      moveLanguageHighlight(-1);
    }
    return;
  }

  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    if (!languageCombobox.classList.contains("open")) {
      openLanguageDropdown();
    } else {
      closeLanguageDropdown();
    }
    return;
  }

  if (/^[a-z]$/i.test(e.key) && !languageCombobox.classList.contains("open")) {
    e.preventDefault();
    selectByInitialLetter(e.key);
  }
});

document.addEventListener("mousedown", (e) => {
  if (!languageCombobox.contains(e.target as Node)) {
    closeLanguageDropdown();
  }
});

// Listen for recording state changes
listen<string>("recording-state", (event) => {
  const state = event.payload;
  statusDot.className = "";
  if (state === "Recording") {
    statusDot.classList.add("recording");
    statusText.textContent = "Recording...";
  } else if (state === "Transcribing") {
    statusDot.classList.add("transcribing");
    statusText.textContent = "Transcribing...";
  } else {
    statusDot.classList.add("ready");
    statusText.textContent = "Ready";
  }
});

// Listen for download progress
listen<DownloadProgress>("download-progress", (event) => {
  const { percent } = event.payload;
  progressFill.style.width = `${percent}%`;
});

// Hotkey utilities
const isMac = navigator.platform.toUpperCase().includes("MAC");

function formatHotkey(hotkey: string): string {
  return hotkey
    .replace("CmdOrCtrl", isMac ? "Cmd" : "Ctrl")
    .replace("Return", "Enter");
}

function keyEventToAccelerator(e: KeyboardEvent): string | null {
  if (["Control", "Meta", "Alt", "Shift"].includes(e.key)) return null;

  const modifiers: string[] = [];
  if (e.ctrlKey || e.metaKey) modifiers.push("CmdOrCtrl");
  if (e.altKey) modifiers.push("Alt");
  if (e.shiftKey) modifiers.push("Shift");
  if (modifiers.length === 0) return null;

  const namedKeys: Record<string, string> = {
    " ": "Space",
    "ArrowLeft": "ArrowLeft",
    "ArrowRight": "ArrowRight",
    "ArrowUp": "ArrowUp",
    "ArrowDown": "ArrowDown",
    "Enter": "Return",
    "Tab": "Tab",
    "Backspace": "Backspace",
    "Delete": "Delete",
    "Home": "Home",
    "End": "End",
    "PageUp": "PageUp",
    "PageDown": "PageDown",
    "Insert": "Insert",
    "Escape": "Escape",
  };

  let key: string | null = null;
  if (namedKeys[e.key] !== undefined) {
    key = namedKeys[e.key];
  } else if (e.key.length === 1) {
    key = e.key.toUpperCase();
  } else if (/^F\d+$/.test(e.key)) {
    key = e.key;
  }

  if (!key) return null;
  return [...modifiers, key].join("+");
}

let capturing = false;
let stopCapture: (() => void) | null = null;

hotkeyChangeBtn.addEventListener("click", () => {
  if (capturing) {
    stopCapture?.();
    return;
  }

  capturing = true;
  hotkeyText.textContent = "Press shortcut…";
  hotkeyText.classList.add("capturing");
  hotkeyChangeBtn.textContent = "Cancel";

  function onKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      stopCapture?.();
      return;
    }
    const accelerator = keyEventToAccelerator(e);
    if (!accelerator) return;
    document.removeEventListener("keydown", onKeyDown, true);
    stopCapture = null;
    capturing = false;
    applyHotkey(accelerator);
  }

  document.addEventListener("keydown", onKeyDown, true);

  stopCapture = () => {
    document.removeEventListener("keydown", onKeyDown, true);
    capturing = false;
    stopCapture = null;
    hotkeyText.textContent = formatHotkey(currentSettings.hotkey);
    hotkeyText.classList.remove("capturing");
    hotkeyChangeBtn.textContent = "Change";
  };
});

async function applyHotkey(accelerator: string) {
  hotkeyText.textContent = formatHotkey(accelerator);
  hotkeyText.classList.remove("capturing");
  hotkeyChangeBtn.textContent = "Change";

  try {
    await invoke("update_hotkey", { newHotkey: accelerator });
    currentSettings.hotkey = accelerator;
  } catch (e) {
    console.error("Failed to set hotkey:", e);
    hotkeyText.textContent = "Failed — try another";
    hotkeyText.classList.add("capturing");
    hotkeyChangeBtn.textContent = "Cancel";
    capturing = true;
    setTimeout(() => stopCapture?.(), 2000);
  }
}

// Initialize
loadSettings();
