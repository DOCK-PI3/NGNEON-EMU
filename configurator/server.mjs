import { createServer } from "node:http";
import { readFile, writeFile, stat, readdir } from "node:fs/promises";
import { createReadStream, existsSync } from "node:fs";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = process.env.NGNEON_ROOT
  ? path.resolve(process.env.NGNEON_ROOT)
  : path.resolve(__dirname, "..");
const configFile = path.join(projectRoot, "config", "ngneon.conf");
const emulatorExe = path.join(projectRoot, "ngneon-emu.exe");
const distDir = path.join(__dirname, "dist");
const devIndex = path.join(__dirname, "index.html");
const port = Number(process.env.NGNEON_CONFIG_PORT || 4177);

const publicKeys = new Set([
  "rom_path",
  "bios_dir",
  "media_dir",
  "lang",
  "scanlines",
  "curvature",
  "bloom",
  "fullscreen",
  "aspect_ratio",
  "volume",
  "window_scale",
  "auto_save",
  "muted",
  "bios",
  "ra_username",
  "ra_hardcore",
  "diagnostic_dumps",
  "gamepad",
]);

const secretKeys = new Set(["ra_token", "ra_password", "ra_api_key"]);
const writableKeys = new Set([...publicKeys, ...secretKeys]);
const romExtensions = new Set([".neo", ".zip"]);
const imageExtensions = new Set([".png", ".jpg", ".jpeg", ".webp", ".gif", ".avif"]);
const videoExtensions = new Set([".mp4", ".webm", ".mov", ".m4v"]);
const mediaExtensions = new Set([...imageExtensions, ...videoExtensions, ".pdf"]);
const systemZipStems = new Set(["neogeo", "aes", "uni-bios", "unibios", "mvstemp"]);
const assetRegistry = new Map();
const raCache = new Map();
const RA_CACHE_MS = 60_000;

function isSystemZipName(name) {
  const ext = path.extname(name).toLowerCase();
  if (ext !== ".zip") return false;
  const stem = path.basename(name, ext).toLowerCase();
  return systemZipStems.has(stem) || stem.startsWith("uni-bios-") || stem.startsWith("unibios-");
}

function isPlayableRomName(name) {
  const ext = path.extname(name).toLowerCase();
  return romExtensions.has(ext) && !(ext === ".zip" && isSystemZipName(name));
}

function parseConfig(text) {
  const lines = text.split(/\r?\n/);
  const values = {};
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const index = trimmed.indexOf("=");
    if (index < 0) continue;
    values[trimmed.slice(0, index)] = trimmed.slice(index + 1);
  }
  return { lines, values };
}

async function readConfig() {
  const text = await readFile(configFile, "utf8");
  return parseConfig(text);
}

function publicConfig(values) {
  const out = {};
  for (const key of publicKeys) {
    if (Object.prototype.hasOwnProperty.call(values, key)) {
      out[key] = values[key];
    }
  }
  out.aspect_ratio ||= "4:3";
  return out;
}

async function sendJson(res, data, statusCode = 200) {
  const body = JSON.stringify(data, null, 2);
  res.writeHead(statusCode, {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "no-store",
  });
  res.end(body);
}

async function readRequestJson(req) {
  const chunks = [];
  for await (const chunk of req) {
    chunks.push(chunk);
  }
  const body = Buffer.concat(chunks).toString("utf8");
  return body ? JSON.parse(body) : {};
}

async function persistConfigValues(updates, writable = writableKeys) {
  const current = await readConfig();
  const nextValues = { ...current.values };
  for (const [key, value] of Object.entries(updates)) {
    if (!writable.has(key)) continue;
    nextValues[key] = String(value);
  }

  const seen = new Set();
  const nextLines = current.lines.map((line) => {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#") || !trimmed.includes("=")) {
      return line;
    }
    const key = trimmed.slice(0, trimmed.indexOf("="));
    if (!writable.has(key)) {
      return line;
    }
    seen.add(key);
    return `${key}=${nextValues[key] ?? ""}`;
  });

  for (const key of writable) {
    if (!seen.has(key) && Object.prototype.hasOwnProperty.call(nextValues, key)) {
      nextLines.push(`${key}=${nextValues[key]}`);
    }
  }

  await writeFile(configFile, `${nextLines.join("\n").replace(/\n+$/u, "")}\n`, "utf8");
}

async function handleGetConfig(_req, res) {
  const { values } = await readConfig();
  await sendJson(res, {
    config: publicConfig(values),
    secrets: {
      raTokenConfigured: Boolean(values.ra_token),
      raPasswordConfigured: Boolean(values.ra_password),
      raApiKeyConfigured: Boolean(values.ra_api_key),
    },
    paths: {
      projectRoot,
      configFile,
      emulatorExe,
    },
  });
}

async function handleSaveConfig(req, res) {
  const payload = await readRequestJson(req);
  if (!payload || typeof payload.config !== "object") {
    await sendJson(res, { error: "Payload config invalido" }, 400);
    return;
  }

  const updates = {};
  for (const [key, value] of Object.entries(payload.config)) {
    if (!publicKeys.has(key) || secretKeys.has(key)) continue;
    updates[key] = String(value);
  }

  if (payload.secrets && typeof payload.secrets === "object") {
    for (const [key, value] of Object.entries(payload.secrets)) {
      if (!secretKeys.has(key)) continue;
      const text = String(value);
      if (text.length > 0) {
        updates[key] = text;
      }
    }
  }

  await persistConfigValues(updates, writableKeys);
  raCache.clear();
  await sendJson(res, { ok: true });
}

async function raLogin(username, credential) {
  const body = new URLSearchParams();
  body.set("r", "login2");
  body.set("u", username);
  if (credential.password) {
    body.set("p", credential.password);
  } else if (credential.token) {
    body.set("t", credential.token);
  } else {
    throw new Error("Faltan credenciales RA");
  }

  const response = await fetch("https://retroachievements.org/dorequest.php", {
    method: "POST",
    headers: {
      "content-type": "application/x-www-form-urlencoded",
      "user-agent": "NGNEON-EMU Configurator/0.2",
    },
    body,
  });
  const text = await response.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    throw new Error(`Respuesta RA invalida: HTTP ${response.status}`);
  }

  if (!response.ok || data.Success === false || data.Success === "false") {
    throw new Error(data.Error || `RA login error HTTP ${response.status}`);
  }
  if (!data.User || !data.Token) {
    throw new Error("RA login OK, pero la respuesta no trae usuario/token");
  }
  return data;
}

async function handleRaConnect(req, res) {
  const payload = await readRequestJson(req);
  const { values } = await readConfig();
  const username = String(payload.username || values.ra_username || "").trim();
  const typedPassword = String(payload.password || "");
  const typedToken = String(payload.token || "");
  const typedApiKey = String(payload.apiKey || "");
  const savedPassword = String(values.ra_password || "");
  const savedToken = String(values.ra_token || "");

  if (!username) {
    await sendJson(res, { error: "Falta usuario RA" }, 400);
    return;
  }

  const attempts = [];
  if (typedPassword) attempts.push({ method: "password", password: typedPassword });
  if (typedToken) attempts.push({ method: "token", token: typedToken });
  if (!typedPassword && !typedToken && savedToken) attempts.push({ method: "token", token: savedToken });
  if (!typedPassword && !typedToken && savedPassword) attempts.push({ method: "password", password: savedPassword });

  if (attempts.length === 0) {
    await sendJson(res, { error: "Falta token o contrasena RA" }, 400);
    return;
  }

  const errors = [];
  for (const attempt of attempts) {
    try {
      const login = await raLogin(username, attempt);
      const updates = {
        ra_username: String(login.User),
        ra_token: String(login.Token),
      };
      if (attempt.password) {
        updates.ra_password = attempt.password;
      } else if (typedPassword) {
        updates.ra_password = typedPassword;
      }
      if (typedApiKey) {
        updates.ra_api_key = typedApiKey;
      }
      await persistConfigValues(updates, writableKeys);
      raCache.clear();
      await sendJson(res, {
        ok: true,
        username: String(login.User),
        score: Number(login.Score || 0),
        softcoreScore: Number(login.SoftcoreScore || 0),
        method: attempt.method,
      });
      return;
    } catch (error) {
      errors.push(error instanceof Error ? error.message : String(error));
    }
  }

  await sendJson(res, { error: errors.join(" / ") || "RA login error" }, 401);
}

async function handleRaDisconnect(_req, res) {
  await persistConfigValues({ ra_token: "", ra_password: "", ra_api_key: "" }, writableKeys);
  raCache.clear();
  await sendJson(res, { ok: true });
}

function raMediaUrl(value) {
  if (!value) return "";
  if (/^https?:\/\//iu.test(value)) return value;
  return `https://media.retroachievements.org${value.startsWith("/") ? value : `/${value}`}`;
}

async function fetchRaApi(endpoint, apiKey, params) {
  const url = new URL(`https://retroachievements.org/API/${endpoint}`);
  url.searchParams.set("y", apiKey);
  for (const [key, value] of Object.entries(params)) {
    url.searchParams.set(key, String(value));
  }
  const response = await fetch(url, {
    headers: { "user-agent": "NGNEON-EMU Configurator/0.2" },
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`RetroAchievements Web API: HTTP ${response.status}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error("RetroAchievements Web API devolvio una respuesta no valida");
  }
}

function normalizeRaDashboard(summary, recentlyPlayed, recentAchievements) {
  const profile = {
    username: String(summary.User || ""),
    ulid: String(summary.ULID || ""),
    avatar: raMediaUrl(summary.UserPic),
    memberSince: String(summary.MemberSince || ""),
    motto: String(summary.Motto || ""),
    richPresence: String(summary.RichPresenceMsg || ""),
    status: String(summary.Status || "Offline"),
    points: Number(summary.TotalPoints || 0),
    softcorePoints: Number(summary.TotalSoftcorePoints || 0),
    truePoints: Number(summary.TotalTruePoints || 0),
    rank: Number(summary.Rank || 0),
    totalRanked: Number(summary.TotalRanked || 0),
  };

  const games = (Array.isArray(recentlyPlayed) ? recentlyPlayed : []).map((game) => {
    const possible = Number(game.NumPossibleAchievements || game.AchievementsTotal || 0);
    const achieved = Number(game.NumAchieved || 0);
    const hardcore = Number(game.NumAchievedHardcore || 0);
    return {
      id: Number(game.GameID || 0),
      title: String(game.Title || ""),
      consoleName: String(game.ConsoleName || ""),
      image: raMediaUrl(game.ImageBoxArt || game.ImageIcon),
      icon: raMediaUrl(game.ImageIcon),
      lastPlayed: String(game.LastPlayed || ""),
      achievementsTotal: possible,
      achieved,
      hardcore,
      possibleScore: Number(game.PossibleScore || 0),
      scoreAchieved: Number(game.ScoreAchieved || 0),
      progress: possible > 0 ? Math.round((achieved / possible) * 100) : 0,
    };
  });

  const achievements = (Array.isArray(recentAchievements) ? recentAchievements : []).map((achievement) => ({
    id: Number(achievement.AchievementID || 0),
    title: String(achievement.Title || ""),
    description: String(achievement.Description || ""),
    gameTitle: String(achievement.GameTitle || ""),
    gameId: Number(achievement.GameID || 0),
    consoleName: String(achievement.ConsoleName || ""),
    points: Number(achievement.Points || 0),
    trueRatio: Number(achievement.TrueRatio || 0),
    hardcore: Boolean(Number(achievement.HardcoreMode || 0)),
    date: String(achievement.Date || ""),
    badge: raMediaUrl(achievement.BadgeURL || (achievement.BadgeName ? `/Badge/${achievement.BadgeName}.png` : "")),
  }));

  return {
    available: true,
    needsApiKey: false,
    profile,
    recentlyPlayed: games,
    recentAchievements: achievements,
    profileUrl: `https://retroachievements.org/user/${encodeURIComponent(profile.username)}`,
    refreshedAt: new Date().toISOString(),
  };
}

async function handleRaDashboard(_req, res) {
  const { values } = await readConfig();
  const username = String(values.ra_username || "").trim();
  const apiKey = String(values.ra_api_key || "").trim();
  const token = String(values.ra_token || "").trim();

  if (!username) {
    await sendJson(res, { available: false, connected: false, message: "Configura tu usuario RA." });
    return;
  }

  if (!apiKey) {
    let basic = { username, score: 0, softcoreScore: 0 };
    if (token) {
      try {
        const login = await raLogin(username, { token });
        basic = {
          username: String(login.User || username),
          score: Number(login.Score || 0),
          softcoreScore: Number(login.SoftcoreScore || 0),
        };
      } catch {
        // Keep the configurator usable if the session token is stale.
      }
    }
    await sendJson(res, {
      available: false,
      connected: Boolean(token),
      needsApiKey: true,
      basic,
      message: "Anade la Web API Key de RetroAchievements para cargar perfil, progreso y logros recientes.",
      apiKeyUrl: "https://retroachievements.org/controlpanel.php",
    });
    return;
  }

  const cacheKey = `${username}:${createHash("sha1").update(apiKey).digest("hex")}`;
  const cached = raCache.get(cacheKey);
  if (cached && Date.now() - cached.timestamp < RA_CACHE_MS) {
    await sendJson(res, cached.data);
    return;
  }

  const [summary, recentlyPlayed, recentAchievements] = await Promise.all([
    fetchRaApi("API_GetUserSummary.php", apiKey, { u: username, g: 10, a: 10 }),
    fetchRaApi("API_GetUserRecentlyPlayedGames.php", apiKey, { u: username, c: 12, o: 0 }),
    fetchRaApi("API_GetUserRecentAchievements.php", apiKey, { u: username, m: 43_200 }),
  ]);
  const data = normalizeRaDashboard(summary, recentlyPlayed, recentAchievements.slice(0, 16));
  raCache.set(cacheKey, { timestamp: Date.now(), data });
  await sendJson(res, data);
}

function resolveConfiguredPath(configValue, fallback) {
  const raw = String(configValue || fallback || "");
  const candidate = raw && path.isAbsolute(raw) ? raw : path.join(projectRoot, raw || fallback);
  if (existsSync(candidate)) {
    return candidate;
  }
  const localFallback = path.join(projectRoot, fallback);
  if (existsSync(localFallback)) {
    return localFallback;
  }
  const releaseFallback = path.join(projectRoot, "target", "release", fallback);
  if (existsSync(releaseFallback)) {
    return releaseFallback;
  }
  return candidate;
}

async function listFilesSafe(dir, predicate) {
  try {
    const entries = await readdir(dir, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isFile())
      .map((entry) => entry.name)
      .filter(predicate)
      .sort((a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }));
  } catch {
    return [];
  }
}

function decodeXml(value) {
  let text = String(value || "");
  for (let pass = 0; pass < 2; pass += 1) {
    text = text
      .replaceAll("&lt;", "<")
      .replaceAll("&gt;", ">")
      .replaceAll("&quot;", '"')
      .replaceAll("&apos;", "'")
      .replaceAll("&#39;", "'")
      .replaceAll("&amp;", "&");
  }
  return text.trim();
}

function readXmlTag(block, tag) {
  const match = block.match(new RegExp(`<${tag}>([\\s\\S]*?)</${tag}>`, "iu"));
  return match ? decodeXml(match[1]) : "";
}

function registerAsset(file) {
  if (!file || !existsSync(file)) return "";
  const resolved = path.resolve(file);
  const id = createHash("sha1").update(resolved.toLowerCase()).digest("hex").slice(0, 20);
  assetRegistry.set(id, resolved);
  return `/api/media/${id}`;
}

function resolveGamelistAsset(gamelistDir, value) {
  if (!value || /^https?:\/\//iu.test(value)) return "";
  const resolved = path.resolve(gamelistDir, value.replaceAll("/", path.sep));
  return existsSync(resolved) ? resolved : "";
}

function parseGamelist(xml, gamelistDir, romByName) {
  const games = [];
  const tags = [
    "path",
    "name",
    "desc",
    "image",
    "thumbnail",
    "marquee",
    "video",
    "manual",
    "fanart",
    "titleshot",
    "boxart",
    "cartridge",
    "screenshot",
    "mix",
    "wheel",
    "box2d",
    "box3d",
    "cartridge2d",
    "developer",
    "publisher",
    "genre",
    "players",
    "rating",
    "releasedate",
    "playcount",
    "lastplayed",
  ];
  for (const match of xml.matchAll(/<game>([\s\S]*?)<\/game>/giu)) {
    const values = Object.fromEntries(tags.map((tag) => [tag, readXmlTag(match[1], tag)]));
    const xmlRomPath = path.resolve(gamelistDir, values.path.replaceAll("/", path.sep));
    const romName = path.basename(xmlRomPath);
    const localRom = romByName.get(romName.toLowerCase());
    if (!localRom) continue;
    const ext = path.extname(localRom).toLowerCase();
    const media = {};
    for (const tag of [
      "video",
      "image",
      "fanart",
      "screenshot",
      "titleshot",
      "marquee",
      "wheel",
      "thumbnail",
      "boxart",
      "box2d",
      "box3d",
      "cartridge",
      "cartridge2d",
      "mix",
      "manual",
    ]) {
      media[tag] = registerAsset(resolveGamelistAsset(gamelistDir, values[tag]));
    }
    const base = path.basename(localRom, ext);
    games.push({
      name: romName,
      base,
      title: values.name || base,
      format: ext.slice(1),
      path: localRom,
      description: values.desc,
      developer: values.developer,
      publisher: values.publisher,
      genre: values.genre,
      players: values.players,
      rating: Number(values.rating || 0),
      releaseDate: values.releasedate,
      playCount: Number(values.playcount || 0),
      lastPlayed: values.lastplayed,
      hasMedia: Object.values(media).some(Boolean),
      media,
    });
  }
  return games;
}

async function findGamelist(romDir) {
  const candidates = [
    path.join(romDir, "gamelist.xml"),
    path.join(projectRoot, "roms", "gamelist.xml"),
    path.join(projectRoot, "gamelist.xml"),
  ];
  return candidates.find((candidate) => existsSync(candidate)) || "";
}

async function handleLibrary(_req, res) {
  const { values } = await readConfig();
  const romDir = resolveConfiguredPath(values.rom_path, "roms");
  const biosDir = resolveConfiguredPath(values.bios_dir, "bios");
  const mediaDir = resolveConfiguredPath(values.media_dir, "media");
  const gamelistPath = await findGamelist(romDir);

  const romNames = await listFilesSafe(romDir, isPlayableRomName);
  const romByName = new Map(romNames.map((name) => [name.toLowerCase(), path.join(romDir, name)]));
  const biosArchives = await listFilesSafe(
    biosDir,
    (name) => name.toLowerCase() === "neogeo.zip" || name.toLowerCase() === "aes.zip",
  );

  assetRegistry.clear();
  let games = [];
  if (gamelistPath) {
    const xml = await readFile(gamelistPath, "utf8");
    games = parseGamelist(xml, path.dirname(gamelistPath), romByName);
  }

  const matched = new Set(games.map((game) => game.name.toLowerCase()));
  for (const name of romNames) {
    if (matched.has(name.toLowerCase())) continue;
    const ext = path.extname(name).toLowerCase();
    const base = path.basename(name, ext);
    const fallbackCandidates = [...imageExtensions].map((mediaExt) => path.join(mediaDir, `${base}${mediaExt}`));
    const imageFile = fallbackCandidates.find((candidate) => existsSync(candidate)) || "";
    games.push({
      name,
      base,
      title: base,
      format: ext.slice(1),
      path: path.join(romDir, name),
      description: "",
      developer: "",
      publisher: "",
      genre: "",
      players: "",
      rating: 0,
      releaseDate: "",
      playCount: 0,
      lastPlayed: "",
      hasMedia: Boolean(imageFile),
      media: {
        image: registerAsset(imageFile),
        thumbnail: registerAsset(imageFile),
        boxart: registerAsset(imageFile),
      },
    });
  }

  games.sort((a, b) => a.title.localeCompare(b.title, undefined, { numeric: true, sensitivity: "base" }));
  await sendJson(res, {
    roots: { romDir, biosDir, mediaDir, gamelistPath },
    roms: games,
    biosArchives,
    mediaCount: assetRegistry.size,
    mediaMatched: games.filter((game) => game.hasMedia).length,
  });
}

async function handleMedia(req, res, assetId) {
  const file = assetRegistry.get(assetId);
  if (!file || !existsSync(file) || !mediaExtensions.has(path.extname(file).toLowerCase())) {
    res.writeHead(404);
    res.end("Media not found");
    return;
  }

  const info = await stat(file);
  const range = req.headers.range;
  const headers = {
    "content-type": contentType(file),
    "accept-ranges": "bytes",
    "cache-control": "public, max-age=3600",
  };
  if (!range) {
    res.writeHead(200, { ...headers, "content-length": info.size });
    createReadStream(file).pipe(res);
    return;
  }

  const match = /^bytes=(\d*)-(\d*)$/u.exec(range);
  if (!match) {
    res.writeHead(416, { "content-range": `bytes */${info.size}` });
    res.end();
    return;
  }
  const start = match[1] ? Number(match[1]) : 0;
  const end = match[2] ? Math.min(Number(match[2]), info.size - 1) : info.size - 1;
  if (start > end || start >= info.size) {
    res.writeHead(416, { "content-range": `bytes */${info.size}` });
    res.end();
    return;
  }
  res.writeHead(206, {
    ...headers,
    "content-range": `bytes ${start}-${end}/${info.size}`,
    "content-length": end - start + 1,
  });
  createReadStream(file, { start, end }).pipe(res);
}

async function handleLaunch(req, res) {
  const exe = existsSync(emulatorExe)
    ? emulatorExe
    : path.join(projectRoot, "target", "release", "ngneon-emu.exe");
  if (!existsSync(exe)) {
    await sendJson(res, { error: `No encuentro el emulador: ${exe}` }, 404);
    return;
  }
  const payload = req.method === "POST" ? await readRequestJson(req) : {};
  const args = [];
  if (payload.romPath) {
    const romPath = path.resolve(String(payload.romPath));
    if (!isPlayableRomName(path.basename(romPath)) || !existsSync(romPath)) {
      await sendJson(res, { error: `ROM invalida: ${romPath}` }, 400);
      return;
    }
    args.push(romPath);
  }

  const child = spawn(exe, args, {
    cwd: path.dirname(exe),
    detached: true,
    stdio: "ignore",
    windowsHide: false,
  });
  child.unref();
  await sendJson(res, { ok: true, exe });
}

async function serveStatic(req, res) {
  const url = new URL(req.url ?? "/", `http://127.0.0.1:${port}`);
  const pathname = decodeURIComponent(url.pathname);
  const baseDir = existsSync(distDir) ? distDir : __dirname;
  const requested = pathname === "/" ? "index.html" : pathname.slice(1);
  const file = path.normalize(path.join(baseDir, requested));

  if (!file.startsWith(baseDir)) {
    res.writeHead(403);
    res.end("Forbidden");
    return;
  }

  const finalFile = existsSync(file) ? file : existsSync(distDir) ? path.join(distDir, "index.html") : devIndex;
  try {
    const info = await stat(finalFile);
    if (!info.isFile()) throw new Error("Not a file");
    res.writeHead(200, {
      "content-type": contentType(finalFile),
      "cache-control": finalFile.endsWith(".html") ? "no-cache" : "public, max-age=3600",
    });
    createReadStream(finalFile).pipe(res);
  } catch {
    res.writeHead(404);
    res.end("Not found");
  }
}

function contentType(file) {
  const ext = path.extname(file).toLowerCase();
  const types = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".svg": "image/svg+xml",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".webp": "image/webp",
    ".gif": "image/gif",
    ".avif": "image/avif",
    ".mp4": "video/mp4",
    ".webm": "video/webm",
    ".mov": "video/quicktime",
    ".m4v": "video/mp4",
    ".pdf": "application/pdf",
  };
  return types[ext] || "application/octet-stream";
}

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url ?? "/", `http://127.0.0.1:${port}`);
    const mediaMatch = url.pathname.match(/^\/api\/media\/([a-f0-9]+)$/u);
    if (req.method === "GET" && url.pathname === "/api/config") {
      await handleGetConfig(req, res);
    } else if (req.method === "GET" && url.pathname === "/api/library") {
      await handleLibrary(req, res);
    } else if (req.method === "GET" && url.pathname === "/api/ra/dashboard") {
      await handleRaDashboard(req, res);
    } else if (req.method === "GET" && mediaMatch) {
      await handleMedia(req, res, mediaMatch[1]);
    } else if (req.method === "POST" && url.pathname === "/api/config") {
      await handleSaveConfig(req, res);
    } else if (req.method === "POST" && url.pathname === "/api/ra/connect") {
      await handleRaConnect(req, res);
    } else if (req.method === "POST" && url.pathname === "/api/ra/disconnect") {
      await handleRaDisconnect(req, res);
    } else if (req.method === "POST" && url.pathname === "/api/launch") {
      await handleLaunch(req, res);
    } else {
      await serveStatic(req, res);
    }
  } catch (error) {
    await sendJson(res, { error: error instanceof Error ? error.message : String(error) }, 500);
  }
});

server.listen(port, "127.0.0.1", () => {
  console.log(`NGNEON Configurator: http://127.0.0.1:${port}`);
  console.log(`Root: ${projectRoot}`);
});
