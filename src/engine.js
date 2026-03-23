function js_now() { return performance.now(); }

// ─── persistent state (called by WASM store/remove builtins) ──────────────────
function js_persist_state(json) {
    fetch('/state', {
        method:  'POST',
        headers: { 'Content-Type': 'application/json' },
        body:    json,
    }).catch(e => console.warn('[state] persist failed:', e));
}

// ─── loading overlay ──────────────────────────────────────────────────────────
const _loadEl = document.createElement('div');
_loadEl.style.cssText = [
    'position:fixed','inset:0','background:#05050f',
    'display:flex','flex-direction:column',
    'align-items:center','justify-content:center',
    'z-index:99999','font:14px/1.8 monospace','color:#8af',
].join(';');
_loadEl.innerHTML = `
  <div style="width:360px">
    <div style="font:bold 20px monospace;color:#fff;letter-spacing:0.08em;margin-bottom:4px">bruecke</div>
    <div style="font:11px monospace;color:#334;margin-bottom:18px">real-time Python → WebGPU</div>
    <div style="background:#0d0d20;border-radius:4px;height:5px;overflow:hidden;margin-bottom:10px">
      <div id="_bar" style="height:100%;width:0%;background:linear-gradient(90deg,#28e,#8af);border-radius:4px;transition:width 0.2s ease-out"></div>
    </div>
    <div style="display:flex;justify-content:space-between;margin-bottom:16px">
      <div id="_status" style="color:#6af;font-size:12px">initialising...</div>
      <div id="_detail" style="color:#446;font-size:12px;font-variant-numeric:tabular-nums"></div>
    </div>
    <div id="_steps" style="color:#334;font-size:11px;line-height:1.7;border-top:1px solid #0d0d20;padding-top:10px"></div>
  </div>
`;
document.body.appendChild(_loadEl);
document.body.style.userSelect = 'none';

let _t0 = performance.now();

function _fmt(bytes) {
    return bytes >= 1048576
        ? (bytes / 1048576).toFixed(2) + ' MB'
        : (bytes / 1024).toFixed(0) + ' KB';
}
function _elapsed() {
    return ((performance.now() - _t0) / 1000).toFixed(1) + 's';
}
function _setBar(pct, status, detail) {
    const b = document.getElementById('_bar');
    const s = document.getElementById('_status');
    const d = document.getElementById('_detail');
    if (b) b.style.width = Math.min(pct, 100) + '%';
    if (s && status != null) s.textContent = status;
    if (d && detail != null) d.textContent = detail;
}

// ─── real WASM download progress (0 → 60% of bar) ────────────────────────────
async function _brueckeStart() {
    const res = await fetch('/bruecke_bg.wasm');
    const total = parseInt(res.headers.get('Content-Length') || '0', 10);
    let loaded = 0;
    _setBar(0, 'Downloading WASM...', total ? `0 / ${_fmt(total)}` : '0 KB');

    const tracked = new Response(
        new ReadableStream({
            start(controller) {
                const reader = res.body.getReader();
                function pump() {
                    reader.read().then(({ done, value }) => {
                        if (done) { controller.close(); return; }
                        loaded += value.byteLength;
                        const pct    = total > 0 ? (loaded / total) * 60 : 20;
                        const detail = total > 0
                            ? `${_fmt(loaded)} / ${_fmt(total)}  •  ${_elapsed()}`
                            : `${_fmt(loaded)}  •  ${_elapsed()}`;
                        _setBar(pct, 'Downloading WASM...', detail);
                        controller.enqueue(value);
                        pump();
                    }).catch(e => controller.error(e));
                }
                pump();
            }
        }),
        { headers: res.headers }
    );

    await init({ module_or_path: tracked });

    // hydrate persistent state from server → WASM memory
    try {
        const sr = await fetch('/state');
        if (sr.ok) { set_state(await sr.text()); _step('state loaded'); }
    } catch (_) {}

    await main();
}

// ─── step logger (60 → 100% of bar, 6 steps) ─────────────────────────────────
let _stepIdx = 0;
const _STEP_COUNT = 6;

function _step(msg, done) {
    const ms = (performance.now() - _t0).toFixed(0);
    console.log(`[bruecke] ${msg} (+${ms}ms)`);
    const steps = document.getElementById('_steps');
    if (done) {
        _setBar(100, msg, _elapsed());
        if (steps) steps.innerHTML += `<span style="color:#2a4">✓</span> ${msg}<br>`;
        setTimeout(() => _loadEl.remove(), 400);
        return;
    }
    _stepIdx++;
    const pct = 60 + (_stepIdx / _STEP_COUNT) * 40;
    _setBar(pct, msg, _elapsed());
    if (steps) steps.innerHTML += `<span style="color:#2a4">✓</span> ${msg}<br>`;
}

function fatal(err) {
    console.error(err);
    const msg = String(err?.stack || err);
    document.body.style.cssText = 'margin:0;background:#0d0d1a;color:#f88;font:14px/1.6 monospace;padding:24px';
    document.body.innerHTML = '<b>bruecke error</b><br><pre style="white-space:pre-wrap">'
        + msg.replace(/</g, '&lt;') + '</pre>';
}

const canvas = document.getElementById('c');
canvas.style.cursor = 'none';

// ─── SVG cursor overlay (default bruecke cursor) ──────────────────────────────
const _cursorEl = document.createElement('img');
_cursorEl.src = '/cursor.svg';
_cursorEl.style.cssText = [
    'position:fixed','pointer-events:none','z-index:9999',
    'width:20px','height:17px',
    'transform:translate(-2px,-1px)',
].join(';');
document.body.appendChild(_cursorEl);
window.addEventListener('mousemove', e => {
    _cursorEl.style.left = e.clientX + 'px';
    _cursorEl.style.top  = e.clientY + 'px';
});

// ─── error overlay ────────────────────────────────────────────────────────────
const overlay = document.createElement('div');
overlay.style.cssText = [
    'display:none',
    'position:fixed',
    'top:0','left:0','right:0',
    'max-height:45vh',
    'overflow-y:auto',
    'background:rgba(15,0,0,0.93)',
    'color:#ff5555',
    'font:12px/1.65 monospace',
    'padding:14px 18px',
    'border-bottom:1px solid #ff333388',
    'white-space:pre-wrap',
    'word-break:break-word',
    'z-index:9999',
].join(';');
document.body.appendChild(overlay);

function showError(msg) {
    overlay.style.display = 'block';
    overlay.textContent   = '⚠ Python Error\n\n' + msg;
}
function clearError() {
    overlay.style.display = 'none';
    overlay.textContent   = '';
}

let dpr = window.devicePixelRatio || 1;

// ─── shared pipeline blend config ────────────────────────────────────────────
const ALPHA_BLEND = {
    color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
    alpha: { srcFactor: 'one',       dstFactor: 'zero',                operation: 'add' },
};

// ─── foreground shader (vertex-based) ────────────────────────────────────────
const SHADER_FG = `
struct V { @location(0) pos: vec2<f32>, @location(1) col: vec3<f32>, @location(2) alpha: f32 }
struct F { @builtin(position) pos: vec4<f32>, @location(0) col: vec3<f32>, @location(1) alpha: f32 }
@vertex fn vs(v: V) -> F {
    var f: F; f.pos = vec4<f32>(v.pos, 0.0, 1.0); f.col = v.col; f.alpha = v.alpha; return f;
}
@fragment fn fs(f: F) -> @location(0) vec4<f32> { return vec4<f32>(f.col, f.alpha); }
`;

// ─── textured quad shader (for image() builtin) ───────────────────────────
const SHADER_IMG = `
struct V { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32> }
struct F { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(v: V) -> F {
    var f: F; f.pos = vec4<f32>(v.pos, 0.0, 1.0); f.uv = v.uv; return f;
}
@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;
@fragment fn fs(f: F) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, f.uv);
}
`;

// ─── input state ─────────────────────────────────────────────────────────────
let keys = 0;
const mouse = { x: 400, y: 300, btn: 0 };

function _isInputFocused() {
    const t = document.activeElement?.tagName;
    return t === 'INPUT' || t === 'TEXTAREA';
}

window.addEventListener('keydown', e => {
    if (_isInputFocused()) return;
    if (e.key === 'ArrowLeft'  || e.key === 'a') keys |= 1;
    if (e.key === 'ArrowRight' || e.key === 'd') keys |= 2;
    if (e.key === 'ArrowUp'    || e.key === 'w') keys |= 4;
    if (e.key === 'ArrowDown'  || e.key === 's') keys |= 8;
    if (e.key === ' ')                            keys |= 16;
    if (e.key === 'Control')                      keys |= 32;
    if (e.key === 'Shift')                        keys |= 64;
    if (e.key === 'Alt')                          keys |= 128;
    // prevent arrow keys and space from scrolling the page
    if ([' ','ArrowUp','ArrowDown','ArrowLeft','ArrowRight'].includes(e.key)) e.preventDefault();
}, { passive: false });

window.addEventListener('keyup', e => {
    if (_isInputFocused()) return;
    if (e.key === 'ArrowLeft'  || e.key === 'a') keys &= ~1;
    if (e.key === 'ArrowRight' || e.key === 'd') keys &= ~2;
    if (e.key === 'ArrowUp'    || e.key === 'w') keys &= ~4;
    if (e.key === 'ArrowDown'  || e.key === 's') keys &= ~8;
    if (e.key === ' ')                            keys &= ~16;
    if (e.key === 'Control')                      keys &= ~32;
    if (e.key === 'Shift')                        keys &= ~64;
    if (e.key === 'Alt')                          keys &= ~128;
});

window.addEventListener('mousemove', e => {
    const r = canvas.getBoundingClientRect();
    mouse.x = (e.clientX - r.left) / r.width  * 800;
    mouse.y = (e.clientY - r.top)  / r.height * 600;
});
window.addEventListener('mousedown', e => { mouse.btn |= (1 << e.button); ctxHide(); });
window.addEventListener('mouseup',     e => { mouse.btn &= ~(1 << e.button); });

// ─── context menu ─────────────────────────────────────────────────────────────
const ctxMenu = document.createElement('div');
ctxMenu.style.cssText = [
    'position:fixed','display:none','z-index:9000',
    'background:rgba(20,20,40,0.97)',
    'border:1px solid rgba(100,150,255,0.25)',
    'border-radius:6px','padding:4px 0',
    'min-width:140px',
    'box-shadow:0 4px 24px rgba(0,0,0,0.6)',
    'font:13px/1 monospace','color:#cde',
].join(';');
document.body.appendChild(ctxMenu);

function ctxHide() {
    ctxMenu.style.display = 'none';
    ctxMenu.innerHTML = '';
}

function ctxShow(x, y, items) {
    ctxMenu.innerHTML = '';
    items.forEach((label, i) => {
        const item = document.createElement('div');
        item.textContent = label;
        item.style.cssText = 'padding:7px 14px;cursor:pointer;white-space:nowrap;';
        item.addEventListener('mouseenter', () => item.style.background = 'rgba(80,140,255,0.18)');
        item.addEventListener('mouseleave', () => item.style.background = '');
        item.addEventListener('mousedown', ev => {
            ev.stopPropagation();
            on_contextmenu_select(i);
            ctxHide();
        });
        ctxMenu.appendChild(item);
    });
    ctxMenu.style.display = 'block';
    // keep menu within viewport
    const mw = ctxMenu.offsetWidth, mh = ctxMenu.offsetHeight;
    ctxMenu.style.left = Math.min(x, window.innerWidth  - mw - 4) + 'px';
    ctxMenu.style.top  = Math.min(y, window.innerHeight - mh - 4) + 'px';
}

let _ctxItems = [];
window.addEventListener('contextmenu', e => {
    e.preventDefault();
    if (_ctxItems.length > 0) {
        ctxShow(e.clientX, e.clientY, _ctxItems);
    }
});

// ─── resize (16:9 letterbox below 48px header) ────────────────────────────────
function resize(device, fmt) {
    dpr = window.devicePixelRatio || 1;
    const HEADER = 48;
    const avW = window.innerWidth;
    const avH = window.innerHeight - HEADER;
    // fit 16:9 inside available area
    let cssW, cssH;
    if (avW / avH > 16 / 9) {
        cssH = avH;
        cssW = Math.round(avH * 16 / 9);
    } else {
        cssW = avW;
        cssH = Math.round(avW * 9 / 16);
    }
    const pw = Math.round(cssW * dpr);
    const ph = Math.round(cssH * dpr);
    canvas.width = pw; canvas.height = ph;
    canvas.style.width  = cssW + 'px';
    canvas.style.height = cssH + 'px';
    on_resize(pw, ph, dpr);
    return device.createTexture({ size: [pw, ph], sampleCount: 4, format: fmt,
        usage: GPUTextureUsage.RENDER_ATTACHMENT });
}

async function main() {
    _step('WASM initialised');
    if (!('gpu' in navigator)) throw new Error('WebGPU not available — Chrome/Edge 113+ required.');
    _step('requesting WebGPU adapter...');
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) throw new Error('No WebGPU adapter found.');
    const device = await adapter.requestDevice();
    _step('WebGPU ready');
    device.addEventListener('uncapturederror', e => console.error('[WebGPU]', e.error.message));
    const ctx = canvas.getContext('webgpu');
    const fmt = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: fmt, alphaMode: 'opaque' });

    // ─── foreground pipeline ──────────────────────────────────────────────────
    const fgModule   = device.createShaderModule({ code: SHADER_FG });
    const fgPipeline = device.createRenderPipeline({
        layout: 'auto',
        vertex: { module: fgModule, entryPoint: 'vs',
            buffers: [{ arrayStride: 24, attributes: [
                { shaderLocation: 0, offset:  0, format: 'float32x2' },
                { shaderLocation: 1, offset:  8, format: 'float32x3' },
                { shaderLocation: 2, offset: 20, format: 'float32'   },
            ]}],
        },
        fragment: { module: fgModule, entryPoint: 'fs', targets: [{ format: fmt, blend: ALPHA_BLEND }]},
        primitive:   { topology: 'triangle-list' },
        multisample: { count: 4 },
    });

    // ─── image pipeline (textured quads) ─────────────────────────────────────
    const imgModule   = device.createShaderModule({ code: SHADER_IMG });
    const imgBGL      = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT,
          sampler: { type: 'filtering' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT,
          texture: { sampleType: 'float' } },
    ]});
    const imgPipeline = device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [imgBGL] }),
        vertex: { module: imgModule, entryPoint: 'vs',
            buffers: [{ arrayStride: 16, attributes: [
                { shaderLocation: 0, offset: 0, format: 'float32x2' }, // pos
                { shaderLocation: 1, offset: 8, format: 'float32x2' }, // uv
            ]}],
        },
        fragment: { module: imgModule, entryPoint: 'fs', targets: [{ format: fmt, blend: ALPHA_BLEND }]},
        primitive:   { topology: 'triangle-list' },
        multisample: { count: 4 },
    });
    const imgSampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear' });

    // texture cache: url → { tex: GPUTexture, w: number, h: number } | "loading" | "error"
    const texCache = new Map();
    let imgVbufs = []; // destroyed at the top of each frame

    // Resolve local paths to /images/ route; leave URLs untouched
    function resolveUrl(url) {
        if (/^https?:\/\//i.test(url)) return url;
        return '/images/' + url;
    }

    // Upload an ImageBitmap to a new WebGPU texture and store in cache.
    function uploadBitmap(url, bmp) {
        const tex = device.createTexture({
            size: [bmp.width, bmp.height, 1],
            format: 'rgba8unorm',
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,
        });
        device.queue.copyExternalImageToTexture(
            { source: bmp }, { texture: tex }, [bmp.width, bmp.height],
        );
        texCache.set(url, { tex, w: bmp.width, h: bmp.height });
    }

    // Load a URL into the GPU texture cache (async, non-blocking).
    // hintW/hintH used as rasterisation size for SVGs (first call wins).
    function ensureTexture(url, hintW, hintH) {
        if (texCache.has(url)) return;
        texCache.set(url, 'loading');
        if (/\.svg$/i.test(url)) {
            // SVGs: rasterise at DPR-scaled size for crisp rendering on all screens
            const w = (hintW > 0 ? Math.round(hintW * dpr) : 256) | 0;
            const h = (hintH > 0 ? Math.round(hintH * dpr) : 256) | 0;
            const img = new Image(w, h);
            img.onload = () => {
                const oc = new OffscreenCanvas(w, h);
                oc.getContext('2d').drawImage(img, 0, 0, w, h);
                createImageBitmap(oc)
                    .then(bmp => uploadBitmap(url, bmp))
                    .catch(e => { console.warn('[bruecke] svg upload failed:', url, e); texCache.set(url, 'error'); });
            };
            img.onerror = e => { console.warn('[bruecke] svg load failed:', url, e); texCache.set(url, 'error'); };
            img.src = resolveUrl(url);
        } else {
            fetch(resolveUrl(url))
                .then(r => { if (!r.ok) throw new Error(r.status); return r.blob(); })
                .then(b => createImageBitmap(b))
                .then(bmp => uploadBitmap(url, bmp))
                .catch(e => { console.warn('[bruecke] image load failed:', url, e); texCache.set(url, 'error'); });
        }
    }

    // Build 6 vertices (2 triangles) for a rotated textured quad.
    // x,y,w,h are in scene coords (0-800, 0-600). angle in degrees.
    function buildImgVerts(x, y, w, h, angleDeg, u0, v0, u1, v1) {
        const cx = x + w / 2, cy = y + h / 2;
        const hw = w / 2, hh = h / 2;
        const a = angleDeg * Math.PI / 180;
        const cos = Math.cos(a), sin = Math.sin(a);
        // 4 corners relative to center, rotated, converted to NDC
        const corners = [
            [-hw, -hh, u0, v0], // TL
            [ hw, -hh, u1, v0], // TR
            [-hw,  hh, u0, v1], // BL
            [ hw,  hh, u1, v1], // BR
        ].map(([dx, dy, u, v]) => [
            (cx + dx * cos - dy * sin) / 400 - 1, // NDC x
            1 - (cy + dx * sin + dy * cos) / 300,  // NDC y
            u, v,
        ]);
        // 2 triangles: TL,TR,BL  and  TR,BR,BL
        const [tl, tr, bl, br] = corners;
        return new Float32Array([
            ...tl, ...tr, ...bl,
            ...tr, ...br, ...bl,
        ]);
    }

    let msaaTex = resize(device, fmt);
    window.addEventListener('resize', () => { msaaTex = resize(device, fmt); });
    function watchDpr() {
        const mql = window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
        mql.addEventListener('change', () => { msaaTex = resize(device, fmt); watchDpr(); }, { once: true });
    }
    watchDpr();

    // ─── performance stats ────────────────────────────────────────────────────
    const perf = { fps: 0, frame_ms: 0, wasm_ms: 0, buffer_bytes: 0,
                   frame_count: 0, canvas_w: 0, canvas_h: 0, dpr: 1, _ring: [] };

    // ─── perf HUD overlay (bottom-left) ───────────────────────────────────
    const hud = document.createElement('div');
    hud.style.cssText = [
        'position:fixed','bottom:52px','left:10px',
        'font:11px/1.7 monospace','color:#4af','opacity:0.75',
        'background:rgba(0,0,10,0.55)','padding:4px 8px',
        'border-radius:4px','pointer-events:none','z-index:400',
        'white-space:pre','letter-spacing:.03em',
    ].join(';');
    document.body.appendChild(hud);

    function updateHud() {
        const verts = Math.round(perf.buffer_bytes / 24);
        hud.textContent =
            `fps   ${String(perf.fps).padStart(3)}\n` +
            `frame ${perf.frame_ms.toFixed(1).padStart(5)} ms\n` +
            `wasm  ${perf.wasm_ms.toFixed(1).padStart(5)} ms\n` +
            `verts ${String(verts).padStart(5)}`;
    }

    // ─── keyboard HUD (bottom-right) ──────────────────────────────────────────
    // Layout:  [Ctrl] [Shft] [Alt ]
    //                  [ ↑  ]
    //          [ ←  ]  [ ↓  ]  [ →  ]   [      Space      ]
    const kbdHud = document.createElement('div');
    kbdHud.style.cssText = [
        'position:fixed','bottom:12px','right:12px',
        'display:grid',
        'grid-template-columns:repeat(7,32px)',
        'grid-template-rows:repeat(3,26px)',
        'gap:3px',
        'pointer-events:none','z-index:400',
    ].join(';');
    document.body.appendChild(kbdHud);

    // bit → { label, col (1-based), row (1-based), colSpan }
    const KEY_DEFS = [
        { bit: 32,  label: 'Ctrl',  col: 1, row: 1, span: 1 },
        { bit: 64,  label: 'Shft',  col: 2, row: 1, span: 1 },
        { bit: 128, label: 'Alt',   col: 3, row: 1, span: 1 },
        { bit: 4,   label: '↑',     col: 2, row: 2, span: 1 },
        { bit: 1,   label: '←',     col: 1, row: 3, span: 1 },
        { bit: 8,   label: '↓',     col: 2, row: 3, span: 1 },
        { bit: 2,   label: '→',     col: 3, row: 3, span: 1 },
        { bit: 16,  label: 'Space', col: 4, row: 3, span: 4 },
    ];

    const keyEls = KEY_DEFS.map(k => {
        const el = document.createElement('div');
        el.textContent = k.label;
        el.style.cssText = [
            `grid-column:${k.col}/span ${k.span}`,
            `grid-row:${k.row}`,
            'display:flex','align-items:center','justify-content:center',
            'border-radius:5px',
            'font:bold 10px/1 monospace',
            'letter-spacing:.02em',
            'transition:background .06s,color .06s,box-shadow .06s',
            'user-select:none',
        ].join(';');
        kbdHud.appendChild(el);
        return { el, bit: k.bit };
    });

    let _prevKeys = -1;
    function updateKbd() {
        if (keys === _prevKeys) return;
        _prevKeys = keys;
        for (const { el, bit } of keyEls) {
            const on = (keys & bit) !== 0;
            if (on) {
                el.style.background  = 'rgba(80,180,255,0.85)';
                el.style.color       = '#fff';
                el.style.boxShadow   = '0 0 8px rgba(80,180,255,0.7)';
                el.style.border      = '1px solid rgba(120,210,255,0.9)';
            } else {
                el.style.background  = 'rgba(20,22,40,0.72)';
                el.style.color       = 'rgba(160,185,220,0.7)';
                el.style.boxShadow   = 'none';
                el.style.border      = '1px solid rgba(80,100,140,0.35)';
            }
        }
    }
    updateKbd();

    // ─── render loop ──────────────────────────────────────────────────────────
    let vbuf = null, lastTs = -Infinity;
    function raf(ts) {
        requestAnimationFrame(raf);
        if (ts - lastTs < 1000 / fps_target() - 0.5) return;
        const t0 = performance.now();
        lastTs = ts;

        const tw = performance.now();
        let verts;
        try {
            verts = frame(ts, keys, mouse.x, mouse.y, mouse.btn);
        } catch (e) {
            showError('WASM panic: ' + String(e));
            return;
        }
        perf.wasm_ms = +(performance.now() - tw).toFixed(2);

        // update error overlay — never let a WASM error kill the loop
        try {
            const r = JSON.parse(get_result());
            if (r.ok) { clearError(); }
            else if (r.error) { showError(r.error); }
        } catch (_) { /* JSON parse error — ignore */ }

        // sync context-menu items from Python
        const rawItems = get_contextmenu_items();
        _ctxItems = rawItems ? rawItems.split('|') : [];

        // toggle SVG cursor overlay: hide when Python has custom draw fn
        _cursorEl.style.display = get_cursor_custom() ? 'none' : '';

        perf.frame_count++;
        perf._ring.push(ts);
        if (perf._ring.length > 60) perf._ring.shift();
        if (perf._ring.length >= 2) {
            const dt = perf._ring[perf._ring.length - 1] - perf._ring[0];
            perf.fps = Math.round((perf._ring.length - 1) / dt * 1000);
        }
        perf.buffer_bytes = verts ? verts.byteLength : 0;
        perf.canvas_w = canvas.width; perf.canvas_h = canvas.height; perf.dpr = dpr;

        if (vbuf) vbuf.destroy();
        for (const b of imgVbufs) b.destroy();
        imgVbufs = [];
        if (verts && verts.length > 0) {
            vbuf = device.createBuffer({ size: verts.byteLength,
                usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
            device.queue.writeBuffer(vbuf, 0, verts);
        } else {
            vbuf = null;
        }

        const encoder = device.createCommandEncoder();
        const pass    = encoder.beginRenderPass({ colorAttachments: [{
            view: msaaTex.createView(), resolveTarget: ctx.getCurrentTexture().createView(),
            clearValue: { r: 0, g: 0, b: 0, a: 1.0 }, loadOp: 'clear', storeOp: 'discard',
        }]});

        // foreground (Python-generated)
        if (vbuf) {
            pass.setPipeline(fgPipeline);
            pass.setVertexBuffer(0, vbuf);
            pass.draw(verts.length / 6);
        }

        // ── image pipeline ────────────────────────────────────────────────────
        let imgCmds;
        try { imgCmds = JSON.parse(get_image_cmds()); } catch(_) { imgCmds = []; }
        for (const cmd of imgCmds) {
            // negative w = horizontal flip; use abs for texture sizing hint
            const flipX = cmd.w < 0;
            ensureTexture(cmd.url, Math.abs(cmd.w), Math.abs(cmd.h));
            const entry = texCache.get(cmd.url);
            if (!entry || typeof entry !== 'object') continue; // loading or error

            // UV from sprite sheet: sw/sh = -1 means full image
            const iw = entry.w, ih = entry.h;
            const sw = cmd.sw < 0 ? iw : cmd.sw;
            const sh = cmd.sh < 0 ? ih : cmd.sh;
            let u0 = cmd.sx / iw, v0 = cmd.sy / ih;
            let u1 = (cmd.sx + sw) / iw, v1 = (cmd.sy + sh) / ih;
            // swap U coords for horizontal flip
            if (flipX) { const tmp = u0; u0 = u1; u1 = tmp; }

            const imgVerts = buildImgVerts(cmd.x, cmd.y, Math.abs(cmd.w), cmd.h, cmd.angle, u0, v0, u1, v1);
            const vbufImg = device.createBuffer({
                size: imgVerts.byteLength,
                usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
            });
            imgVbufs.push(vbufImg);
            device.queue.writeBuffer(vbufImg, 0, imgVerts);

            const bg = device.createBindGroup({
                layout: imgBGL,
                entries: [
                    { binding: 0, resource: imgSampler },
                    { binding: 1, resource: entry.tex.createView() },
                ],
            });

            pass.setPipeline(imgPipeline);
            pass.setBindGroup(0, bg);
            pass.setVertexBuffer(0, vbufImg);
            pass.draw(6);
        }

        pass.end();
        device.queue.submit([encoder.finish()]);
        perf.frame_ms = +(performance.now() - t0).toFixed(2);
        updateHud();
        updateKbd();
    }
    _step('render loop started');
    requestAnimationFrame(raf);

    // ─── public API ───────────────────────────────────────────────────────────
    window.bruecke = {
        run(source) { on_run(source); },
    };

    // SSE — auto-connect when running on localhost
    if (location.hostname === 'localhost' || location.hostname === '127.0.0.1') {
        _step('connecting SSE...');
        const es = new EventSource('/events');
        es.onmessage = async e => {
            const bytes = e.data.length;
            _step(`Python source received (${bytes} bytes)`);
            try {
                window.bruecke.run(e.data);
            } catch (err) {
                console.error('[bruecke] on_run error:', err);
            }
            let compileResult;
            try { compileResult = JSON.parse(get_result()); } catch(_) {}
            if (compileResult) {
                if (compileResult.ok) {
                    _step(`Python compile OK — ${compileResult.source_lines} lines, ${compileResult.compile_ms?.toFixed(1)}ms`, true);
                } else {
                    _step(`Python compile ERROR: ${compileResult.error}`);
                    console.error('[bruecke] compile error:', compileResult.error);
                }
                try {
                    await fetch('/result', {
                        method: 'POST',
                        body: JSON.stringify({
                            ...compileResult,
                            fps:          perf.fps,
                            frame_ms:     perf.frame_ms,
                            wasm_ms:      perf.wasm_ms,
                            buffer_bytes: perf.buffer_bytes,
                        }),
                    });
                } catch (_) {}
            }
        };
        es.onerror = () => {
            console.warn('[bruecke] SSE connection lost — will retry');
        };
    }
}

// main() is called by the server after WASM init — do not call it here

// ─── AI settings + prompt bar ─────────────────────────────────────────────────
(function () {
    const input    = document.getElementById('ai-input');
    const btn      = document.getElementById('ai-btn');
    const buildCbx = document.getElementById('ai-build');
    const status   = document.getElementById('ai-status');
    const label    = document.getElementById('ai-label');
    const keyBtn   = document.getElementById('ai-key-btn');
    const panel    = document.getElementById('asp');
    const closeBtn = document.getElementById('asp-close');
    const saveBtn  = document.getElementById('asp-save');
    const savedEl  = document.getElementById('asp-saved');
    const tabs     = document.querySelectorAll('.asp-tab');
    const sections = document.querySelectorAll('.asp-section');
    const histBtn   = document.getElementById('ai-hist-btn');
    const histPanel = document.getElementById('ahp');
    const histClose = document.getElementById('ahp-close');
    const histList  = document.getElementById('ahp-list');
    const histEmpty = document.getElementById('ahp-empty');
    if (!input || !btn || !panel) return;

    // ── localStorage helpers ──────────────────────────────────────────────────
    const LS = {
        get: k      => localStorage.getItem('bruecke.' + k) || '',
        set: (k, v) => localStorage.setItem('bruecke.' + k, v),
    };

    // ── provider label ────────────────────────────────────────────────────────
    const NAMES = { anthropic: 'Claude', openai: 'GPT', gemini: 'Gemini' };
    function refreshLabel() {
        const p = LS.get('provider') || 'anthropic';
        if (label) label.textContent = NAMES[p] || 'AI';
    }
    refreshLabel();

    // ── restore saved values ──────────────────────────────────────────────────
    ['anthropic', 'openai', 'gemini'].forEach(p => {
        const keyEl   = document.getElementById('key-'   + p);
        const modelEl = document.getElementById('model-' + p);
        if (keyEl)   keyEl.value   = LS.get('key.'   + p);
        if (modelEl && LS.get('model.' + p)) modelEl.value = LS.get('model.' + p);
    });

    // ── tab switching ─────────────────────────────────────────────────────────
    function switchTab(p) {
        tabs.forEach(t => t.classList.toggle('active', t.dataset.p === p));
        sections.forEach(s => s.classList.toggle('active', s.dataset.p === p));
    }
    tabs.forEach(t => t.addEventListener('click', () => switchTab(t.dataset.p)));

    // ── open / close settings panel ───────────────────────────────────────────
    function openPanel() {
        closeHistPanel();
        switchTab(LS.get('provider') || 'anthropic');
        panel.classList.add('open');
        keyBtn.classList.add('active');
    }
    function closePanel() {
        panel.classList.remove('open');
        keyBtn.classList.remove('active');
    }
    keyBtn.addEventListener('click', e => {
        e.stopPropagation();
        panel.classList.contains('open') ? closePanel() : openPanel();
    });
    closeBtn.addEventListener('click', closePanel);
    document.addEventListener('click', e => {
        if (!panel.contains(e.target) && e.target !== keyBtn) closePanel();
        if (histPanel && !histPanel.contains(e.target) && e.target !== histBtn) closeHistPanel();
    });

    // ── 👁 show / hide key ────────────────────────────────────────────────────
    document.querySelectorAll('.asp-eye').forEach(eye => {
        eye.addEventListener('click', () => {
            const inp = document.getElementById(eye.dataset.for);
            if (inp) inp.type = inp.type === 'password' ? 'text' : 'password';
        });
    });

    // ── save ──────────────────────────────────────────────────────────────────
    saveBtn.addEventListener('click', () => {
        const activeTab = [...tabs].find(t => t.classList.contains('active'));
        if (activeTab) LS.set('provider', activeTab.dataset.p);
        ['anthropic', 'openai', 'gemini'].forEach(p => {
            const k = document.getElementById('key-'   + p)?.value.trim();
            const m = document.getElementById('model-' + p)?.value;
            if (k) LS.set('key.'   + p, k);
            if (m) LS.set('model.' + p, m);
        });
        refreshLabel();
        savedEl.classList.add('show');
        setTimeout(() => { savedEl.classList.remove('show'); closePanel(); }, 900);
    });

    // ── history panel ─────────────────────────────────────────────────────────
    function fmtTime(ts) {
        const d = new Date(ts);
        const now = Date.now();
        const diff = now - ts;
        if (diff < 60000)   return 'just now';
        if (diff < 3600000) return Math.floor(diff / 60000) + 'm ago';
        if (diff < 86400000)return Math.floor(diff / 3600000) + 'h ago';
        return d.toLocaleDateString([], { month: 'short', day: 'numeric' })
             + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    async function loadHistory() {
        try {
            const r = await fetch('/history');
            const entries = await r.json();
            if (!entries.length) {
                if (histEmpty) histEmpty.style.display = '';
                return;
            }
            if (histEmpty) histEmpty.style.display = 'none';
            histList.innerHTML = '';
            for (const e of entries) {
                const el = document.createElement('div');
                el.className = 'ahp-entry';
                el.innerHTML = `
                  <div class="ahp-entry-head">
                    <span class="ahp-time">${fmtTime(e.ts)}</span>
                    <span class="ahp-lines">${e.lines} lines</span>
                  </div>
                  <div class="ahp-preview">${e.preview.replace(/</g,'&lt;')}</div>
                  <div class="ahp-restore">↩ restore</div>`;
                el.addEventListener('click', async () => {
                    try {
                        const src = await (await fetch('/history/' + e.id)).text();
                        await fetch('/run', { method: 'POST', body: src });
                        closeHistPanel();
                        status.textContent = 'restored';
                        status.style.color = '#206040';
                        setTimeout(() => { status.textContent = ''; }, 2000);
                    } catch (err) {
                        showError('restore failed: ' + err);
                    }
                });
                histList.appendChild(el);
            }
        } catch (_) {}
    }

    function openHistPanel() {
        closePanel();
        histPanel.classList.add('open');
        histBtn.classList.add('active');
        loadHistory();
    }
    function closeHistPanel() {
        histPanel.classList.remove('open');
        histBtn.classList.remove('active');
    }
    histBtn.addEventListener('click', e => {
        e.stopPropagation();
        histPanel.classList.contains('open') ? closeHistPanel() : openHistPanel();
    });
    histClose.addEventListener('click', closeHistPanel);

    // ── generate ──────────────────────────────────────────────────────────────
    async function generate() {
        const text = input.value.trim();
        if (!text || btn.disabled) return;

        btn.disabled       = true;
        btn.textContent    = 'thinking...';
        status.textContent = '';
        status.style.color = '#1e2e5e';
        clearError();

        const provider  = LS.get('provider') || 'anthropic';
        const model     = LS.get('model.' + provider) || undefined;
        const api_key   = LS.get('key.'   + provider) || undefined;
        const build_on  = buildCbx.checked;

        const t0 = performance.now();
        try {
            const res = await fetch('/prompt', {
                method:  'POST',
                headers: { 'Content-Type': 'application/json' },
                body:    JSON.stringify({ prompt: text, provider, model, api_key, build_on }),
            });
            const msg     = await res.text();
            const elapsed = ((performance.now() - t0) / 1000).toFixed(1) + 's';
            if (!res.ok) {
                status.textContent = 'error';
                status.style.color = '#802020';
                showError('AI (' + provider + '): ' + msg);
            } else {
                input.value        = '';
                status.textContent = elapsed;
                status.style.color = '#206040';
            }
        } catch (e) {
            status.textContent = 'error';
            status.style.color = '#802020';
            showError('AI fetch error: ' + String(e));
        } finally {
            btn.disabled    = false;
            btn.textContent = 'Generate';
        }
    }

    btn.addEventListener('click', generate);
    input.addEventListener('keydown', e => { if (e.key === 'Enter') generate(); });
})();
