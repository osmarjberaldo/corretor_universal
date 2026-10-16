"""
Gera o icone oficial do Corretor Universal em formato ICO multipla resolucao
(16x16, 24x24, 32x32, 48x48, 64x64, 128x128, 256x256)
no estilo Apple Liquid Glass moderno com gradiente azul, anel de vidro e simbolo 'A' + check.
"""
import math
from PIL import Image, ImageDraw, ImageFont

def criar_imagem_base(size=512):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    margin = size * 0.06
    r = size * 0.22  # raio do squircle

    x0, y0 = margin, margin
    x1, y1 = size - margin, size - margin

    # Fundo com gradiente sutil simulado
    # Azul Apple System / Azure royal (#1a73e8 -> #0b57d0)
    for i in range(int(y0), int(y1)):
        t = (i - y0) / (y1 - y0)
        # Interpolação de cores (topo: 32, 130, 240 -> base: 12, 85, 195)
        r_c = int(32 * (1 - t) + 12 * t)
        g_c = int(135 * (1 - t) + 88 * t)
        b_c = int(245 * (1 - t) + 205 * t)
        
        # Mascara de cantos arredondados
        # Desenha fatia horizontal
        draw.line([(x0, i), (x1, i)], fill=(r_c, g_c, b_c, 255), width=1)

    # Mascara arredondada com cantos suaves
    mask = Image.new("L", (size, size), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle([x0, y0, x1, y1], radius=r, fill=255)

    # Aplica máscara ao gradiente
    fundo = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    fundo.paste(img, (0, 0), mask)

    draw_fundo = ImageDraw.Draw(fundo)

    # Anel de borda vítrea translúcida (efeito Liquid Glass)
    draw_fundo.rounded_rectangle(
        [x0, y0, x1, y1],
        radius=r,
        outline=(255, 255, 255, 70),
        width=int(size * 0.018)
    )

    # Brilho sutil no topo
    topo_mask = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    topo_draw = ImageDraw.Draw(topo_mask)
    topo_draw.rounded_rectangle(
        [x0 + 4, y0 + 4, x1 - 4, y0 + (y1 - y0) * 0.42],
        radius=r * 0.8,
        fill=(255, 255, 255, 30)
    )
    fundo = Image.alpha_composite(fundo, topo_mask)

    draw_final = ImageDraw.Draw(fundo)

    # Desenho central: Letra 'A' estilizada maiúscula e marca de verificação (Check)
    # 1. Traços da letra 'A' moderna em branco límpido
    cx, cy = size * 0.44, size * 0.48
    w = size * 0.42
    h = size * 0.48

    p_topo = (cx, cy - h / 2)
    p_esq = (cx - w / 2, cy + h / 2)
    p_dir = (cx + w / 2, cy + h / 2)
    esp = int(size * 0.055)

    # Sombra sutil projetada para profundidade
    sombra_cor = (0, 30, 80, 100)
    s_off = int(size * 0.016)

    def draw_a(dx, dy, cor):
        # Perna esquerda
        draw_final.line([(p_topo[0] + dx, p_topo[1] + dy), (p_esq[0] + dx, p_esq[1] + dy)], fill=cor, width=esp)
        # Perna direita
        draw_final.line([(p_topo[0] + dx, p_topo[1] + dy), (p_dir[0] + dx, p_dir[1] + dy)], fill=cor, width=esp)
        # Barra horizontal do 'A'
        by = cy + h * 0.12
        b_x0 = cx - w * 0.26
        b_x1 = cx + w * 0.26
        draw_final.line([(b_x0 + dx, by + dy), (b_x1 + dx, by + dy)], fill=cor, width=esp)
        # Vértice suavizado
        draw_final.ellipse(
            [p_topo[0] + dx - esp//2, p_topo[1] + dy - esp//2, p_topo[0] + dx + esp//2, p_topo[1] + dy + esp//2],
            fill=cor
        )

    # Desenha sombra do 'A'
    draw_a(0, s_off, sombra_cor)
    # Desenha o 'A' em branco puro
    draw_a(0, 0, (255, 255, 255, 255))

    # 2. Símbolo de "Check" (verificação / ortografia correta) no canto inferior direito
    # Check verde-esmeralda vibrante ou ciano vítreo no círculo
    circ_cx = size * 0.70
    circ_cy = size * 0.70
    circ_r = size * 0.18

    # Círculo do check com sombra
    draw_final.ellipse(
        [circ_cx - circ_r, circ_cy - circ_r + s_off, circ_cx + circ_r, circ_cy + circ_r + s_off],
        fill=sombra_cor
    )
    # Círculo verde esmeralda Apple (#34C759)
    draw_final.ellipse(
        [circ_cx - circ_r, circ_cy - circ_r, circ_cx + circ_r, circ_cy + circ_r],
        fill=(52, 199, 89, 255)
    )
    # Borda do círculo
    draw_final.ellipse(
        [circ_cx - circ_r, circ_cy - circ_r, circ_cx + circ_r, circ_cy + circ_r],
        outline=(255, 255, 255, 200),
        width=int(size * 0.016)
    )

    # Marca do check branco
    p1 = (circ_cx - circ_r * 0.44, circ_cy - circ_r * 0.02)
    p2 = (circ_cx - circ_r * 0.10, circ_cy + circ_r * 0.38)
    p3 = (circ_cx + circ_r * 0.48, circ_cy - circ_r * 0.36)
    check_esp = int(size * 0.042)

    draw_final.line([p1, p2], fill=(255, 255, 255, 255), width=check_esp)
    draw_final.line([p2, p3], fill=(255, 255, 255, 255), width=check_esp)
    draw_final.ellipse([p2[0]-check_esp//2, p2[1]-check_esp//2, p2[0]+check_esp//2, p2[1]+check_esp//2], fill=(255, 255, 255, 255))

    return fundo

def main():
    img512 = criar_imagem_base(512)
    img512.save("icone.png", format="PNG")
    print("Salvo: icone.png (512x512)")

    tamanhos = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    img512.save("icone.ico", format="ICO", sizes=tamanhos)
    print("Salvo: icone.ico (multi-resolucao 16..256)")

if __name__ == "__main__":
    main()
