# log.py — bruecke performance & input monitor
#
# Dieses File wird vom bruecke-Server automatisch gebundelt, wenn du
#   import log
# in deinem Skript schreibst. Es läuft im selben globalen Scope wie app.py,
# sodass die Frame-Globals (keys, mouse_x, mouse_y, t, mouse_btn) direkt
# verfügbar sind.
#
# API:
#   log.tick(t)           — einmal pro Frame aufrufen (fps, dt berechnen)
#   log.draw()            — fps-Bar + Tastatur-Overlay zeichnen
#   log.fps               — aktuelle FPS (float)
#   log.dt                — letzte Framedauer in ms
#   log.dblclick          — True im Frame eines Doppelklicks
#   log.info(key, value)  — beliebige Werte tracken
#   log.get(key)          — gespeicherten Wert lesen
#
# keys-Bitmask:  1=links  2=rechts  4=oben  8=unten  16=Space
#                32=Ctrl  64=Shift  128=Alt

import sys as _sys

class _Log:
    _ts            = []
    _vals          = {}
    fps            = 0.0
    dt             = 0.0
    _t             = 0.0
    _prev_btn      = 0
    _prev_keys     = 0
    dblclick       = False
    _dbl_flash     = 0
    _last_click_t  = -9999.0
    _key_down_t    = {}
    _last_key_t    = {}
    _dbl_key_flash = {}

    @classmethod
    def tick(cls, t):
        if len(cls._ts) > 0:
            cls.dt = t - cls._ts[-1]
        cls._ts.append(t)
        if len(cls._ts) > 120:
            cls._ts.pop(0)
        if len(cls._ts) >= 2:
            span = cls._ts[-1] - cls._ts[0]
            cls.fps = round((len(cls._ts) - 1) / span) if span > 0 else 0
        cls._t = t

        # Doppelklick
        btn  = mouse_btn
        prev = cls._prev_btn
        if bool(btn & 1) and not bool(prev & 1):
            cls.dblclick = (t - cls._last_click_t) < 300
            if cls.dblclick:
                cls._dbl_flash = 20
            cls._last_click_t = t
        else:
            cls.dblclick = False
        if cls._dbl_flash > 0:
            cls._dbl_flash -= 1
        cls._prev_btn = btn

        # Taste gedrückt / Doppeltastendruck
        k      = keys
        prev_k = cls._prev_keys
        for bit in (1, 2, 4, 8, 16, 32, 64, 128):
            now = bool(k & bit)
            was = bool(prev_k & bit)
            if now and not was:
                last = cls._last_key_t.get(bit, -9999.0)
                if (t - last) < 300:
                    cls._dbl_key_flash[bit] = 20
                cls._last_key_t[bit] = t
                cls._key_down_t[bit] = t
            elif not now and was:
                cls._key_down_t.pop(bit, None)
            if cls._dbl_key_flash.get(bit, 0) > 0:
                cls._dbl_key_flash[bit] -= 1
        cls._prev_keys = k

    @classmethod
    def info(cls, key, value):
        cls._vals[key] = value

    @classmethod
    def get(cls, key, default=None):
        return cls._vals.get(key, default)

    @classmethod
    def draw(cls):
        # ── fps-Bar oben rechts ───────────────────────────────────────────────
        bw, bh, bx, by = 80, 6, 800 - 88, 8
        alpha(0.5); color(0, 0, 0)
        rect(bx - 2, by - 2, bw + 4, bh + 4)
        frac = clamp(cls.fps / 60.0, 0.0, 1.0)
        color(int(lerp(220, 60, frac)), int(lerp(60, 220, frac)), 60)
        alpha(0.9); rect(bx, by, bw * frac, bh)

        # ── Tastatur-Overlay unten rechts ─────────────────────────────────────
        # Zeile 0: [Ctrl][Shft][Alt ]
        # Zeile 1:        [ Up ]
        # Zeile 2: [ Lt  ][ Dn ][ Rt ]    [    Space    ]
        s = 14; p = 3
        ox = 800 - 8 - (s * 4 + p * 3) - (s * 4 + p) - 8
        oy = 600 - 8 - s * 3 - p * 2

        def key_box(x, y, w, h, bit):
            pressed = bool(keys & bit)
            dbl     = cls._dbl_key_flash.get(bit, 0) > 0
            held_ms = (cls._t - cls._key_down_t[bit]) if (pressed and bit in cls._key_down_t) else 0
            if dbl:
                alpha(0.95); color(255, 200, 60)          # gold = Doppeldruck
            elif pressed:
                f = clamp(held_ms / 800.0, 0.0, 1.0)
                alpha(lerp(0.7, 1.0, f))
                color(int(lerp(80, 160, f)), int(lerp(200, 255, f)), 255)  # blau, heller je länger
            else:
                alpha(0.25); color(60, 60, 80)
            rect(x, y, w, h)
            if pressed:
                alpha(0.4); color(255, 255, 255)
                rect(x + 2, y + 2, w - 4, 3)

        key_box(ox,             oy,           s, s, 32)   # Ctrl
        key_box(ox + s + p,     oy,           s, s, 64)   # Shift
        key_box(ox + (s+p)*2,   oy,           s, s, 128)  # Alt

        key_box(ox + s + p,       oy + s + p,     s, s, 4)   # Hoch
        key_box(ox,               oy + (s+p)*2,   s, s, 1)   # Links
        key_box(ox + s + p,       oy + (s+p)*2,   s, s, 8)   # Runter
        key_box(ox + (s+p)*2,     oy + (s+p)*2,   s, s, 2)   # Rechts
        key_box(ox + (s+p)*3 + 8, oy + (s+p)*2,   s*4+p*3, s, 16)  # Space

        # ── Doppelklick-Ring ──────────────────────────────────────────────────
        if cls._dbl_flash > 0:
            f2 = cls._dbl_flash / 20.0
            alpha(f2 * 0.7); color(255, 200, 60)
            circle(mouse_x, mouse_y, int(lerp(32, 20, f2)))

        alpha(1.0)

_sys.modules['log'] = _Log
del _sys, _Log
