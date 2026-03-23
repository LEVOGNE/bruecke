# bruecke — Python API Reference

Scene: **800 × 600 px**, origin top-left, X→ right, Y↓ down.
Everything below is available inside `frame(t)` without any import.

---

## Structure

```python
# module-level code runs once — use for state, classes, lists
balls = []
speed = 3.5

def frame(t):
    global balls, speed   # needed to write module-level vars
    # called ~60× per second
```

---

## Draw Builtins *(no import needed)*

### Color & Opacity
| Call | Description |
|------|-------------|
| `color(r, g, b)` | Set fill color. Each channel 0–255. |
| `alpha(a)` | Set opacity. 0.0 = invisible, 1.0 = fully opaque. Default: 1.0 |

### Shapes
| Call | Description |
|------|-------------|
| `rect(x, y, w, h)` | Filled rectangle. x/y = top-left corner. |
| `circle(x, y, r)` | Filled circle. x/y = center, r = radius. |
| `line(x1, y1, x2, y2)` | Line, 3 px wide. |

### Images & Sprites
```python
image(url, x, y, w, h)                          # image or https:// URL
image(url, x, y, w, h, sx, sy, sw, sh)          # sprite sheet sub-region
image(url, x, y, w, h, sx, sy, sw, sh, angle)   # + clockwise rotation in degrees
```
- `w < 0` → flip horizontally (mirror)
- SVG files are rasterised at full resolution automatically
- Local files are served from CWD (next to `app.py`)

### Transform
| Call | Description |
|------|-------------|
| `translate(dx, dy)` | Shift origin for subsequent draw calls. |
| `scale(s)` | Scale factor for subsequent draw calls. |

---

## Math Builtins *(no import needed)*

| Call | Description |
|------|-------------|
| `lerp(a, b, t)` | Linear interpolation. t=0 → a, t=1 → b. |
| `clamp(x, lo, hi)` | Clamp x between lo and hi. |
| `sign(x)` | Returns -1, 0, or 1. |
| `abs(x)` | Absolute value. |
| `min(a, b)` / `max(a, b)` | Min / max. |
| `round(x)` | Round to nearest integer. |

---

## Per-Frame Inputs *(global variables, updated each frame)*

| Variable | Type | Description |
|----------|------|-------------|
| `t` | float | Seconds since start. |
| `mouse_x` | float | Cursor X position, 0–800. |
| `mouse_y` | float | Cursor Y position, 0–600. |
| `mouse_btn` | int | Bitmask. `1` = left button held. |
| `keys` | int | Bitmask (see table below). |

### `keys` Bitmask
| Bit | Value | Key |
|-----|-------|-----|
| 0 | `1` | ← Left Arrow |
| 1 | `2` | → Right Arrow |
| 2 | `4` | ↑ Up Arrow |
| 3 | `8` | ↓ Down Arrow |
| 4 | `16` | Space |
| 5 | `32` | Ctrl |
| 6 | `64` | Shift |
| 7 | `128` | Alt |

```python
left  = bool(keys & 1)
right = bool(keys & 2)
jump  = bool(keys & 16)
```

---

## Scene Constants *(no import needed)*

| Constant | Value |
|----------|-------|
| — | Scene is always 800 × 600 px. Use literals or define your own. |

---

## Standard Imports

### `import math`
```python
import math
math.sin(x)   math.cos(x)   math.tan(x)
math.sqrt(x)  math.floor(x) math.ceil(x)
math.atan2(y, x)            math.hypot(x, y)
math.radians(deg)           math.degrees(rad)
math.log(x)   math.log2(x)  math.log10(x)
math.exp(x)   math.pow(x,y)
math.pi       math.e        math.tau      math.inf
math.isfinite(x)  math.isinf(x)  math.isnan(x)
math.copysign(x, y)         math.fabs(x)
```

### `import random`
```python
import random
random.random()              # float 0.0–1.0
random.randint(a, b)         # int inclusive a..b
random.choice(seq)           # random element
random.shuffle(seq)          # in-place shuffle
random.uniform(a, b)         # float a..b
random.sample(population, k) # k unique elements
```

---

## `import bruecke`

The full draw + input API is also accessible via the `bruecke` module:

```python
import bruecke
bruecke.circle(400, 300, 50)

# or unpack everything:
from bruecke import *
```

### `bruecke` namespace
```python
bruecke.W          # 800 (scene width)
bruecke.H          # 600 (scene height)
bruecke.color      # → color()
bruecke.alpha      # → alpha()
bruecke.rect       # → rect()
bruecke.circle     # → circle()
bruecke.line       # → line()
bruecke.image      # → image()
bruecke.translate  # → translate()
bruecke.scale      # → scale()
bruecke.lerp       # → lerp()
bruecke.clamp      # → clamp()
bruecke.sign       # → sign()
bruecke.t          # current time
bruecke.mouse_x    # cursor X
bruecke.mouse_y    # cursor Y
bruecke.mouse_btn  # button bitmask
bruecke.keys       # key bitmask
```

---

## `bruecke.cursor` — Custom Cursor

```python
import bruecke

# Properties (set any time, take effect next frame)
bruecke.cursor.visible = True         # show/hide cursor overlay
bruecke.cursor.color   = (255, 220, 80)  # RGB color hint
bruecke.cursor.size    = 16           # size hint

# Custom draw function — replaces the default SVG cursor
def my_cursor():
    color(255, 80, 80)
    circle(mouse_x, mouse_y, 8)
    alpha(0.4)
    circle(mouse_x, mouse_y, 16)

bruecke.cursor.draw = my_cursor   # engine calls this after frame()
bruecke.cursor.draw = None        # restore default SVG cursor
```

When `draw` is set to a function, the SVG overlay is hidden and your function is called automatically after every `frame()`.

---

## `bruecke.contextmenu` — Right-Click Menu

```python
import bruecke

# Add items: (label, callback)
bruecke.contextmenu._items = [
    ("Reset",    lambda: reset()),
    ("Explode",  lambda: explode()),
    ("Separator", None),   # None callback = disabled / visual separator
]
```

- Items are shown on right-click in the browser.
- The callback fires in the frame after the user clicks.
- Update `_items` any time (even inside `frame(t)`).

---

## `import log` — FPS Monitor & Input Overlay

```python
import log

def frame(t):
    log.tick(t)     # call once per frame (required for FPS tracking)
    log.draw()      # optional: draw FPS bar + keyboard overlay
```

### API
| Call / Property | Description |
|-----------------|-------------|
| `log.tick(t)` | Update FPS, dt, click/key tracking. Call once per frame. |
| `log.draw()` | Draw FPS bar (top-right) + keyboard HUD (bottom-right). |
| `log.fps` | Current FPS as float. |
| `log.dt` | Last frame duration in seconds. |
| `log.dblclick` | `True` in the frame a double-click is detected. |
| `log.info(key, value)` | Store arbitrary debug values. |
| `log.get(key)` | Read stored value. `None` if not set. |

---

## Tips

```python
# Detect key press edge (only first frame the key is held)
prev_space = False
def frame(t):
    global prev_space
    space = bool(keys & 16)
    space_down = space and not prev_space
    prev_space = space
    if space_down:
        jump()

# Map mouse to scene coordinates (already in scene space)
x, y = mouse_x, mouse_y

# Flip a sprite horizontally
image("hero.svg", x - w/2, y - h, -w, h)   # negative w = mirror

# Colour from hue (manual HSV)
import math
hue = (t * 60) % 360
r = clamp(int(abs(hue - 180) - 60) * 4, 0, 255)
```
