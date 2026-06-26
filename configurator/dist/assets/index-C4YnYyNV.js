(function(){const t=document.createElement("link").relList;if(t&&t.supports&&t.supports("modulepreload"))return;for(const n of document.querySelectorAll('link[rel="modulepreload"]'))r(n);new MutationObserver(n=>{for(const o of n)if(o.type==="childList")for(const v of o.addedNodes)v.tagName==="LINK"&&v.rel==="modulepreload"&&r(v)}).observe(document,{childList:!0,subtree:!0});function s(n){const o={};return n.integrity&&(o.integrity=n.integrity),n.referrerPolicy&&(o.referrerPolicy=n.referrerPolicy),n.crossOrigin==="use-credentials"?o.credentials="include":n.crossOrigin==="anonymous"?o.credentials="omit":o.credentials="same-origin",o}function r(n){if(n.ep)return;n.ep=!0;const o=s(n);fetch(n.href,o)}})();const j=new Set(["scanlines","curvature","bloom","fullscreen","auto_save","muted","ra_hardcore","diagnostic_dumps","gamepad"]),D={scanlines:"Scanlines",curvature:"Curvatura CRT",bloom:"Phosphor bloom",fullscreen:"Pantalla completa",auto_save:"Auto-save",muted:"Silenciar",diagnostic_dumps:"Dumps de diagnóstico",gamepad:"Gamepad SDL2",ra_hardcore:"Modo hardcore"},A={video:"Vídeo",fanart:"Fanart",screenshot:"Captura",image:"Mix",titleshot:"Título",box3d:"Caja 3D",cartridge:"Cartucho",manual:"Manual"},C="ngneon-launcher-favorites-v1",N="ngneon-launcher-recents-v1",a={loaded:!1,dirty:!1,saving:!1,launching:!1,libraryLoading:!1,raBusy:!1,raLoading:!1,raStatus:"",error:null,toast:"",view:"library",response:null,library:null,ra:null,selectedRomPath:"",mediaMode:"video",search:"",filter:"all",draft:{},secretDraft:{},favorites:new Set(R(C)),recents:R(N)},P=document.querySelector("#app");if(!P)throw new Error("Missing #app root");const h=P;function R(e){try{const t=JSON.parse(localStorage.getItem(e)||"[]");return Array.isArray(t)?t.filter(s=>typeof s=="string"):[]}catch{return[]}}function w(){localStorage.setItem(C,JSON.stringify([...a.favorites])),localStorage.setItem(N,JSON.stringify(a.recents.slice(0,20)))}function k(e){return e==="on"||e==="true"||e==="1"}function p(e,t=""){const s=a.draft[e];return typeof s=="string"?s:t}function I(e,t){const s=Number(a.draft[e]);return Number.isFinite(s)?s:t}function B(e){const t={...e.config};for(const s of j)t[s]=k(e.config[s]);return t.volume=Number(e.config.volume??100),t.window_scale=Number(e.config.window_scale??3),t}async function f(e,t){const s=await fetch(e,{headers:{"content-type":"application/json"},...t});if(!s.ok){const r=await s.text();let n=r;try{n=JSON.parse(r).error||r}catch{}throw new Error(n||`HTTP ${s.status}`)}return await s.json()}async function g(){try{a.error=null;const e=await f("/api/config");a.response=e,a.draft=B(e),a.loaded=!0,a.dirty=!1,await Promise.all([E(!1),O(!1)])}catch(e){a.error=e instanceof Error?e.message:String(e)}l()}async function E(e=!0){a.libraryLoading=!0,e&&l();try{a.library=await f("/api/library"),(!a.selectedRomPath||!a.library.roms.some(t=>t.path===a.selectedRomPath))&&(a.selectedRomPath=a.library.roms[0]?.path??"")}catch(t){a.error=t instanceof Error?t.message:String(t)}finally{a.libraryLoading=!1,e&&l()}}async function O(e=!0){a.raLoading=!0,e&&l();try{a.ra=await f("/api/ra/dashboard")}catch(t){a.ra={available:!1,message:t instanceof Error?t.message:String(t)}}finally{a.raLoading=!1,e&&l()}}function H(){return Object.fromEntries(Object.entries(a.draft).map(([e,t])=>[e,typeof t=="boolean"?t?"on":"off":String(t)]))}function q(){return Object.fromEntries(Object.entries(a.secretDraft).filter(([,e])=>e.length>0))}async function V(){a.saving=!0,a.error=null,l();try{await f("/api/config",{method:"POST",body:JSON.stringify({config:H(),secrets:q()})}),a.secretDraft={},a.dirty=!1,a.toast="Configuración guardada",await g()}catch(e){a.error=e instanceof Error?e.message:String(e)}finally{a.saving=!1,l()}}async function G(){a.raBusy=!0,a.error=null,a.raStatus="Conectando con RetroAchievements…",l();try{const e=await f("/api/ra/connect",{method:"POST",body:JSON.stringify({username:p("ra_username"),password:a.secretDraft.ra_password??"",token:a.secretDraft.ra_token??"",apiKey:a.secretDraft.ra_api_key??""})});a.secretDraft={},a.dirty=!1,a.raStatus=`Conectado como ${e.username} · ${c(e.score)} puntos`,await g(),a.view="achievements"}catch(e){a.raStatus=e instanceof Error?e.message:String(e)}finally{a.raBusy=!1,l()}}async function F(){a.raBusy=!0,a.raStatus="Desconectando…",l();try{await f("/api/ra/disconnect",{method:"POST",body:"{}"}),a.secretDraft={},a.dirty=!1,a.raStatus="RetroAchievements desconectado.",await g()}catch(e){a.raStatus=e instanceof Error?e.message:String(e)}finally{a.raBusy=!1,l()}}async function _(){const e=b();if(e){a.launching=!0,a.error=null,l();try{await f("/api/launch",{method:"POST",body:JSON.stringify({romPath:e.path})}),a.recents=[e.path,...a.recents.filter(t=>t!==e.path)].slice(0,20),w(),a.toast=`${e.title} iniciado`}catch(t){a.error=t instanceof Error?t.message:String(t)}finally{a.launching=!1,l()}}}function i(e){return String(e).replaceAll("&","&amp;").replaceAll('"',"&quot;").replaceAll("'","&#39;").replaceAll("<","&lt;").replaceAll(">","&gt;")}function c(e){return new Intl.NumberFormat("es-ES").format(e||0)}function L(e){if(!e)return"";const t=new Date(e.replace(" ","T")+(e.includes("Z")?"":"Z"));return Number.isNaN(t.getTime())?e:new Intl.DateTimeFormat("es-ES",{day:"2-digit",month:"short",year:"numeric"}).format(t)}function b(){return a.library?.roms.find(e=>e.path===a.selectedRomPath)??null}function T(){const e=a.search.trim().toLocaleLowerCase("es"),t=new Set(a.recents);return(a.library?.roms??[]).filter(s=>a.filter==="favorites"&&!a.favorites.has(s.path)||a.filter==="recent"&&!t.has(s.path)?!1:e?[s.title,s.base,s.developer,s.publisher,s.genre].join(" ").toLocaleLowerCase("es").includes(e):!0)}function y(e,...t){for(const s of t){const r=e.media[s];if(r)return r}return""}function U(e){return[e.developer,e.publisher,e.genre,e.players?`${e.players} jugador${e.players==="1"?"":"es"}`:""].filter(Boolean).slice(0,4)}function d(e){return`<svg viewBox="0 0 24 24" aria-hidden="true">${{library:'<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 8h10M7 12h10M7 16h6"/>',achievement:'<path d="M8 3h8v4a4 4 0 0 1-8 0V3Z"/><path d="M8 5H4v1a4 4 0 0 0 4 4M16 5h4v1a4 4 0 0 1-4 4M12 11v5M8 21h8M9 16h6v5H9z"/>',settings:'<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21h-4v-.1A1.7 1.7 0 0 0 8.6 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-1.6-1H3v-4h.1A1.7 1.7 0 0 0 4.6 9a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6V3h4v.1A1.7 1.7 0 0 0 15 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 9a1.7 1.7 0 0 0 1.6 1h.1v4H21a1.7 1.7 0 0 0-1.6 1Z"/>',search:'<circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>',play:'<path d="m8 5 11 7-11 7V5Z"/>',heart:'<path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8l1.1 1.1L12 21l7.8-7.5 1.1-1.1a5.5 5.5 0 0 0-.1-7.8Z"/>',refresh:'<path d="M20 6v5h-5M4 18v-5h5"/><path d="M18.5 9A7 7 0 0 0 6 6.5L4 9m16 6-2 2.5A7 7 0 0 1 5.5 15"/>'}[e]}</svg>`}function K(){const e=a.ra?.profile;return`
    <header class="app-header">
      <button class="brand" data-action="view" data-view="library" aria-label="Ir a biblioteca">
        <span class="brand-mark">N</span>
        <span><strong>NGNEON</strong><small>NEO GEO SYSTEM</small></span>
      </button>
      <nav class="main-nav" aria-label="Navegación principal">
        <button class="${a.view==="library"?"is-active":""}" data-action="view" data-view="library">
          ${d("library")}<span>Biblioteca</span>
        </button>
        <button class="${a.view==="achievements"?"is-active":""}" data-action="view" data-view="achievements">
          ${d("achievement")}<span>RetroAchievements</span>
        </button>
        <button class="${a.view==="settings"?"is-active":""}" data-action="view" data-view="settings">
          ${d("settings")}<span>Ajustes</span>
        </button>
      </nav>
      <button class="profile-chip" data-action="view" data-view="achievements">
        ${e?.avatar?`<img src="${i(e.avatar)}" alt="" />`:`<span>${i((p("ra_username","RA")[0]||"R").toUpperCase())}</span>`}
        <span><strong>${i(e?.username||p("ra_username","Sin conectar"))}</strong><small>${e?`${c(e.points)} pts`:"RetroAchievements"}</small></span>
      </button>
    </header>
  `}function J(e){const t=e.media[a.mediaMode],s=a.mediaMode==="video"?t:"",r=s?y(e,"fanart","screenshot","image"):t||y(e,"fanart","screenshot","image","mix"),n=["image","box3d","cartridge"].includes(a.mediaMode)?" hero-media--portrait":"";return s?`
      <div class="hero-media-stage">
        ${r?`<img class="hero-media-backdrop" src="${i(r)}" alt="" aria-hidden="true" />`:""}
        <video class="hero-media" src="${i(s)}" poster="${i(r)}" autoplay muted loop playsinline preload="metadata"></video>
      </div>
      <div class="hero-vignette"></div>
    `:r?`
      <div class="hero-media-stage">
        <img class="hero-media-backdrop" src="${i(r)}" alt="" aria-hidden="true" />
        <img class="hero-media${n}" src="${i(r)}" alt="" />
      </div>
      <div class="hero-vignette"></div>
    `:'<div class="hero-fallback"><span>NG</span></div><div class="hero-vignette"></div>'}function z(e){return["video","fanart","screenshot","image","titleshot","box3d","cartridge","manual"].filter(s=>e.media[s]).map(s=>`
        <button class="${a.mediaMode===s?"is-active":""}" data-action="${s==="manual"?"open-media":"media"}" data-media="${s}" title="${A[s]??s}">
          <span>${s==="video"?"▶":s==="manual"?"↗":"●"}</span>${A[s]??s}
        </button>
      `).join("")}function Z(){if(a.raLoading)return'<aside class="ra-mini skeleton-panel"></aside>';const e=a.ra;if(!e?.available||!e.profile)return`
      <aside class="ra-mini ra-mini-empty">
        <div class="section-label">${d("achievement")}<span>RetroAchievements</span></div>
        <strong>${i(e?.basic?.username||p("ra_username","Conecta tu cuenta"))}</strong>
        <p>${i(e?.message||"Consulta tu progreso y últimos logros mientras eliges juego.")}</p>
        ${e?.basic?`<div class="score-big">${c(e.basic.score)}<small>puntos</small></div>`:""}
        <button class="text-button" data-action="view" data-view="settings">Configurar cuenta →</button>
      </aside>
    `;const t=e.recentAchievements?.[0];return`
    <aside class="ra-mini">
      <div class="ra-mini-profile">
        <img src="${i(e.profile.avatar)}" alt="" />
        <div><strong>${i(e.profile.username)}</strong><span>Rango #${c(e.profile.rank)}</span></div>
        <span class="online-dot ${e.profile.status.toLowerCase()==="online"?"is-online":""}"></span>
      </div>
      <div class="ra-points">
        <div><strong>${c(e.profile.points)}</strong><span>puntos</span></div>
        <div><strong>${c(e.profile.truePoints)}</strong><span>true points</span></div>
      </div>
      ${t?`
        <div class="latest-unlock">
          <span>Último logro</span>
          <div>
            <img src="${i(t.badge)}" alt="" />
            <p><strong>${i(t.title)}</strong><small>${i(t.gameTitle)} · ${t.points} pts</small></p>
          </div>
        </div>
      `:'<p class="quiet">Aún no hay logros recientes.</p>'}
      <button class="text-button" data-action="view" data-view="achievements">Ver actividad completa →</button>
    </aside>
  `}function W(){const e=b(),t=T();if(!e)return'<section class="empty-screen"><h1>No hay juegos</h1><p>Revisa la ruta de ROMs en Ajustes y vuelve a escanear.</p></section>';const s=U(e),r=a.favorites.has(e.path);return`
    <main class="launcher">
      <section class="hero">
        ${J(e)}
        <div class="hero-content">
          ${e.media.wheel?`<img class="game-logo" src="${i(e.media.wheel)}" alt="${i(e.title)}" />`:`<h1>${i(e.title)}</h1>`}
          <div class="game-meta">${s.map(n=>`<span>${i(n)}</span>`).join("")}<span>${e.format.toUpperCase()}</span></div>
          <p class="game-description">${i(e.description||"Sin descripción disponible en gamelist.xml.")}</p>
          <div class="hero-actions">
            <button class="play-button" data-action="launch" ${a.launching?"disabled":""}>${d("play")}<span>${a.launching?"ABRIENDO…":"JUGAR"}</span></button>
            <button class="round-button ${r?"is-favorite":""}" data-action="favorite" title="Favorito">${d("heart")}</button>
          </div>
          <div class="media-tabs">${z(e)}</div>
        </div>
        ${Z()}
      </section>

      <section class="library-section">
        <div class="library-toolbar">
          <div>
            <h2>Tu colección</h2>
            <p>${a.library?.roms.length??0} juegos · ${a.library?.mediaMatched??0} con media</p>
          </div>
          <div class="library-controls">
            <div class="filter-group">
              <button class="${a.filter==="all"?"is-active":""}" data-action="filter" data-filter="all">Todos</button>
              <button class="${a.filter==="favorites"?"is-active":""}" data-action="filter" data-filter="favorites">Favoritos</button>
              <button class="${a.filter==="recent"?"is-active":""}" data-action="filter" data-filter="recent">Recientes</button>
            </div>
            <label class="search-box">${d("search")}<input type="search" data-action="search" value="${i(a.search)}" placeholder="Buscar juego…" /></label>
            <button class="icon-button" data-action="rescan" title="Volver a escanear">${d("refresh")}</button>
          </div>
        </div>
        <div class="game-rail" data-rail>
          ${t.length?t.map(Y).join(""):'<div class="no-results">No hay juegos para este filtro.</div>'}
        </div>
        <div class="keyboard-hint"><span>← →</span> navegar <span>Enter</span> jugar <span>F</span> favorito</div>
      </section>
    </main>
  `}function Y(e){const t=e.path===a.selectedRomPath,s=y(e,"box3d","box2d","thumbnail","image","mix");return`
    <button class="game-card ${t?"is-selected":""}" data-action="select-rom" data-path="${encodeURIComponent(e.path)}" aria-label="${i(e.title)}">
      <span class="cover-frame">
        ${s?`<img src="${i(s)}" loading="lazy" alt="" />`:'<span class="cover-placeholder">NG</span>'}
        ${a.favorites.has(e.path)?'<span class="favorite-mark">♥</span>':""}
      </span>
      <strong>${i(e.title)}</strong>
      <small>${i(e.developer||e.publisher||"Neo Geo")} · ${e.format.toUpperCase()}</small>
    </button>
  `}function Q(){if(a.raLoading)return'<main class="content-page"><div class="page-loading">Cargando actividad RetroAchievements…</div></main>';const e=a.ra;if(!e?.available||!e.profile)return`
      <main class="content-page ra-onboarding">
        <section>
          <div class="ra-emblem">${d("achievement")}</div>
          <h1>RetroAchievements, dentro de tu launcher</h1>
          <p>${i(e?.message||"Conecta tu cuenta para mostrar tu progreso.")}</p>
          ${e?.basic?`<div class="basic-score"><strong>${c(e.basic.score)}</strong><span>puntos de ${i(e.basic.username)}</span></div>`:""}
          <button class="primary-button" data-action="view" data-view="settings">Abrir ajustes de cuenta</button>
          ${e?.apiKeyUrl?`<a href="${i(e.apiKeyUrl)}" target="_blank" rel="noreferrer">Obtener Web API Key ↗</a>`:""}
        </section>
      </main>
    `;const t=e.profile,s=e.recentlyPlayed??[],r=e.recentAchievements??[];return`
    <main class="content-page achievements-page">
      <section class="profile-hero">
        <img src="${i(t.avatar)}" alt="" />
        <div class="profile-copy">
          <span class="profile-status"><i class="${t.status.toLowerCase()==="online"?"is-online":""}"></i>${i(t.status)}</span>
          <h1>${i(t.username)}</h1>
          <p>${i(t.richPresence||t.motto||"Preparado para el siguiente reto.")}</p>
          <div class="profile-actions">
            ${e.profileUrl?`<a class="primary-button" href="${i(e.profileUrl)}" target="_blank" rel="noreferrer">Ver perfil oficial ↗</a>`:""}
            <button class="secondary-button" data-action="refresh-ra">${d("refresh")} Actualizar</button>
          </div>
        </div>
        <div class="profile-stats">
          <div><strong>${c(t.points)}</strong><span>Puntos</span></div>
          <div><strong>#${c(t.rank)}</strong><span>Rango global</span></div>
          <div><strong>${c(t.truePoints)}</strong><span>True points</span></div>
        </div>
      </section>

      <section class="ra-grid">
        <div class="recent-games">
          <div class="section-heading"><div><h2>Últimos juegos</h2><p>Progreso sincronizado con RetroAchievements</p></div><span>${s.length}</span></div>
          <div class="recent-game-list">
            ${s.length?s.map(n=>`
              <article class="recent-game">
                <img src="${i(n.image||n.icon)}" alt="" />
                <div>
                  <h3>${i(n.title)}</h3>
                  <p>${i(n.consoleName)} · ${L(n.lastPlayed)}</p>
                  <div class="progress-line"><i style="width:${n.progress}%"></i></div>
                  <small>${n.achieved}/${n.achievementsTotal} logros · ${n.progress}%</small>
                </div>
              </article>
            `).join(""):'<p class="quiet">No hay partidas recientes.</p>'}
          </div>
        </div>

        <div class="unlock-feed">
          <div class="section-heading"><div><h2>Últimos logros</h2><p>Actividad de los últimos 30 días</p></div><span>${r.length}</span></div>
          <div class="achievement-list">
            ${r.length?r.map(n=>`
              <article class="achievement-item">
                <img src="${i(n.badge)}" alt="" />
                <div>
                  <div class="achievement-title"><h3>${i(n.title)}</h3><strong>+${n.points}</strong></div>
                  <p>${i(n.description)}</p>
                  <small>${i(n.gameTitle)} · ${L(n.date)}${n.hardcore?" · HARDCORE":""}</small>
                </div>
              </article>
            `).join(""):'<p class="quiet">Todavía no hay logros recientes.</p>'}
          </div>
        </div>
      </section>
    </main>
  `}function u(e){const t=!!a.draft[e];return`
    <button class="setting-toggle ${t?"is-on":""}" data-action="toggle" data-key="${e}">
      <span><strong>${D[e]??e}</strong><small>${t?"Activado":"Desactivado"}</small></span>
      <i></i>
    </button>
  `}function m(e,t,s=""){return`
    <label class="setting-field"><span>${t}</span>
      <input data-action="config-input" data-key="${e}" value="${i(p(e))}" placeholder="${i(s)}" />
    </label>
  `}function $(e,t,s,r=""){return`
    <label class="setting-field"><span>${t}${s?"<em>Configurado</em>":""}</span>
      <input type="password" autocomplete="off" data-action="secret-input" data-key="${e}" value="${i(a.secretDraft[e]??"")}" placeholder="${s?"Escribe para sustituirlo":"No configurado"}" />
      ${r?`<small>${r}</small>`:""}
    </label>
  `}function M(e,t,s,r,n){const o=I(e,e==="volume"?100:3);return`
    <label class="setting-range"><span><strong>${t}</strong><em>${o}${e==="volume"?"%":"×"}</em></span>
      <input type="range" min="${s}" max="${r}" step="${n}" value="${o}" data-action="range" data-key="${e}" />
    </label>
  `}function X(e,t,s,r=""){const n=p(e,r);return`
    <label class="setting-field"><span>${t}</span><select data-action="config-input" data-key="${e}">
      ${s.map(o=>`<option value="${i(o.value)}" ${o.value===n?"selected":""}>${i(o.text)}</option>`).join("")}
    </select></label>
  `}function ee(){const e=a.response;if(!e)return"";const t=a.library,s=(t?.biosArchives??[]).map(r=>`${r}:uni-bios_4_0.rom`);return`
    <main class="content-page settings-page">
      <div class="settings-heading">
        <div><h1>Ajustes</h1><p>Configuración del emulador y servicios conectados.</p></div>
        <div>
          <button class="secondary-button" data-action="rescan">${d("refresh")} Escanear biblioteca</button>
          <button class="primary-button" data-action="save" ${a.saving?"disabled":""}>${a.saving?"Guardando…":a.dirty?"Guardar cambios":"Todo guardado"}</button>
        </div>
      </div>
      <div class="settings-grid">
        <section class="settings-panel">
          <div class="settings-panel-title"><span>01</span><div><h2>Vídeo y audio</h2><p>Presentación, CRT y salida de sonido.</p></div></div>
          <div class="toggle-grid">${u("scanlines")}${u("curvature")}${u("bloom")}${u("fullscreen")}${u("muted")}</div>
          ${X("aspect_ratio","Relación de aspecto",[{value:"4:3",text:"4:3 original"},{value:"16:9",text:"16:9 panorámico"}],"4:3")}
          ${M("volume","Volumen",0,100,5)}
          ${M("window_scale","Escala de ventana",2,4,1)}
        </section>

        <section class="settings-panel">
          <div class="settings-panel-title"><span>02</span><div><h2>Sistema</h2><p>BIOS, idioma, guardado y controles.</p></div></div>
          <label class="setting-field"><span>BIOS activa</span><select data-action="config-input" data-key="bios">
            ${Array.from(new Set([p("bios"),...s].filter(Boolean))).map(r=>`<option ${r===p("bios")?"selected":""}>${i(r)}</option>`).join("")}
          </select></label>
          ${m("lang","Idioma","es / en")}
          <div class="toggle-grid">${u("auto_save")}${u("diagnostic_dumps")}${u("gamepad")}</div>
        </section>

        <section class="settings-panel settings-wide">
          <div class="settings-panel-title"><span>03</span><div><h2>Biblioteca y media</h2><p>Rutas resueltas y gamelist utilizado.</p></div></div>
          <div class="field-grid">${m("rom_path","ROMs","roms")}${m("bios_dir","BIOS","bios")}${m("media_dir","Media de respaldo","media")}</div>
          <div class="path-summary">
            <span><strong>gamelist.xml</strong>${i(t?.roots.gamelistPath||"No detectado")}</span>
            <span><strong>Media indexada</strong>${c(t?.mediaCount??0)} recursos</span>
            <span><strong>Juegos</strong>${c(t?.roms.length??0)} detectados</span>
          </div>
        </section>

        <section class="settings-panel settings-wide ra-settings">
          <div class="settings-panel-title"><span>04</span><div><h2>RetroAchievements</h2><p>Login del emulador y datos ampliados del perfil.</p></div></div>
          <div class="field-grid">
            ${m("ra_username","Usuario RA","Tu usuario")}
            ${$("ra_password","Contraseña RA",e.secrets.raPasswordConfigured)}
            ${$("ra_token","Token rcheevos",e.secrets.raTokenConfigured,"Lo usa el emulador para iniciar sesión y desbloquear logros.")}
            ${$("ra_api_key","Web API Key",e.secrets.raApiKeyConfigured,"Permite cargar perfil, rango, juegos recientes y últimos logros.")}
          </div>
          <div class="ra-settings-footer">
            <div>${u("ra_hardcore")}</div>
            <div class="ra-connect-actions">
              <button class="secondary-button danger" data-action="ra-disconnect" ${a.raBusy?"disabled":""}>Desconectar</button>
              <button class="primary-button" data-action="ra-connect" ${a.raBusy?"disabled":""}>${a.raBusy?"Conectando…":"Conectar y comprobar"}</button>
            </div>
          </div>
          ${a.raStatus?`<p class="status-message">${i(a.raStatus)}</p>`:""}
          <p class="security-note">Las credenciales se guardan solo en <code>config/ngneon.conf</code> y nunca se devuelven al navegador.</p>
        </section>
      </div>
    </main>
  `}function l(){if(!a.loaded){h.innerHTML=`
      <main class="boot">
        <div class="boot-logo">N</div>
        <h1>NGNEON</h1>
        <p>${a.error?i(a.error):"Preparando tu colección…"}</p>
        ${a.error?'<button data-action="reload">Reintentar</button>':'<div class="loading-line"><i></i></div>'}
      </main>
    `;return}const e=a.view==="library"?W():a.view==="achievements"?Q():ee();h.innerHTML=`
    <div class="app-shell">
      ${K()}
      ${a.error?`<div class="error-banner">${i(a.error)}<button data-action="dismiss-error">×</button></div>`:""}
      ${e}
      ${a.toast?`<div class="toast">${i(a.toast)}</div>`:""}
    </div>
  `,ae(),a.toast&&window.setTimeout(()=>{a.toast="",document.querySelector(".toast")?.remove()},2600)}function ae(){requestAnimationFrame(()=>{document.querySelector(".game-card.is-selected")?.scrollIntoView({behavior:"smooth",block:"nearest",inline:"nearest"})})}function x(e){a.selectedRomPath=e;const t=b();a.mediaMode=t?.media.video?"video":t?.media.fanart?"fanart":"image",l()}h.addEventListener("click",e=>{const t=e.target;if(!(t instanceof HTMLElement))return;const s=t.closest("[data-action]");if(!s)return;const r=s.dataset.action,n=s.dataset.key;if(r==="view")a.view=s.dataset.view||"library",l();else if(r==="toggle"&&n)a.draft[n]=!a.draft[n],a.dirty=!0,l();else if(r==="save")V();else if(r==="reload")g();else if(r==="rescan")E();else if(r==="refresh-ra")O();else if(r==="launch")_();else if(r==="ra-connect")G();else if(r==="ra-disconnect")F();else if(r==="select-rom")x(decodeURIComponent(s.dataset.path??""));else if(r==="favorite"){const o=a.selectedRomPath;a.favorites.has(o)?a.favorites.delete(o):a.favorites.add(o),w(),l()}else if(r==="filter")a.filter=s.dataset.filter||"all",l();else if(r==="media")a.mediaMode=s.dataset.media||"image",l();else if(r==="open-media"){const o=b(),v=s.dataset.media,S=o?.media[v];S&&window.open(S,"_blank","noopener,noreferrer")}else r==="dismiss-error"&&(a.error=null,l())});h.addEventListener("input",e=>{const t=e.target;if(!(t instanceof HTMLInputElement)&&!(t instanceof HTMLSelectElement))return;const s=t.dataset.action,r=t.dataset.key;s==="search"?(a.search=t.value,l()):s==="range"&&r?(a.draft[r]=Number(t.value),a.dirty=!0,l()):s==="config-input"&&r?(a.draft[r]=t.value,a.dirty=!0,document.querySelector('[data-action="save"]')?.replaceChildren("Guardar cambios")):s==="secret-input"&&r&&(a.secretDraft[r]=t.value,a.dirty=!0,document.querySelector('[data-action="save"]')?.replaceChildren("Guardar cambios"))});document.addEventListener("keydown",e=>{if(a.view!=="library"||e.target instanceof HTMLInputElement||e.target instanceof HTMLSelectElement)return;const t=T();if(!t.length)return;const s=Math.max(0,t.findIndex(r=>r.path===a.selectedRomPath));if(e.key==="ArrowRight"||e.key==="ArrowLeft"){e.preventDefault();const r=e.key==="ArrowRight"?1:-1,n=(s+r+t.length)%t.length;x(t[n].path)}else if(e.key==="Enter")e.preventDefault(),_();else if(e.key.toLowerCase()==="f"){e.preventDefault();const r=a.selectedRomPath;a.favorites.has(r)?a.favorites.delete(r):a.favorites.add(r),w(),l()}});g();
