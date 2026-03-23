import math
import log

# ── Bälle ─────────────────────────────────────────────────────────────────────
balls = []
class Ball:
    def __init__(self, x, y, vx, vy, r, cr, cg, cb):
        self.x, self.y = x, y
        self.vx, self.vy = vx, vy
        self.r = r
        self.cr, self.cg, self.cb = cr, cg, cb

for i in range(12):
    ang = i * 6.28 / 12
    balls.append(Ball(
        200 + (i * 53) % 400,
        100 + (i * 37) % 400,
        math.cos(ang) * 2.5,
        math.sin(ang) * 2.5,
        14 + (i % 4) * 5,
        80 + (i * 40) % 175,
        120 + (i * 70) % 135,
        200 + (i * 30) % 55,
    ))

# ── Cat Hero ──────────────────────────────────────────────────────────────────
CAT_SPEED   = 3.5
CAT_JUMP    = -11.0
CAT_GRAVITY = 0.45
CAT_FLOOR   = 560      # Y wo der Boden ist (Füße der Katze)
CAT_W       = 90
CAT_H       = 90
CAT_R       = 30       # Kollisionsradius

cat_x       = 400.0
cat_y       = float(CAT_FLOOR)
cat_vx      = 0.0
cat_vy      = 0.0
cat_ground  = True
cat_right   = True     # schaut nach rechts?
cat_anim_t  = 0.0
prev_space  = False

# ── Drag/Slingshot-State ───────────────────────────────────────────────────────
dragged     = None
mouse_hist  = []
shoot_start = None
prev_btn    = 0
shot_count  = 0
prev_mx     = 0.0
prev_my     = 0.0

def frame(t):
    global balls, dragged, mouse_hist, shoot_start, prev_btn, shot_count
    global prev_mx, prev_my, prev_space
    global cat_x, cat_y, cat_vx, cat_vy, cat_ground, cat_right, cat_anim_t
    log.tick(t)

    btn_down = (mouse_btn & 1) and not (prev_btn & 1)
    btn_up   = not (mouse_btn & 1) and (prev_btn & 1)
    prev_btn = mouse_btn

    mouse_hist.append((mouse_x, mouse_y))
    if len(mouse_hist) > 8:
        mouse_hist.pop(0)

    # ── Hintergrund ───────────────────────────────────────────────────────────
    steps = 20
    for i in range(steps):
        frac = i / steps
        color(int(5 + frac * 10), int(15 + frac * 50), int(60 + frac * 120))
        alpha(1.0)
        y0 = int(i * 600 / steps)
        rect(0, y0, 800, int((i + 1) * 600 / steps) - y0)

    # ── Boden ─────────────────────────────────────────────────────────────────
    alpha(0.18)
    color(255, 255, 255)
    rect(0, CAT_FLOOR - 3, 800, 3)

    # ── Cat Input ─────────────────────────────────────────────────────────────
    left  = bool(keys & 1)
    right = bool(keys & 2)
    space = bool(keys & 16)
    space_down = space and not prev_space
    prev_space = space

    cat_vx = 0.0
    if left:
        cat_vx = -CAT_SPEED
        cat_right = False
    if right:
        cat_vx = CAT_SPEED
        cat_right = True
    if space_down and cat_ground:
        cat_vy = CAT_JUMP
        cat_ground = False

    # ── Cat Physik ────────────────────────────────────────────────────────────
    cat_vy += CAT_GRAVITY
    cat_x  += cat_vx
    cat_y  += cat_vy

    if cat_y >= CAT_FLOOR:
        cat_y = float(CAT_FLOOR)
        cat_vy = 0.0
        cat_ground = True

    if cat_x - CAT_W / 2 < 0:   cat_x = CAT_W / 2
    if cat_x + CAT_W / 2 > 800: cat_x = 800 - CAT_W / 2

    if abs(cat_vx) > 0.1 and cat_ground:
        cat_anim_t += 1 / 60

    # ── Klick: Ball greifen ODER Slingshot starten ────────────────────────────
    if btn_down and dragged is None and shoot_start is None:
        hit = None
        for b in balls:
            dx = mouse_x - b.x
            dy = mouse_y - b.y
            if dx * dx + dy * dy <= (b.r + 8) * (b.r + 8):
                hit = b
                break
        if hit:
            dragged = hit
            hit.vx = 0.0
            hit.vy = 0.0
            mouse_hist.clear()
            mouse_hist.append((mouse_x, mouse_y))
        else:
            shoot_start = (mouse_x, mouse_y)

    # ── Ball mitziehen ────────────────────────────────────────────────────────
    if dragged is not None and (mouse_btn & 1):
        dragged.x += (mouse_x - dragged.x) * 0.55
        dragged.y += (mouse_y - dragged.y) * 0.55

    # ── Ball loslassen → Wurf ─────────────────────────────────────────────────
    if btn_up and dragged is not None:
        if len(mouse_hist) >= 2:
            ox, oy = mouse_hist[0]
            ex, ey = mouse_hist[-1]
            n = len(mouse_hist) - 1
            dragged.vx = (ex - ox) / n * 0.9
            dragged.vy = (ey - oy) / n * 0.9
        dragged = None
        mouse_hist.clear()

    # ── Slingshot loslassen → Schuss-Ball spawnen ─────────────────────────────
    if btn_up and shoot_start is not None:
        sx, sy = shoot_start
        dx = mouse_x - sx
        dy = mouse_y - sy
        dist = math.sqrt(dx * dx + dy * dy)
        if dist > 5:
            speed = min(dist * 0.35, 18.0)
            vx = -(dx / dist) * speed
            vy = -(dy / dist) * speed
            colors = [
                (255, 80,  80),
                (255, 180, 40),
                (80,  255, 120),
                (80,  200, 255),
                (200, 80,  255),
                (255, 255, 80),
            ]
            cr, cg, cb = colors[shot_count % len(colors)]
            shot_count += 1
            balls.append(Ball(sx, sy, vx, vy, 14, cr, cg, cb))
            if len(balls) > 30:
                balls.pop(12)
        shoot_start = None

    # ── Bälle Physik (3 Substeps gegen Tunneling) ─────────────────────────────
    STEPS = 3
    inv = 1.0 / STEPS
    for _ in range(STEPS):
        # Bewegung (1/STEPS pro Sub-Step)
        for b in balls:
            if b is dragged:
                continue
            b.x += b.vx * inv
            b.y += b.vy * inv
            if b.x - b.r < 0:   b.x = b.r;       b.vx = abs(b.vx)
            if b.x + b.r > 800: b.x = 800 - b.r; b.vx = -abs(b.vx)
            if b.y - b.r < 0:   b.y = b.r;       b.vy = abs(b.vy)
            if b.y + b.r > 600: b.y = 600 - b.r; b.vy = -abs(b.vy)

        # Ball-zu-Ball Kollision
        for i in range(len(balls)):
            for j in range(i + 1, len(balls)):
                a, b2 = balls[i], balls[j]
                dx = b2.x - a.x
                dy = b2.y - a.y
                dist = math.sqrt(dx * dx + dy * dy)
                min_d = a.r + b2.r
                if dist < min_d and dist > 0.001:
                    nx = dx / dist
                    ny = dy / dist
                    ov = (min_d - dist) * 0.5
                    if a is not dragged:  a.x -= nx * ov;  a.y -= ny * ov
                    if b2 is not dragged: b2.x += nx * ov; b2.y += ny * ov
                    rel = (a.vx - b2.vx) * nx + (a.vy - b2.vy) * ny
                    if rel > 0:
                        if a is not dragged:  a.vx -= rel * nx; a.vy -= rel * ny
                        if b2 is not dragged: b2.vx += rel * nx; b2.vy += rel * ny

        # Katze ↔ Ball Kollision (pro Sub-Step)
        for b in balls:
            if b is dragged:
                continue
            dx = cat_x - b.x
            dy = cat_y - b.y
            dist = math.sqrt(dx * dx + dy * dy)
            min_d = b.r + CAT_R
            if dist < min_d and dist > 0.001:
                nx = dx / dist
                ny = dy / dist
                overlap = min_d - dist
                b.x -= nx * overlap * 0.8
                b.y -= ny * overlap * 0.8
                cat_x += nx * overlap * 0.2
                cat_x = max(CAT_W / 2, min(800 - CAT_W / 2, cat_x))
                rel_vx = cat_vx - b.vx
                rel_vy = cat_vy - b.vy
                dot = rel_vx * nx + rel_vy * ny
                if dot > 0:
                    b.vx -= nx * dot * 1.3
                    b.vy -= ny * dot * 1.3
                    if not cat_ground:
                        cat_vy += ny * dot * 0.15

    # ── Cursor-Kollision ──────────────────────────────────────────────────────
    cur_r  = 16
    cvx    = mouse_x - prev_mx
    cvy    = mouse_y - prev_my
    for b in balls:
        if b is dragged:
            continue
        dx = b.x - mouse_x
        dy = b.y - mouse_y
        dist = math.sqrt(dx * dx + dy * dy)
        min_d = cur_r + b.r
        if dist < min_d and dist > 0.001:
            nx = dx / dist
            ny = dy / dist
            b.x += nx * (min_d - dist)
            b.y += ny * (min_d - dist)
            push = max((cvx * nx + cvy * ny) * 1.4, 0.0)
            b.vx += nx * push
            b.vy += ny * push
    prev_mx = mouse_x
    prev_my = mouse_y

    # ── Bälle zeichnen ────────────────────────────────────────────────────────
    for b in balls:
        alpha(1.0)
        color(b.cr, b.cg, b.cb)
        circle(b.x, b.y, b.r)
        alpha(0.35)
        color(255, 255, 255)
        circle(b.x - b.r * 0.3, b.y - b.r * 0.3, b.r * 0.35)

    # Wurfpfeil beim Drag
    if dragged is not None and len(mouse_hist) >= 2:
        ox2, oy2 = mouse_hist[0]
        ex2, ey2 = mouse_hist[-1]
        n2 = len(mouse_hist) - 1
        pvx = (ex2 - ox2) / n2 * 0.9
        pvy = (ey2 - oy2) / n2 * 0.9
        for s in range(1, 7):
            frac = s / 6
            px = dragged.x + pvx * s * 6
            py = dragged.y + pvy * s * 6 + 0.5 * s * s * 0.15
            alpha(0.7 - frac * 0.55)
            color(255, 240, 100)
            circle(px, py, max(4 - frac * 2.5, 1.0))
        alpha(0.7)
        color(255, 255, 255)
        circle(dragged.x, dragged.y, dragged.r + 5)

    # ── Slingshot-Vorschau ────────────────────────────────────────────────────
    if shoot_start is not None:
        sx, sy = shoot_start
        dx = mouse_x - sx
        dy = mouse_y - sy
        dist = math.sqrt(dx * dx + dy * dy)
        alpha(0.5)
        color(255, 200, 80)
        line(sx, sy, mouse_x, mouse_y)
        if dist > 5:
            speed = min(dist * 0.35, 18.0)
            vx = -(dx / dist) * speed
            vy = -(dy / dist) * speed
            alpha(0.4)
            color(255, 200, 80)
            circle(sx, sy, 14)
            for s in range(1, 10):
                frac = s / 9
                px = sx + vx * s * 3
                py = sy + vy * s * 3
                alpha(0.6 - frac * 0.5)
                color(255, 200, 80)
                circle(px, py, max(10 - frac * 8, 2.0))

    # ── Cat Frame auswählen ───────────────────────────────────────────────────
    if not cat_ground:
        cat_frame = 3 if cat_vy < 0 else 4
    elif abs(cat_vx) > 0.1:
        cat_frame = int(cat_anim_t * 10) % 5 + 1
    else:
        cat_frame = 5

    # ── Cat zeichnen ──────────────────────────────────────────────────────────
    # draw_x ist immer top-left; negatives draw_w = horizontal gespiegelt
    draw_x = cat_x - CAT_W / 2
    draw_w = CAT_W if cat_right else -CAT_W
    draw_y = cat_y - CAT_H

    image("cat_0" + str(cat_frame) + ".svg", draw_x, draw_y, draw_w, CAT_H)
