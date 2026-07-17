import "./styles.css";

type ConfigValue = string | boolean | number;
type ViewName = "library" | "achievements" | "settings";
type LaunchMode = "normal" | "clean" | "diagnostic";
type MediaKey =
  | "video"
  | "fanart"
  | "screenshot"
  | "image"
  | "titleshot"
  | "marquee"
  | "wheel"
  | "thumbnail"
  | "boxart"
  | "box2d"
  | "box3d"
  | "cartridge"
  | "cartridge2d"
  | "mix"
  | "manual";

type ConfigResponse = {
  config: Record<string, string>;
  secrets: {
    raTokenConfigured: boolean;
    raPasswordConfigured: boolean;
    raApiKeyConfigured: boolean;
  };
  paths: {
    projectRoot: string;
    configFile: string;
    emulatorExe: string;
  };
};

type RaConnectResponse = {
  ok: boolean;
  username: string;
  score: number;
  softcoreScore: number;
  method: "password" | "token" | string;
};

type RomItem = {
  name: string;
  base: string;
  title: string;
  format: "neo" | "zip" | string;
  path: string;
  description: string;
  developer: string;
  publisher: string;
  genre: string;
  players: string;
  rating: number;
  releaseDate: string;
  playCount: number;
  lastPlayed: string;
  hasMedia: boolean;
  media: Partial<Record<MediaKey, string>>;
};

type LibraryResponse = {
  roots: {
    romDir: string;
    biosDir: string;
    mediaDir: string;
    gamelistPath: string;
  };
  roms: RomItem[];
  biosArchives: string[];
  biosOptions?: BiosOption[];
  mediaCount: number;
  mediaMatched: number;
};

type RaProfile = {
  username: string;
  ulid: string;
  avatar: string;
  memberSince: string;
  motto: string;
  richPresence: string;
  status: string;
  points: number;
  softcorePoints: number;
  truePoints: number;
  rank: number;
  totalRanked: number;
};

type RaRecentGame = {
  id: number;
  title: string;
  consoleName: string;
  image: string;
  icon: string;
  lastPlayed: string;
  achievementsTotal: number;
  achieved: number;
  hardcore: number;
  possibleScore: number;
  scoreAchieved: number;
  progress: number;
};

type RaAchievement = {
  id: number;
  title: string;
  description: string;
  gameTitle: string;
  gameId: number;
  consoleName: string;
  points: number;
  trueRatio: number;
  hardcore: boolean;
  date: string;
  badge: string;
};

type BiosKind = "aes" | "mvs" | "unibios" | "unknown";
type BiosProfile = "neogeo" | "aes" | "unibios";
type BiosRegion = "usa" | "japan" | "asia" | "europe";

type BiosOption = {
  label: string;
  kind: BiosKind;
  title: string;
  purpose: string;
  raNote: string;
  hardcoreSafe: boolean;
};

type RaDashboard = {
  available: boolean;
  connected?: boolean;
  needsApiKey?: boolean;
  message?: string;
  apiKeyUrl?: string;
  basic?: { username: string; score: number; softcoreScore: number };
  profile?: RaProfile;
  recentlyPlayed?: RaRecentGame[];
  recentAchievements?: RaAchievement[];
  profileUrl?: string;
  refreshedAt?: string;
};

type EmulatorSession = {
  running: boolean;
  pid: number;
  mode: LaunchMode;
  romPath: string;
  logPath: string;
  startedAt: string;
  endedAt: string;
  exitCode: number | null;
  signal: string;
};

type SessionResponse = {
  session: EmulatorSession;
};

const booleanKeys = new Set([
  "scanlines",
  "curvature",
  "bloom",
  "fullscreen",
  "auto_save",
  "muted",
  "ra_hardcore",
  "diagnostic_dumps",
  "gamepad",
  "cpu_overclock",
  "disable_adpcm_wrap",
  "memcard",
  "memcard_write_protect",
  "setting_mode",
  "freeplay",
]);

const labels: Record<string, string> = {
  scanlines: "Scanlines",
  curvature: "Curvatura CRT",
  bloom: "Phosphor bloom",
  fullscreen: "Pantalla completa",
  auto_save: "Auto-save",
  muted: "Silenciar",
  diagnostic_dumps: "Dumps de diagnóstico",
  gamepad: "Gamepad SDL2",
  ra_hardcore: "Modo hardcore",
  bios_region: "Región BIOS",
  cpu_overclock: "CPU 68K a 24 MHz",
  disable_adpcm_wrap: "Desactivar ADPCM wrap",
  memcard: "Tarjeta de memoria",
  memcard_write_protect: "Proteger tarjeta",
  setting_mode: "Modo configuración MVS",
  freeplay: "Freeplay MVS",
};

const mediaLabels: Partial<Record<MediaKey, string>> = {
  video: "Vídeo",
  fanart: "Fanart",
  screenshot: "Captura",
  image: "Mix",
  titleshot: "Título",
  box3d: "Caja 3D",
  cartridge: "Cartucho",
  manual: "Manual",
};

const biosGuide: Record<BiosKind, Omit<BiosOption, "label" | "kind">> = {
  aes: {
    title: "AES consola doméstica",
    purpose: "Modo doméstico sin monedas, con las variantes y menús de consola que incluya el cartucho.",
    raNote: "RetroAchievements identifica la ROM; AES no es un requisito general. La BIOS oficial sí es válida en Hardcore.",
    hardcoreSafe: true,
  },
  mvs: {
    title: "NeoGeo MVS arcade",
    purpose: "Modo cabina: monedas, DIP, test y bookkeeping. Es el hardware original de los sets arcade.",
    raNote: "RetroAchievements identifica la ROM; MVS no impide logros por sí mismo y su BIOS oficial es válida en Hardcore.",
    hardcoreSafe: true,
  },
  unibios: {
    title: "UniBIOS",
    purpose: "BIOS comunitaria con menú de región, modo AES/MVS y extras de diagnóstico.",
    raNote: "No usar para RetroAchievements Hardcore; NGNEON la bloquea en ese modo.",
    hardcoreSafe: false,
  },
  unknown: {
    title: "BIOS detectada",
    purpose: "El configurador no reconoce el modo por el nombre. Verifica el archivo antes de usar RA.",
    raNote: "Para Hardcore usa una BIOS oficial reconocida. El tipo AES/MVS no sustituye la identificación de la ROM.",
    hardcoreSafe: true,
  },
};

const favoritesStorageKey = "ngneon-launcher-favorites-v1";
const recentsStorageKey = "ngneon-launcher-recents-v1";

const state: {
  loaded: boolean;
  dirty: boolean;
  saving: boolean;
  launching: boolean;
  libraryLoading: boolean;
  raBusy: boolean;
  raLoading: boolean;
  raStatus: string;
  error: string | null;
  toast: string;
  view: ViewName;
  response: ConfigResponse | null;
  library: LibraryResponse | null;
  ra: RaDashboard | null;
  session: EmulatorSession | null;
  selectedRomPath: string;
  mediaMode: MediaKey;
  launchMode: LaunchMode;
  search: string;
  filter: "all" | "favorites" | "recent";
  draft: Record<string, ConfigValue>;
  secretDraft: Record<string, string>;
  favorites: Set<string>;
  recents: string[];
} = {
  loaded: false,
  dirty: false,
  saving: false,
  launching: false,
  libraryLoading: false,
  raBusy: false,
  raLoading: false,
  raStatus: "",
  error: null,
  toast: "",
  view: "library",
  response: null,
  library: null,
  ra: null,
  session: null,
  selectedRomPath: "",
  mediaMode: "video",
  launchMode: "normal",
  search: "",
  filter: "all",
  draft: {},
  secretDraft: {},
  favorites: new Set(readStoredList(favoritesStorageKey)),
  recents: readStoredList(recentsStorageKey),
};

const appRoot = document.querySelector<HTMLDivElement>("#app");
if (!appRoot) throw new Error("Missing #app root");
const app = appRoot;

function readStoredList(key: string): string[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(key) || "[]");
    return Array.isArray(parsed) ? parsed.filter((value) => typeof value === "string") : [];
  } catch {
    return [];
  }
}

function persistLocalLists() {
  localStorage.setItem(favoritesStorageKey, JSON.stringify([...state.favorites]));
  localStorage.setItem(recentsStorageKey, JSON.stringify(state.recents.slice(0, 20)));
}

function parseBool(value: string | undefined): boolean {
  return value === "on" || value === "true" || value === "1";
}

function configString(key: string, fallback = ""): string {
  const value = state.draft[key];
  return typeof value === "string" ? value : fallback;
}

function getNumber(key: string, fallback: number): number {
  const value = Number(state.draft[key]);
  return Number.isFinite(value) ? value : fallback;
}

function initDraft(response: ConfigResponse): Record<string, ConfigValue> {
  const draft: Record<string, ConfigValue> = { ...response.config };
  for (const key of booleanKeys) draft[key] = parseBool(response.config[key]);
  if (response.config.memcard === undefined) draft.memcard = true;
  draft.volume = Number(response.config.volume ?? 100);
  draft.window_scale = Number(response.config.window_scale ?? 3);
  draft.bios_region =
    response.config.bios_region || biosRegionFromLabel(response.config.bios || "") || "europe";
  return draft;
}

async function api<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    headers: { "content-type": "application/json" },
    ...init,
  });
  if (!res.ok) {
    const text = await res.text();
    let message = text;
    try {
      message = JSON.parse(text).error || text;
    } catch {
      // Keep plain server errors readable.
    }
    throw new Error(message || `HTTP ${res.status}`);
  }
  return (await res.json()) as T;
}

async function loadConfig() {
  try {
    state.error = null;
    const response = await api<ConfigResponse>("/api/config");
    state.response = response;
    state.draft = initDraft(response);
    state.loaded = true;
    state.dirty = false;
    await Promise.all([loadLibrary(false), loadRa(false), loadSession(false)]);
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
  }
  render();
}

async function loadLibrary(withRender = true) {
  state.libraryLoading = true;
  if (withRender) render();
  try {
    state.library = await api<LibraryResponse>("/api/library");
    if (!state.selectedRomPath || !state.library.roms.some((rom) => rom.path === state.selectedRomPath)) {
      state.selectedRomPath = state.library.roms[0]?.path ?? "";
    }
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
  } finally {
    state.libraryLoading = false;
    if (withRender) render();
  }
}

async function loadRa(withRender = true) {
  state.raLoading = true;
  if (withRender) render();
  try {
    state.ra = await api<RaDashboard>("/api/ra/dashboard");
  } catch (error) {
    state.ra = {
      available: false,
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    state.raLoading = false;
    if (withRender) render();
  }
}

async function loadSession(withRender = true) {
  try {
    const previous = JSON.stringify(state.session);
    const response = await api<SessionResponse>("/api/session");
    state.session = response.session;
    if (withRender || previous !== JSON.stringify(state.session)) render();
  } catch (error) {
    if (withRender) {
      state.error = error instanceof Error ? error.message : String(error);
      render();
    }
  }
}

function serializeDraft(): Record<string, string> {
  return Object.fromEntries(
    Object.entries(state.draft).map(([key, value]) => [
      key,
      typeof value === "boolean" ? (value ? "on" : "off") : String(value),
    ]),
  );
}

function serializeSecretDraft(): Record<string, string> {
  return Object.fromEntries(Object.entries(state.secretDraft).filter(([, value]) => value.length > 0));
}

async function saveConfig() {
  state.saving = true;
  state.error = null;
  render();
  try {
    await api("/api/config", {
      method: "POST",
      body: JSON.stringify({ config: serializeDraft(), secrets: serializeSecretDraft() }),
    });
    state.secretDraft = {};
    state.dirty = false;
    state.toast = "Configuración guardada";
    await loadConfig();
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
  } finally {
    state.saving = false;
    render();
  }
}

async function connectRetroAchievements() {
  state.raBusy = true;
  state.error = null;
  state.raStatus = "Conectando con RetroAchievements…";
  render();
  try {
    const result = await api<RaConnectResponse>("/api/ra/connect", {
      method: "POST",
      body: JSON.stringify({
        username: configString("ra_username"),
        password: state.secretDraft.ra_password ?? "",
        token: state.secretDraft.ra_token ?? "",
        apiKey: state.secretDraft.ra_api_key ?? "",
      }),
    });
    state.secretDraft = {};
    state.dirty = false;
    state.raStatus = `Conectado como ${result.username} · ${formatNumber(result.score)} puntos`;
    await loadConfig();
    state.view = "achievements";
  } catch (error) {
    state.raStatus = error instanceof Error ? error.message : String(error);
  } finally {
    state.raBusy = false;
    render();
  }
}

async function disconnectRetroAchievements() {
  state.raBusy = true;
  state.raStatus = "Desconectando…";
  render();
  try {
    await api("/api/ra/disconnect", { method: "POST", body: "{}" });
    state.secretDraft = {};
    state.dirty = false;
    state.raStatus = "RetroAchievements desconectado.";
    await loadConfig();
  } catch (error) {
    state.raStatus = error instanceof Error ? error.message : String(error);
  } finally {
    state.raBusy = false;
    render();
  }
}

async function launchEmulator() {
  const rom = selectedRom();
  if (!rom || state.session?.running) return;
  state.launching = true;
  state.error = null;
  render();
  try {
    if (state.dirty) {
      await api("/api/config", {
        method: "POST",
        body: JSON.stringify({ config: serializeDraft(), secrets: serializeSecretDraft() }),
      });
      state.secretDraft = {};
      state.dirty = false;
    }
    const response = await api<SessionResponse>("/api/launch", {
      method: "POST",
      body: JSON.stringify({ romPath: rom.path, mode: state.launchMode }),
    });
    state.session = response.session;
    state.recents = [rom.path, ...state.recents.filter((item) => item !== rom.path)].slice(0, 20);
    persistLocalLists();
    state.toast = `${rom.title} iniciado · ${launchModeLabel(state.launchMode)}`;
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
  } finally {
    state.launching = false;
    render();
  }
}

async function stopEmulator() {
  if (!state.session?.running) return;
  try {
    await api<SessionResponse>("/api/stop", { method: "POST", body: "{}" });
    state.toast = "Cerrando sesión";
    window.setTimeout(() => void loadSession(), 300);
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
    render();
  }
}

function launchModeLabel(mode: LaunchMode): string {
  if (mode === "clean") return "Sesión limpia";
  if (mode === "diagnostic") return "Diagnóstico";
  return "Normal";
}

function escapeHtml(value: string): string {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function biosKindFromLabel(label: string): BiosKind {
  const lower = label.toLowerCase();
  if (lower.includes("uni-bios") || lower.includes("unibios")) return "unibios";
  if (lower.includes("aes.zip") || lower.includes("neo-epo.bin") || lower.includes("neo-po.bin")) return "aes";
  if (
    lower.includes("neogeo.zip") ||
    lower.includes("sp-s2.sp1") ||
    lower.includes("sp-s.sp1") ||
    lower.includes("sp-u") ||
    lower.includes("sp1-u") ||
    lower.includes("sp-e") ||
    lower.includes("sp-j") ||
    lower.includes("sp1.jipan") ||
    lower.includes("japan") ||
    lower.includes("asia-s") ||
    lower.includes("vs-bios.rom")
  ) {
    return "mvs";
  }
  return "unknown";
}

function biosRegionFromLabel(label: string): BiosRegion | "" {
  const lower = label.toLowerCase();
  if (lower.includes("uni-bios") || lower.includes("unibios")) return "";
  if (
    lower.includes("neo-po.bin") ||
    lower.includes("japan") ||
    lower.includes("sp-j") ||
    lower.includes("sp1-j") ||
    lower.includes("sp1.jipan")
  ) {
    return "japan";
  }
  if (lower.includes("sp-45.sp1") || lower.includes("asia-s")) return "asia";
  if (lower.includes("neo-epo.bin") || lower.includes("sp-u") || lower.includes("sp1-u")) return "usa";
  if (lower.includes("sp-s2.sp1") || lower.includes("sp-s.sp1") || lower.includes("sp-s3.sp1") || lower.includes("sp-e")) {
    return "europe";
  }
  return "";
}

function biosLabelSupportsRegion(label: string, region: BiosRegion): boolean {
  if (label.toLowerCase().includes("neo-epo.bin")) return region === "usa" || region === "europe";
  return biosRegionFromLabel(label) === region;
}

function normalizeBiosRegion(value: string): BiosRegion {
  const lower = value.toLowerCase();
  if (lower === "us" || lower === "usa" || lower === "u") return "usa";
  if (lower === "jp" || lower === "japan" || lower === "j") return "japan";
  if (lower === "as" || lower === "asia" || lower === "a") return "asia";
  if (lower === "eu" || lower === "eur" || lower === "europe" || lower === "e") return "europe";
  return "europe";
}

function biosProfileFromKind(kind: BiosKind): BiosProfile | "" {
  if (kind === "mvs") return "neogeo";
  if (kind === "aes" || kind === "unibios") return kind;
  return "";
}

function biosProfileFromLabel(label: string): BiosProfile | "" {
  return biosProfileFromKind(biosKindFromLabel(label));
}

function isBiosProfile(value: string): value is BiosProfile {
  return value === "neogeo" || value === "aes" || value === "unibios";
}

function isBiosRegion(value: string): value is BiosRegion {
  return value === "usa" || value === "japan" || value === "asia" || value === "europe";
}

function describeBios(label: string): BiosOption {
  const kind = biosKindFromLabel(label);
  return { label, kind, ...biosGuide[kind] };
}

function biosOptionsForSettings(current: string, discovered: BiosOption[] = []): BiosOption[] {
  const byLabel = new Map<string, BiosOption>();
  for (const option of discovered) byLabel.set(option.label, option);
  if (current && !byLabel.has(current)) byLabel.set(current, describeBios(current));
  return [...byLabel.values()];
}

const biosProfileOrder: BiosProfile[] = ["neogeo", "aes", "unibios"];
const biosRegionOrder: BiosRegion[] = ["usa", "japan", "asia", "europe"];

function biosKindForProfile(profile: BiosProfile): BiosKind {
  return profile === "neogeo" ? "mvs" : profile;
}

function biosProfileLabel(profile: BiosProfile): string {
  return profile === "neogeo" ? "NEOGEO" : profile === "aes" ? "AES" : "UNIBIOS";
}

function biosProfileTitle(profile: BiosProfile): string {
  return biosGuide[biosKindForProfile(profile)].title;
}

function biosRegionLabel(region: BiosRegion): string {
  const labelsByRegion: Record<BiosRegion, string> = {
    usa: "USA",
    japan: "Japón",
    asia: "Asia",
    europe: "Europa",
  };
  return labelsByRegion[region];
}

function biosProfilesForSettings(current: string, options: BiosOption[]): BiosProfile[] {
  const present = new Set<BiosProfile>();
  for (const option of options) {
    const profile = biosProfileFromKind(option.kind);
    if (profile) present.add(profile);
  }
  const currentProfile = biosProfileFromLabel(current);
  if (currentProfile) present.add(currentProfile);
  return biosProfileOrder.filter((profile) => present.has(profile));
}

function biosRegionsForProfile(profile: BiosProfile, options: BiosOption[]): Set<BiosRegion> {
  const regions = new Set<BiosRegion>();
  for (const option of options) {
    if (biosProfileFromKind(option.kind) !== profile) continue;
    for (const region of biosRegionOrder) {
      if (biosLabelSupportsRegion(option.label, region)) regions.add(region);
    }
  }
  return regions;
}

function resolveBiosOption(profile: BiosProfile, region: BiosRegion, options: BiosOption[]): BiosOption | undefined {
  return (
    options.find((option) => biosProfileFromKind(option.kind) === profile && biosLabelSupportsRegion(option.label, region)) ??
    options.find((option) => biosProfileFromKind(option.kind) === profile)
  );
}

function setBiosSelection(profile: BiosProfile, requestedRegion: BiosRegion): void {
  const options = biosOptionsForSettings(configString("bios"), state.library?.biosOptions ?? []);
  const availableRegions = biosRegionsForProfile(profile, options);
  const region =
    profile === "unibios"
      ? requestedRegion
      : availableRegions.has(requestedRegion)
        ? requestedRegion
        : [...availableRegions][0] ?? requestedRegion;
  const option = resolveBiosOption(profile, region, options);
  if (option) state.draft.bios = option.label;
  state.draft.bios_region = region;
  state.dirty = true;
  render();
}

function renderBiosGuide(profile: BiosProfile, region: BiosRegion, selected: BiosOption | undefined): string {
  const currentKind = biosKindForProfile(profile);
  const current = selected ?? { label: "", kind: currentKind, ...biosGuide[currentKind] };
  const guideCards = biosProfileOrder
    .map((profileOption) => {
      const kind = biosKindForProfile(profileOption);
      const info = biosGuide[kind];
      return `
        <div class="bios-mode-card bios-${kind}">
          <span>${biosProfileLabel(profileOption)}</span>
          <strong>${escapeHtml(info.title)}</strong>
          <p>${escapeHtml(info.purpose)}</p>
        </div>
      `;
    })
    .join("");
  return `
    <div class="bios-guide">
      <div class="bios-selected-card bios-${current.kind}">
        <span>${biosProfileLabel(profile)}</span>
        <div>
          <strong>${escapeHtml(current.title)}</strong>
          <small>${profile === "unibios" ? `Hardware: ${configString("unibios_hardware", "mvs").toUpperCase()}` : `Región: ${biosRegionLabel(region)}`}</small>
          <p>${escapeHtml(current.purpose)}</p>
          <em class="${current.hardcoreSafe ? "is-safe" : "is-blocked"}">${escapeHtml(current.raNote)}</em>
        </div>
      </div>
      <div class="bios-mode-grid">${guideCards}</div>
    </div>
  `;
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("es-ES").format(value || 0);
}

function formatDate(value: string): string {
  if (!value) return "";
  const date = new Date(value.replace(" ", "T") + (value.includes("Z") ? "" : "Z"));
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("es-ES", { day: "2-digit", month: "short", year: "numeric" }).format(date);
}

function selectedRom(): RomItem | null {
  return state.library?.roms.find((rom) => rom.path === state.selectedRomPath) ?? null;
}

function filteredRoms(): RomItem[] {
  const query = state.search.trim().toLocaleLowerCase("es");
  const recents = new Set(state.recents);
  return (state.library?.roms ?? []).filter((rom) => {
    if (state.filter === "favorites" && !state.favorites.has(rom.path)) return false;
    if (state.filter === "recent" && !recents.has(rom.path)) return false;
    if (!query) return true;
    return [rom.title, rom.base, rom.developer, rom.publisher, rom.genre]
      .join(" ")
      .toLocaleLowerCase("es")
      .includes(query);
  });
}

function artwork(rom: RomItem, ...keys: MediaKey[]): string {
  for (const key of keys) {
    const url = rom.media[key];
    if (url) return url;
  }
  return "";
}

function gameMeta(rom: RomItem): string[] {
  return [rom.developer, rom.publisher, rom.genre, rom.players ? `${rom.players} jugador${rom.players === "1" ? "" : "es"}` : ""]
    .filter(Boolean)
    .slice(0, 4);
}

function navIcon(name: "library" | "achievement" | "settings" | "search" | "play" | "stop" | "heart" | "refresh"): string {
  const paths = {
    library: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 8h10M7 12h10M7 16h6"/>',
    achievement: '<path d="M8 3h8v4a4 4 0 0 1-8 0V3Z"/><path d="M8 5H4v1a4 4 0 0 0 4 4M16 5h4v1a4 4 0 0 1-4 4M12 11v5M8 21h8M9 16h6v5H9z"/>',
    settings: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21h-4v-.1A1.7 1.7 0 0 0 8.6 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-1.6-1H3v-4h.1A1.7 1.7 0 0 0 4.6 9a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6V3h4v.1A1.7 1.7 0 0 0 15 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 9a1.7 1.7 0 0 0 1.6 1h.1v4H21a1.7 1.7 0 0 0-1.6 1Z"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>',
    play: '<path d="m8 5 11 7-11 7V5Z"/>',
    stop: '<rect x="6" y="6" width="12" height="12" rx="1"/>',
    heart: '<path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8l1.1 1.1L12 21l7.8-7.5 1.1-1.1a5.5 5.5 0 0 0-.1-7.8Z"/>',
    refresh: '<path d="M20 6v5h-5M4 18v-5h5"/><path d="M18.5 9A7 7 0 0 0 6 6.5L4 9m16 6-2 2.5A7 7 0 0 1 5.5 15"/>',
  };
  return `<svg viewBox="0 0 24 24" aria-hidden="true">${paths[name]}</svg>`;
}

function renderHeader(): string {
  const ra = state.ra?.profile;
  const sessionRom = state.library?.roms.find((rom) => rom.path === state.session?.romPath);
  return `
    <header class="app-header">
      <button class="brand" data-action="view" data-view="library" aria-label="Ir a biblioteca">
        <span class="brand-mark">N</span>
        <span><strong>NGNEON</strong><small>NEO GEO SYSTEM</small></span>
      </button>
      <nav class="main-nav" aria-label="Navegación principal">
        <button class="${state.view === "library" ? "is-active" : ""}" data-action="view" data-view="library">
          ${navIcon("library")}<span>Biblioteca</span>
        </button>
        <button class="${state.view === "achievements" ? "is-active" : ""}" data-action="view" data-view="achievements">
          ${navIcon("achievement")}<span>RetroAchievements</span>
        </button>
        <button class="${state.view === "settings" ? "is-active" : ""}" data-action="view" data-view="settings">
          ${navIcon("settings")}<span>Ajustes</span>
        </button>
      </nav>
      <div class="header-actions">
        ${state.session?.running ? `
          <button class="session-chip" data-action="stop-session" title="Cerrar emulador">
            <i></i><span><strong>${escapeHtml(sessionRom?.title || "Emulador activo")}</strong><small>${escapeHtml(launchModeLabel(state.session.mode))}</small></span>${navIcon("stop")}
          </button>
        ` : ""}
        <button class="profile-chip" data-action="view" data-view="achievements">
          ${ra?.avatar ? `<img src="${escapeHtml(ra.avatar)}" alt="" />` : `<span>${escapeHtml((configString("ra_username", "RA")[0] || "R").toUpperCase())}</span>`}
          <span><strong>${escapeHtml(ra?.username || configString("ra_username", "Sin conectar"))}</strong><small>${ra ? `${formatNumber(ra.points)} pts` : "RetroAchievements"}</small></span>
        </button>
      </div>
    </header>
  `;
}

function renderHeroMedia(rom: RomItem): string {
  const requested = rom.media[state.mediaMode];
  const video = state.mediaMode === "video" ? requested : "";
  const image = video ? artwork(rom, "fanart", "screenshot", "image") : requested || artwork(rom, "fanart", "screenshot", "image", "mix");
  const portraitClass = ["image", "box3d", "cartridge"].includes(state.mediaMode) ? " hero-media--portrait" : "";
  if (video) {
    return `
      <div class="hero-media-stage">
        ${image ? `<img class="hero-media-backdrop" src="${escapeHtml(image)}" alt="" aria-hidden="true" />` : ""}
        <video class="hero-media" src="${escapeHtml(video)}" poster="${escapeHtml(image)}" autoplay muted loop playsinline preload="metadata"></video>
      </div>
      <div class="hero-vignette"></div>
    `;
  }
  if (image) {
    return `
      <div class="hero-media-stage">
        <img class="hero-media-backdrop" src="${escapeHtml(image)}" alt="" aria-hidden="true" />
        <img class="hero-media${portraitClass}" src="${escapeHtml(image)}" alt="" />
      </div>
      <div class="hero-vignette"></div>
    `;
  }
  return `<div class="hero-fallback"><span>NG</span></div><div class="hero-vignette"></div>`;
}

function renderMediaTabs(rom: RomItem): string {
  const available = (["video", "fanart", "screenshot", "image", "titleshot", "box3d", "cartridge", "manual"] as MediaKey[]).filter(
    (key) => rom.media[key],
  );
  return available
    .map(
      (key) => `
        <button class="${state.mediaMode === key ? "is-active" : ""}" data-action="${key === "manual" ? "open-media" : "media"}" data-media="${key}" title="${mediaLabels[key] ?? key}">
          <span>${key === "video" ? "▶" : key === "manual" ? "↗" : "●"}</span>${mediaLabels[key] ?? key}
        </button>
      `,
    )
    .join("");
}

function renderMiniRa(): string {
  if (state.raLoading) return `<aside class="ra-mini skeleton-panel"></aside>`;
  const ra = state.ra;
  if (!ra?.available || !ra.profile) {
    return `
      <aside class="ra-mini ra-mini-empty">
        <div class="section-label">${navIcon("achievement")}<span>RetroAchievements</span></div>
        <strong>${escapeHtml(ra?.basic?.username || configString("ra_username", "Conecta tu cuenta"))}</strong>
        <p>${escapeHtml(ra?.message || "Consulta tu progreso y últimos logros mientras eliges juego.")}</p>
        ${ra?.basic ? `<div class="score-big">${formatNumber(ra.basic.score)}<small>puntos</small></div>` : ""}
        <button class="text-button" data-action="view" data-view="settings">Configurar cuenta →</button>
      </aside>
    `;
  }
  const latest = ra.recentAchievements?.[0];
  return `
    <aside class="ra-mini">
      <div class="ra-mini-profile">
        <img src="${escapeHtml(ra.profile.avatar)}" alt="" />
        <div><strong>${escapeHtml(ra.profile.username)}</strong><span>Rango #${formatNumber(ra.profile.rank)}</span></div>
        <span class="online-dot ${ra.profile.status.toLowerCase() === "online" ? "is-online" : ""}"></span>
      </div>
      <div class="ra-points">
        <div><strong>${formatNumber(ra.profile.points)}</strong><span>puntos</span></div>
        <div><strong>${formatNumber(ra.profile.truePoints)}</strong><span>true points</span></div>
      </div>
      ${latest ? `
        <div class="latest-unlock">
          <span>Último logro</span>
          <div>
            <img src="${escapeHtml(latest.badge)}" alt="" />
            <p><strong>${escapeHtml(latest.title)}</strong><small>${escapeHtml(latest.gameTitle)} · ${latest.points} pts</small></p>
          </div>
        </div>
      ` : `<p class="quiet">Aún no hay logros recientes.</p>`}
      <button class="text-button" data-action="view" data-view="achievements">Ver actividad completa →</button>
    </aside>
  `;
}

function renderLibrary(): string {
  const rom = selectedRom();
  const games = filteredRoms();
  if (!rom) {
    return `<section class="empty-screen"><h1>No hay juegos</h1><p>Revisa la ruta de ROMs en Ajustes y vuelve a escanear.</p></section>`;
  }
  const setName = rom.title.toLocaleLowerCase("es") === rom.base.toLocaleLowerCase("es") ? "" : rom.base;
  const meta = [setName, ...gameMeta(rom)].filter(Boolean);
  const favorite = state.favorites.has(rom.path);
  return `
    <main class="launcher">
      <section class="hero">
        ${renderHeroMedia(rom)}
        <div class="hero-content">
          ${rom.media.wheel ? `<img class="game-logo" src="${escapeHtml(rom.media.wheel)}" alt="${escapeHtml(rom.title)}" />` : `<h1>${escapeHtml(rom.title)}</h1>`}
          <div class="game-meta">${meta.map((value) => `<span>${escapeHtml(value)}</span>`).join("")}<span>${rom.format.toUpperCase()}</span></div>
          <p class="game-description">${escapeHtml(rom.description || "Sin descripción disponible en gamelist.xml.")}</p>
          <div class="launch-panel">
            <div class="launch-mode" role="group" aria-label="Modo de sesión">
              ${(["normal", "clean", "diagnostic"] as LaunchMode[])
                .map((mode) => `<button class="${state.launchMode === mode ? "is-active" : ""}" data-action="launch-mode" data-mode="${mode}">${escapeHtml(launchModeLabel(mode))}</button>`)
                .join("")}
            </div>
            <div class="hero-actions">
              <button class="play-button" data-action="launch" ${state.launching || state.session?.running ? "disabled" : ""}>${navIcon("play")}<span>${state.session?.running ? "EN EJECUCIÓN" : state.launching ? "ABRIENDO…" : "JUGAR"}</span></button>
              ${state.session?.running ? `<button class="round-button stop-button" data-action="stop-session" title="Cerrar emulador">${navIcon("stop")}</button>` : ""}
              <button class="round-button ${favorite ? "is-favorite" : ""}" data-action="favorite" title="Favorito">${navIcon("heart")}</button>
            </div>
          </div>
          <div class="media-tabs">${renderMediaTabs(rom)}</div>
        </div>
        ${renderMiniRa()}
      </section>

      <section class="library-section">
        <div class="library-toolbar">
          <div>
            <h2>Tu colección</h2>
            <p>${state.library?.roms.length ?? 0} juegos · ${state.library?.mediaMatched ?? 0} con media</p>
          </div>
          <div class="library-controls">
            <div class="filter-group">
              <button class="${state.filter === "all" ? "is-active" : ""}" data-action="filter" data-filter="all">Todos</button>
              <button class="${state.filter === "favorites" ? "is-active" : ""}" data-action="filter" data-filter="favorites">Favoritos</button>
              <button class="${state.filter === "recent" ? "is-active" : ""}" data-action="filter" data-filter="recent">Recientes</button>
            </div>
            <label class="search-box">${navIcon("search")}<input type="search" data-action="search" value="${escapeHtml(state.search)}" placeholder="Buscar juego…" /></label>
            <button class="icon-button" data-action="rescan" title="Volver a escanear">${navIcon("refresh")}</button>
          </div>
        </div>
        <div class="game-rail" data-rail>
          ${games.length ? games.map(renderGameCard).join("") : `<div class="no-results">No hay juegos para este filtro.</div>`}
        </div>
      </section>
    </main>
  `;
}

function renderGameCard(rom: RomItem): string {
  const selected = rom.path === state.selectedRomPath;
  const cover = artwork(rom, "box3d", "box2d", "thumbnail", "image", "mix");
  const setName = rom.title.toLocaleLowerCase("es") === rom.base.toLocaleLowerCase("es") ? "" : rom.base;
  return `
    <button class="game-card ${selected ? "is-selected" : ""}" data-action="select-rom" data-path="${encodeURIComponent(rom.path)}" aria-label="${escapeHtml(rom.title)}">
      <span class="cover-frame">
        ${cover ? `<img src="${escapeHtml(cover)}" loading="lazy" alt="" />` : `<span class="cover-placeholder">NG</span>`}
        ${state.favorites.has(rom.path) ? `<span class="favorite-mark">♥</span>` : ""}
      </span>
      <strong>${escapeHtml(rom.title)}</strong>
      <small>${setName ? `${escapeHtml(setName)} · ` : ""}${escapeHtml(rom.developer || rom.publisher || "Neo Geo")} · ${rom.format.toUpperCase()}</small>
    </button>
  `;
}

function renderAchievements(): string {
  if (state.raLoading) return `<main class="content-page"><div class="page-loading">Cargando actividad RetroAchievements…</div></main>`;
  const ra = state.ra;
  if (!ra?.available || !ra.profile) {
    return `
      <main class="content-page ra-onboarding">
        <section>
          <div class="ra-emblem">${navIcon("achievement")}</div>
          <h1>RetroAchievements, dentro de tu launcher</h1>
          <p>${escapeHtml(ra?.message || "Conecta tu cuenta para mostrar tu progreso.")}</p>
          ${ra?.basic ? `<div class="basic-score"><strong>${formatNumber(ra.basic.score)}</strong><span>puntos de ${escapeHtml(ra.basic.username)}</span></div>` : ""}
          <button class="primary-button" data-action="view" data-view="settings">Abrir ajustes de cuenta</button>
          ${ra?.apiKeyUrl ? `<a href="${escapeHtml(ra.apiKeyUrl)}" target="_blank" rel="noreferrer">Obtener Web API Key ↗</a>` : ""}
        </section>
      </main>
    `;
  }

  const profile = ra.profile;
  const games = ra.recentlyPlayed ?? [];
  const achievements = ra.recentAchievements ?? [];
  return `
    <main class="content-page achievements-page">
      <section class="profile-hero">
        <img src="${escapeHtml(profile.avatar)}" alt="" />
        <div class="profile-copy">
          <span class="profile-status"><i class="${profile.status.toLowerCase() === "online" ? "is-online" : ""}"></i>${escapeHtml(profile.status)}</span>
          <h1>${escapeHtml(profile.username)}</h1>
          <p>${escapeHtml(profile.richPresence || profile.motto || "Preparado para el siguiente reto.")}</p>
          <div class="profile-actions">
            ${ra.profileUrl ? `<a class="primary-button" href="${escapeHtml(ra.profileUrl)}" target="_blank" rel="noreferrer">Ver perfil oficial ↗</a>` : ""}
            <button class="secondary-button" data-action="refresh-ra">${navIcon("refresh")} Actualizar</button>
          </div>
        </div>
        <div class="profile-stats">
          <div><strong>${formatNumber(profile.points)}</strong><span>Puntos</span></div>
          <div><strong>#${formatNumber(profile.rank)}</strong><span>Rango global</span></div>
          <div><strong>${formatNumber(profile.truePoints)}</strong><span>True points</span></div>
        </div>
      </section>

      <section class="ra-grid">
        <div class="recent-games">
          <div class="section-heading"><div><h2>Últimos juegos</h2><p>Progreso sincronizado con RetroAchievements</p></div><span>${games.length}</span></div>
          <div class="recent-game-list">
            ${games.length ? games.map((game) => `
              <article class="recent-game">
                <img src="${escapeHtml(game.image || game.icon)}" alt="" />
                <div>
                  <h3>${escapeHtml(game.title)}</h3>
                  <p>${escapeHtml(game.consoleName)} · ${formatDate(game.lastPlayed)}</p>
                  <div class="progress-line"><i style="width:${game.progress}%"></i></div>
                  <small>${game.achieved}/${game.achievementsTotal} logros · ${game.progress}%</small>
                </div>
              </article>
            `).join("") : `<p class="quiet">No hay partidas recientes.</p>`}
          </div>
        </div>

        <div class="unlock-feed">
          <div class="section-heading"><div><h2>Últimos logros</h2><p>Actividad de los últimos 30 días</p></div><span>${achievements.length}</span></div>
          <div class="achievement-list">
            ${achievements.length ? achievements.map((achievement) => `
              <article class="achievement-item">
                <img src="${escapeHtml(achievement.badge)}" alt="" />
                <div>
                  <div class="achievement-title"><h3>${escapeHtml(achievement.title)}</h3><strong>+${achievement.points}</strong></div>
                  <p>${escapeHtml(achievement.description)}</p>
                  <small>${escapeHtml(achievement.gameTitle)} · ${formatDate(achievement.date)}${achievement.hardcore ? " · HARDCORE" : ""}</small>
                </div>
              </article>
            `).join("") : `<p class="quiet">Todavía no hay logros recientes.</p>`}
          </div>
        </div>
      </section>
    </main>
  `;
}

function toggle(key: string): string {
  const checked = Boolean(state.draft[key]);
  return `
    <button class="setting-toggle ${checked ? "is-on" : ""}" data-action="toggle" data-key="${key}">
      <span><strong>${labels[key] ?? key}</strong><small>${checked ? "Activado" : "Desactivado"}</small></span>
      <i></i>
    </button>
  `;
}

function field(key: string, label: string, placeholder = ""): string {
  return `
    <label class="setting-field"><span>${label}</span>
      <input data-action="config-input" data-key="${key}" value="${escapeHtml(configString(key))}" placeholder="${escapeHtml(placeholder)}" />
    </label>
  `;
}

function secretField(key: string, label: string, configured: boolean, help = ""): string {
  return `
    <label class="setting-field"><span>${label}${configured ? `<em>Configurado</em>` : ""}</span>
      <input type="password" autocomplete="off" data-action="secret-input" data-key="${key}" value="${escapeHtml(state.secretDraft[key] ?? "")}" placeholder="${configured ? "Escribe para sustituirlo" : "No configurado"}" />
      ${help ? `<small>${help}</small>` : ""}
    </label>
  `;
}

function range(key: string, label: string, min: number, max: number, step: number): string {
  const value = getNumber(key, key === "volume" ? 100 : 3);
  return `
    <label class="setting-range"><span><strong>${label}</strong><em>${value}${key === "volume" ? "%" : "×"}</em></span>
      <input type="range" min="${min}" max="${max}" step="${step}" value="${value}" data-action="range" data-key="${key}" />
    </label>
  `;
}

function selectField(
  key: string,
  label: string,
  options: Array<{ value: string; text: string }>,
  fallback = "",
): string {
  const current = configString(key, fallback);
  return `
    <label class="setting-field"><span>${label}</span><select data-action="config-input" data-key="${key}">
      ${options.map((option) => `<option value="${escapeHtml(option.value)}" ${option.value === current ? "selected" : ""}>${escapeHtml(option.text)}</option>`).join("")}
    </select></label>
  `;
}

function renderSettings(): string {
  const response = state.response;
  if (!response) return "";
  const library = state.library;
  const currentBios = configString("bios");
  const biosOptions = biosOptionsForSettings(currentBios, library?.biosOptions ?? []);
  const biosProfiles = biosProfilesForSettings(currentBios, biosOptions);
  const currentProfile = biosProfileFromLabel(currentBios);
  const selectedProfile =
    currentProfile && biosProfiles.includes(currentProfile) ? currentProfile : biosProfiles[0] ?? "neogeo";
  const profileRegions = biosRegionsForProfile(selectedProfile, biosOptions);
  const configuredRegion = normalizeBiosRegion(configString("bios_region", "europe"));
  const currentRegion = biosRegionFromLabel(currentBios);
  const selectedRegion =
    selectedProfile === "unibios"
      ? configuredRegion
      : profileRegions.has(configuredRegion)
        ? configuredRegion
        : currentRegion && profileRegions.has(currentRegion)
          ? currentRegion
          : [...profileRegions][0] ?? configuredRegion;
  const selectedBios = resolveBiosOption(selectedProfile, selectedRegion, biosOptions);
  const regionDisabled = selectedProfile === "unibios" || !biosOptions.length || !profileRegions.size;
  return `
    <main class="content-page settings-page">
      <div class="settings-heading">
        <div><h1>Ajustes</h1><p>Configuración del emulador y servicios conectados.</p></div>
        <div>
          <button class="secondary-button" data-action="rescan">${navIcon("refresh")} Escanear biblioteca</button>
          <button class="primary-button" data-action="save" ${state.saving ? "disabled" : ""}>${state.saving ? "Guardando…" : state.dirty ? "Guardar cambios" : "Todo guardado"}</button>
        </div>
      </div>
      <div class="settings-grid">
        <section class="settings-panel">
          <div class="settings-panel-title"><span>01</span><div><h2>Vídeo y audio</h2><p>Presentación, CRT y salida de sonido.</p></div></div>
          <div class="toggle-grid">${toggle("scanlines")}${toggle("curvature")}${toggle("bloom")}${toggle("fullscreen")}${toggle("muted")}</div>
          ${selectField("aspect_ratio", "Relación de aspecto", [
            { value: "4:3", text: "4:3 corregido" },
            { value: "1:1", text: "1:1 PAR (píxel cuadrado)" },
            { value: "45:44", text: "45:44 PAR (hardware)" },
            { value: "16:9", text: "16:9 panorámico" },
          ], "4:3")}
          ${selectField("palette_mode", "Conversión de paleta", [
            { value: "resnet", text: "Red de resistencias (precisa)" },
            { value: "raw", text: "Raw 6-bit" },
          ], "resnet")}
          ${range("volume", "Volumen", 0, 100, 5)}
          ${range("window_scale", "Escala de ventana", 2, 4, 1)}
        </section>

        <section class="settings-panel">
          <div class="settings-panel-title"><span>02</span><div><h2>Sistema</h2><p>BIOS, idioma, guardado y controles.</p></div></div>
          <div class="bios-pair-grid">
            <label class="setting-field bios-select-field"><span>BIOS activa</span><select data-action="bios-profile" ${biosProfiles.length ? "" : "disabled"}>
              ${biosProfiles.map((profile) => `<option value="${profile}" ${profile === selectedProfile ? "selected" : ""}>${escapeHtml(`${biosProfileLabel(profile)} · ${biosProfileTitle(profile)}`)}</option>`).join("")}
            </select><small>${biosProfiles.length ? "AES es consola; NeoGeo/MVS es arcade; UniBIOS se bloquea en Hardcore." : "No se detectaron BIOS en bios/ ni en roms/."}</small></label>
            ${selectedProfile === "unibios"
              ? `<label class="setting-field bios-select-field"><span>Hardware UniBIOS</span><select data-action="config-input" data-key="unibios_hardware">
                  <option value="mvs" ${configString("unibios_hardware", "mvs") === "mvs" ? "selected" : ""}>MVS arcade</option>
                  <option value="aes" ${configString("unibios_hardware", "mvs") === "aes" ? "selected" : ""}>AES consola</option>
                </select><small>La región se cambia desde el menú UniBIOS; este selector fija el hardware anunciado al juego.</small></label>`
              : `<label class="setting-field bios-select-field"><span>Región</span><select data-action="bios-region" data-profile="${selectedProfile}" ${regionDisabled ? "disabled" : ""}>
                  ${biosRegionOrder.map((region) => {
                    const available = profileRegions.has(region);
                    const disabled = !available && profileRegions.size ? "disabled" : "";
                    return `<option value="${region}" ${region === selectedRegion ? "selected" : ""} ${disabled}>${escapeHtml(biosRegionLabel(region))}</option>`;
                  }).join("")}
                </select><small>${selectedBios ? "Se usará la BIOS regional compatible detectada." : "No hay archivo compatible para este perfil."}</small></label>`}
          </div>
          ${renderBiosGuide(selectedProfile, selectedRegion, selectedBios)}
          ${field("lang", "Idioma", "es / en")}
          <div class="toggle-grid">${toggle("auto_save")}${toggle("diagnostic_dumps")}${toggle("gamepad")}</div>
          <div class="field-grid">
            ${selectField("sprite_limit", "Sprites por línea", [
              { value: "96", text: "96 (hardware)" },
              { value: "192", text: "192 (hack)" },
              { value: "288", text: "288 (hack)" },
              { value: "381", text: "381 (hack)" },
            ], "96")}
          </div>
          <div class="toggle-grid">${toggle("setting_mode")}${toggle("freeplay")}${toggle("memcard")}${toggle("memcard_write_protect")}${toggle("cpu_overclock")}${toggle("disable_adpcm_wrap")}</div>
        </section>

        <section class="settings-panel settings-wide">
          <div class="settings-panel-title"><span>03</span><div><h2>Biblioteca y media</h2><p>Rutas resueltas y gamelist utilizado.</p></div></div>
          <div class="field-grid">${field("rom_path", "ROMs", "roms")}${field("bios_dir", "BIOS", "bios")}${field("media_dir", "Media de respaldo", "media")}</div>
          <div class="path-summary">
            <span><strong>gamelist.xml</strong>${escapeHtml(library?.roots.gamelistPath || "No detectado")}</span>
            <span><strong>Media indexada</strong>${formatNumber(library?.mediaCount ?? 0)} recursos</span>
            <span><strong>Juegos</strong>${formatNumber(library?.roms.length ?? 0)} detectados</span>
          </div>
        </section>

        <section class="settings-panel settings-wide ra-settings">
          <div class="settings-panel-title"><span>04</span><div><h2>RetroAchievements</h2><p>Login del emulador y datos ampliados del perfil.</p></div></div>
          <div class="field-grid">
            ${field("ra_username", "Usuario RA", "Tu usuario")}
            ${secretField("ra_password", "Contraseña RA", response.secrets.raPasswordConfigured)}
            ${secretField("ra_token", "Token rcheevos", response.secrets.raTokenConfigured, "Lo usa el emulador para iniciar sesión y desbloquear logros.")}
            ${secretField("ra_api_key", "Web API Key", response.secrets.raApiKeyConfigured, "Permite cargar perfil, rango, juegos recientes y últimos logros.")}
          </div>
          <div class="ra-settings-footer">
            <div>${toggle("ra_hardcore")}</div>
            <div class="ra-connect-actions">
              <button class="secondary-button danger" data-action="ra-disconnect" ${state.raBusy ? "disabled" : ""}>Desconectar</button>
              <button class="primary-button" data-action="ra-connect" ${state.raBusy ? "disabled" : ""}>${state.raBusy ? "Conectando…" : "Conectar y comprobar"}</button>
            </div>
          </div>
          ${state.raStatus ? `<p class="status-message">${escapeHtml(state.raStatus)}</p>` : ""}
          <p class="security-note">Las credenciales se guardan solo en <code>config/ngneon.conf</code> y nunca se devuelven al navegador.</p>
        </section>
      </div>
    </main>
  `;
}

function render() {
  if (!state.loaded) {
    app.innerHTML = `
      <main class="boot">
        <div class="boot-logo">N</div>
        <h1>NGNEON</h1>
        <p>${state.error ? escapeHtml(state.error) : "Preparando tu colección…"}</p>
        ${state.error ? `<button data-action="reload">Reintentar</button>` : `<div class="loading-line"><i></i></div>`}
      </main>
    `;
    return;
  }

  const content = state.view === "library" ? renderLibrary() : state.view === "achievements" ? renderAchievements() : renderSettings();
  app.innerHTML = `
    <div class="app-shell">
      ${renderHeader()}
      ${state.error ? `<div class="error-banner">${escapeHtml(state.error)}<button data-action="dismiss-error">×</button></div>` : ""}
      ${content}
      ${state.toast ? `<div class="toast">${escapeHtml(state.toast)}</div>` : ""}
    </div>
  `;
  scrollSelectedCardIntoView();
  if (state.toast) {
    window.setTimeout(() => {
      state.toast = "";
      document.querySelector(".toast")?.remove();
    }, 2600);
  }
}

function scrollSelectedCardIntoView() {
  requestAnimationFrame(() => {
    document.querySelector<HTMLElement>(".game-card.is-selected")?.scrollIntoView({
      behavior: "smooth",
      block: "nearest",
      inline: "nearest",
    });
  });
}

function selectRom(pathValue: string) {
  state.selectedRomPath = pathValue;
  const rom = selectedRom();
  state.mediaMode = rom?.media.video ? "video" : rom?.media.fanart ? "fanart" : "image";
  render();
}

app.addEventListener("click", (event) => {
  const target = event.target;
  if (!(target instanceof HTMLElement)) return;
  const actionEl = target.closest<HTMLElement>("[data-action]");
  if (!actionEl) return;
  const action = actionEl.dataset.action;
  const key = actionEl.dataset.key;

  if (action === "view") {
    state.view = (actionEl.dataset.view as ViewName) || "library";
    render();
  } else if (action === "toggle" && key) {
    state.draft[key] = !state.draft[key];
    state.dirty = true;
    render();
  } else if (action === "save") {
    void saveConfig();
  } else if (action === "reload") {
    void loadConfig();
  } else if (action === "rescan") {
    void loadLibrary();
  } else if (action === "refresh-ra") {
    void loadRa();
  } else if (action === "launch") {
    void launchEmulator();
  } else if (action === "launch-mode") {
    const mode = actionEl.dataset.mode;
    if (mode === "normal" || mode === "clean" || mode === "diagnostic") {
      state.launchMode = mode;
      render();
    }
  } else if (action === "stop-session") {
    void stopEmulator();
  } else if (action === "ra-connect") {
    void connectRetroAchievements();
  } else if (action === "ra-disconnect") {
    void disconnectRetroAchievements();
  } else if (action === "select-rom") {
    selectRom(decodeURIComponent(actionEl.dataset.path ?? ""));
  } else if (action === "favorite") {
    const pathValue = state.selectedRomPath;
    if (state.favorites.has(pathValue)) state.favorites.delete(pathValue);
    else state.favorites.add(pathValue);
    persistLocalLists();
    render();
  } else if (action === "filter") {
    state.filter = (actionEl.dataset.filter as typeof state.filter) || "all";
    render();
  } else if (action === "media") {
    state.mediaMode = (actionEl.dataset.media as MediaKey) || "image";
    render();
  } else if (action === "open-media") {
    const rom = selectedRom();
    const media = actionEl.dataset.media as MediaKey;
    const url = rom?.media[media];
    if (url) window.open(url, "_blank", "noopener,noreferrer");
  } else if (action === "dismiss-error") {
    state.error = null;
    render();
  }
});

app.addEventListener("input", (event) => {
  const target = event.target;
  if (!(target instanceof HTMLInputElement) && !(target instanceof HTMLSelectElement)) return;
  const action = target.dataset.action;
  const key = target.dataset.key;
  if (action === "search") {
    state.search = target.value;
    render();
  } else if (action === "range" && key) {
    state.draft[key] = Number(target.value);
    state.dirty = true;
    render();
  } else if (action === "bios-profile" && isBiosProfile(target.value)) {
    setBiosSelection(target.value, normalizeBiosRegion(configString("bios_region", "europe")));
  } else if (action === "bios-region" && isBiosRegion(target.value)) {
    const datasetProfile = target.dataset.profile ?? "";
    const profile = isBiosProfile(datasetProfile) ? datasetProfile : biosProfileFromLabel(configString("bios"));
    setBiosSelection(profile || "neogeo", target.value);
  } else if (action === "config-input" && key) {
    state.draft[key] = target.value;
    state.dirty = true;
    if (key === "bios") {
      render();
    } else {
      document.querySelector<HTMLButtonElement>('[data-action="save"]')?.replaceChildren("Guardar cambios");
    }
  } else if (action === "secret-input" && key) {
    state.secretDraft[key] = target.value;
    state.dirty = true;
    document.querySelector<HTMLButtonElement>('[data-action="save"]')?.replaceChildren("Guardar cambios");
  }
});

document.addEventListener("keydown", (event) => {
  if (state.view !== "library") return;
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;
  const games = filteredRoms();
  if (!games.length) return;
  const index = Math.max(0, games.findIndex((rom) => rom.path === state.selectedRomPath));
  if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
    event.preventDefault();
    const delta = event.key === "ArrowRight" ? 1 : -1;
    const next = (index + delta + games.length) % games.length;
    selectRom(games[next].path);
  } else if (event.key === "Enter") {
    event.preventDefault();
    void launchEmulator();
  } else if (event.key.toLowerCase() === "f") {
    event.preventDefault();
    const pathValue = state.selectedRomPath;
    if (state.favorites.has(pathValue)) state.favorites.delete(pathValue);
    else state.favorites.add(pathValue);
    persistLocalLists();
    render();
  }
});

void loadConfig();
window.setInterval(() => {
  if (state.loaded) void loadSession(false);
}, 1500);
