// S2MM live position overlay, injected into joric's Stalker 2 map by the local S2MM server.
(function () {
  const EN = window.__s2mmLang !== "de";
  const L = (de, en) => (EN ? en : de);
  const css = `
    #s2mm-live{position:fixed;left:50%;transform:translateX(-50%);bottom:12px;z-index:9999;display:flex;align-items:center;gap:10px;
      padding:8px 12px;border-radius:8px;background:rgba(18,19,17,.92);color:#e9e5d8;border:1px solid #3d3e35;
      font:13px "Segoe UI",sans-serif;box-shadow:0 6px 24px rgba(0,0,0,.4)}
    #s2mm-live .dot{width:9px;height:9px;border-radius:50%;background:#6f6d62}
    #s2mm-live.on .dot{background:#8bb662}
    #s2mm-live .coords{font:12px Consolas,monospace;color:#a3a092}
    #s2mm-live button{background:#282922;color:#e9e5d8;border:1px solid #3d3e35;border-radius:6px;padding:4px 9px;cursor:pointer;font:inherit}
    #s2mm-live button.active{background:#e0a526;color:#1b1404;border-color:#e0a526;font-weight:600}
    #s2mm-layers{position:fixed;right:12px;top:92px;z-index:10001;width:250px;max-height:calc(100vh - 190px);display:flex;flex-direction:column;
      border-radius:8px;background:rgba(18,19,17,.94);color:#e9e5d8;border:1px solid #3d3e35;font:13px "Segoe UI",sans-serif;box-shadow:0 6px 24px rgba(0,0,0,.4)}
    #s2mm-layers header{display:flex;align-items:center;gap:8px;padding:8px 10px;user-select:none;font-weight:600}
    #s2mm-layers header span{flex:1}
    #s2mm-layers header button{background:#282922;color:#e9e5d8;border:1px solid #3d3e35;border-radius:6px;width:26px;height:22px;cursor:pointer;line-height:1}
    #s2mm-layers.s2mm-collapsed{width:250px}
    #s2mm-layers .body{overflow:auto;padding:0 10px 10px;border-top:1px solid #2d2e27}
    #s2mm-layers.s2mm-collapsed .body{display:none}
    #s2mm-layers h4{margin:10px 0 4px;font:600 11px "Segoe UI",sans-serif;letter-spacing:.08em;text-transform:uppercase;color:#6f6d62}
    #s2mm-layers label{display:flex;align-items:center;gap:8px;padding:3px 2px;border-radius:4px;cursor:pointer}
    #s2mm-layers label:hover{background:#282922}
    #s2mm-layers label .n{margin-left:auto;font:11px Consolas,monospace;color:#6f6d62}
    #s2mm-layers input[type=checkbox],#s2mm-layers input[type=radio]{accent-color:#e0a526}
    #s2mm-layers input[type=search]{width:100%;box-sizing:border-box;margin-top:8px;padding:5px 8px;border-radius:6px;border:1px solid #3d3e35;background:#121311;color:#e9e5d8}
    #s2mm-layers .row{display:flex;gap:6px;margin-top:6px}
    #s2mm-layers .row button{flex:1;background:#282922;color:#e9e5d8;border:1px solid #3d3e35;border-radius:6px;padding:3px;cursor:pointer;font:12px "Segoe UI",sans-serif}`;
  const style = document.createElement("style");
  style.textContent = css;
  document.head.appendChild(style);

  const panel = document.createElement("div");
  panel.id = "s2mm-live";
  panel.innerHTML = '<span class="dot"></span><strong>S2MM Live</strong><span class="coords">' + L("warte auf Spiel …", "waiting for the game …") + '</span><button type="button">' + L("Folgen", "Follow") + "</button>";
  document.body.appendChild(panel);
  const coords = panel.querySelector(".coords");
  const followBtn = panel.querySelector("button");

  let follow = true;
  let layer = null;
  let dot = null;
  let arrow = null;
  let hooked = false;
  const setFollow = (v) => {
    follow = v;
    followBtn.classList.toggle("active", v);
  };
  setFollow(true);
  followBtn.onclick = () => setFollow(!follow);

  function getMap() {
    return typeof window.__s2mmMap === "function" ? window.__s2mmMap() : null;
  }

  function ensureLayer(map) {
    if (layer) return true;
    if (!window.maptalks) return false;
    layer = new maptalks.VectorLayer("s2mm-live", { zIndex: 1000, enableAltitude: false }).addTo(map);
    dot = new maptalks.Marker([0, 0], {
      symbol: { markerType: "ellipse", markerWidth: 18, markerHeight: 18, markerFill: "#e0a526", markerLineColor: "#1b1404", markerLineWidth: 3 },
    }).addTo(layer);
    arrow = new maptalks.Marker([0, 0], {
      symbol: { markerType: "triangle", markerWidth: 14, markerHeight: 18, markerFill: "#e0a526", markerLineColor: "#1b1404", markerLineWidth: 2, markerDy: -16 },
    }).addTo(layer);
    if (!hooked) {
      map.on("dragstart", () => setFollow(false));
      hooked = true;
    }
    return true;
  }

  async function tick() {
    try {
      const r = await fetch("/s2mm/live", { cache: "no-store" });
      const p = await r.json();
      const live = p.ok && p.age < 6;
      panel.classList.toggle("on", live);
      if (!p.ok) {
        coords.textContent = L("keine Daten – Spiel mit S2MM-Live-Map-Mod starten", "no data – start the game with the S2MM live map mod");
        return;
      }
      coords.textContent = `${live ? "" : L("(veraltet) ", "(stale) ")}X ${Math.round(p.x)}  Y ${Math.round(p.y)}  Z ${Math.round(p.z)}`;
      const map = getMap();
      if (!map || !ensureLayer(map)) return;
      dot.setCoordinates([p.x, p.y]);
      arrow.setCoordinates([p.x, p.y]);
      // Unreal yaw: 0 = +X (right on the map), 90 = +Y (down on the map); markerRotation is counter-clockwise.
      arrow.updateSymbol({ markerRotation: -(p.yaw + 90) });
      if (follow) map.panTo([p.x, p.y], { duration: 400 });
    } catch (e) {
      panel.classList.remove("on");
      coords.textContent = L("S2MM nicht erreichbar", "S2MM not reachable");
    }
  }
  setInterval(tick, 1000);
  tick();

  // ---------------------------------------------------------------- layer filter
  const names = {
    notes: "Notizen", regions: "Regionen", hubs: "Siedlungen", bases: "Basen", locations: "Orte", stashes: "Verstecke",
    items: "Gegenstände", anomalies: "Anomalien", misc: "Sonstiges", enemies: "Gegner", circles: "Kreise", polygons: "Gebiete",
    quests: "Quests", artifacts: "Artefakte", traders: "Händler", technicians: "Techniker", medics: "Ärzte", guides: "Führer",
    shelters: "Unterschlupf", campfires: "Lagerfeuer", ladders: "Leitern", doors: "Türen", keys: "Schlüssel", weapons: "Waffen",
    armor: "Rüstungen", upgrades: "Upgrades", documents: "Dokumente", photos: "Fotos", locks: "Schlösser",
    heightmap: "Höhenpunkte", lines: "Verbindungen", actors: "NPCs & Objekte", helpers: "Hilfspunkte",
  };
  const baseNames = {
    T_ZU_UndergroundMapDLC_UDIM: "Karte + Untergrund (DLC)", T_DLCWorldMap_UDIM: "Weltkarte (DLC)", diegetic: "PDA-Karte",
    T_DLCWorldMap_UDIM_nofog: "Weltkarte ohne Nebel", T_ZU_UndergroundMap_UDIM: "Untergrund", topographic: "Topografisch",
    relief: "Relief", blank: "Leer",
  };
  const baseNamesEn = {
    T_ZU_UndergroundMapDLC_UDIM: "Map + underground (DLC)", T_DLCWorldMap_UDIM: "World map (DLC)", diegetic: "PDA map",
    T_DLCWorldMap_UDIM_nofog: "World map without fog", T_ZU_UndergroundMap_UDIM: "Underground", topographic: "Topographic",
    relief: "Relief", blank: "Blank",
  };
  const namesEn = { heightmap: "Height points", lines: "Connections", actors: "NPCs & objects", helpers: "Helper points", polygons: "Areas", hubs: "Settlements" };
  const pretty = (id) => id.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());
  const label = (id) => (EN ? namesEn[id.toLowerCase()] || pretty(id) : names[id.toLowerCase()] || pretty(id));
  const baseLabel = (id) => (EN ? baseNamesEn[id] : baseNames[id]) || id.replace(/_/g, " ");

  const box = document.createElement("div");
  box.id = "s2mm-layers";
  let collapsed = localStorage.getItem("s2mm-layers-collapsed") === "1";
  box.innerHTML = '<header><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#e0a526" stroke-width="2"><path d="m12 2 10 5-10 5L2 7z"/><path d="m2 17 10 5 10-5"/><path d="m2 12 10 5 10-5"/></svg><span>' + L("Ebenen", "Layers") + '</span><button type="button" title="' + L("Ein-/ausklappen", "Collapse/expand") + '"></button></header><div class="body"></div>';
  document.body.appendChild(box);
  const body = box.querySelector(".body");
  const toggleBtn = box.querySelector("header button");
  const applyCollapsed = () => {
    box.classList.toggle("s2mm-collapsed", collapsed);
    toggleBtn.textContent = collapsed ? "▸" : "▾";
    toggleBtn.setAttribute("aria-expanded", String(!collapsed));
  };
  applyCollapsed();
  toggleBtn.onclick = () => {
    collapsed = !collapsed;
    applyCollapsed();
    try { localStorage.setItem("s2mm-layers-collapsed", collapsed ? "1" : "0"); } catch (e) {}
  };
  // Keep clicks, drags and wheel inside our panels away from the map underneath.
  for (const el of [box, panel]) {
    for (const ev of ["mousedown", "mouseup", "click", "dblclick", "pointerdown", "pointerup", "touchstart", "wheel", "contextmenu"]) {
      el.addEventListener(ev, (e) => e.stopPropagation(), { passive: true });
    }
  }
  // Re-attach if the page ever replaces our elements.
  setInterval(() => {
    for (const el of [box, panel, style]) {
      if (!document.body.contains(el) && !document.head.contains(el)) (el === style ? document.head : document.body).appendChild(el);
    }
  }, 1000);
  let filter = "";

  function overlays() {
    const g = typeof window.__s2mmGroup === "function" ? window.__s2mmGroup() : null;
    return g ? g.getLayers().filter((l) => l.getId && l.getId() !== "s2mm-live") : [];
  }
  function bases() {
    const map = getMap();
    const base = map && map.getBaseLayer && map.getBaseLayer();
    return base && base.getLayers ? base.getLayers() : [];
  }
  function count(l) {
    try { return l.getGeometries ? l.getGeometries().length : ""; } catch (e) { return ""; }
  }

  function render() {
    const ovs = overlays();
    if (!ovs.length) return false;
    const q = filter.toLowerCase();
    let html = '<input type="search" placeholder="' + L("Ebene suchen …", "Search layers …") + '" value="' + filter.replace(/"/g, "") + '">';
    html += '<div class="row"><button data-all="1">' + L("Alle", "All") + '</button><button data-all="0">' + L("Keine", "None") + "</button></div><h4>" + L("Markierungen", "Markers") + "</h4>";
    for (const l of ovs) {
      const id = l.getId();
      if (q && !label(id).toLowerCase().includes(q) && !id.toLowerCase().includes(q)) continue;
      html += '<label><input type="checkbox" data-layer="' + id + '"' + (l.isVisible() ? " checked" : "") + "> " + label(id) + '<span class="n">' + count(l) + "</span></label>";
    }
    const bs = bases();
    if (bs.length) {
      html += "<h4>" + L("Grundkarte", "Base map") + "</h4>";
      for (const b of bs) {
        const id = b.getId();
        html += '<label><input type="radio" name="s2mm-base" data-base="' + id + '"' + (b.isVisible() ? " checked" : "") + "> " + baseLabel(id) + "</label>";
      }
    }
    body.innerHTML = html;
    const search = body.querySelector("input[type=search]");
    search.oninput = () => {
      filter = search.value;
      render();
      const s = body.querySelector("input[type=search]");
      s.focus();
      s.setSelectionRange(s.value.length, s.value.length);
    };
    body.querySelectorAll("[data-layer]").forEach((cb) => {
      cb.onchange = () => {
        const l = overlays().find((x) => x.getId() === cb.dataset.layer);
        if (l) cb.checked ? l.show() : l.hide();
      };
    });
    body.querySelectorAll("[data-all]").forEach((btn) => {
      btn.onclick = () => {
        const show = btn.dataset.all === "1";
        for (const l of overlays()) {
          if (q && !label(l.getId()).toLowerCase().includes(q) && !l.getId().toLowerCase().includes(q)) continue;
          show ? l.show() : l.hide();
        }
        render();
      };
    });
    body.querySelectorAll("[data-base]").forEach((rb) => {
      rb.onchange = () => {
        for (const b of bases()) b.getId() === rb.dataset.base ? b.show() : b.hide();
      };
    });
    return true;
  }
  // The map builds its layers asynchronously; render once they exist and keep in sync.
  const wait = setInterval(() => {
    if (render()) {
      clearInterval(wait);
      const map = getMap();
      if (map) map.on("zoomend", () => box.contains(document.activeElement) || render());
    }
  }, 500);
})();
