#!/usr/bin/env python3
"""Mockup: objects on the app's dark wood and light leather cards (illustration only)."""
import sys
from PIL import Image, ImageDraw, ImageFont, ImageFilter
IMG, OUT = sys.argv[1], sys.argv[2]
F = "fonts/"
serif = ImageFont.truetype(F + "NotoSerif-Bold.ttf", 40)
sans = ImageFont.truetype(F + "NotoSans-Regular.ttf", 24)
items = [("menorah_temple", "Праздники", "Менора Храма и Ханука"),
         ("kiddush_cup", "Шаббат", "Кидуш, свечи и хала"),
         ("torah_rolled", "Основы", "Тора, народ и вера")]
themes = [("textures/tex-walnut.webp", (36, 23, 16), (201, 162, 90), (239, 227, 204), (191, 174, 143)),
          ("textures/tex-leather.webp", (200, 179, 139), (107, 71, 18), (42, 26, 12), (90, 66, 40))]
W, H, PAD = 560, 640, 36
sheet = Image.new("RGB", (PAD + len(items) * (W + PAD), PAD + 2 * (H + PAD)))
for r, (tex, card, line, fg, mut) in enumerate(themes):
    bg = Image.open(tex).convert("RGB").resize((sheet.width, H + 2 * PAD))
    sheet.paste(bg, (0, r * (H + PAD)))
    for c, (name, title, sub) in enumerate(items):
        x, y = PAD + c * (W + PAD), PAD + r * (H + PAD)
        panel = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        d = ImageDraw.Draw(panel)
        d.rounded_rectangle([0, 0, W - 1, H - 1], 22, fill=card + (238,), outline=line + (150,), width=2)
        glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        ImageDraw.Draw(glow).ellipse([90, 40, W - 90, 440], fill=(255, 190, 110, 40 if r == 0 else 26))
        panel.alpha_composite(glow.filter(ImageFilter.GaussianBlur(50)))
        obj = Image.open(f"{IMG}/objects/{name}-512.webp").convert("RGBA").resize((440, 440), Image.LANCZOS)
        panel.alpha_composite(obj, ((W - 440) // 2, 20))
        d = ImageDraw.Draw(panel)
        d.text((W // 2, 490), title, font=serif, fill=fg, anchor="mm")
        d.text((W // 2, 545), sub, font=sans, fill=mut, anchor="mm")
        sheet.paste(panel, (x, y), panel)
sheet.save(OUT, "WEBP", quality=90, method=6)
print(OUT, sheet.size)
