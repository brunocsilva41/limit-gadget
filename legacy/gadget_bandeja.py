import tkinter as tk
import pystray
from PIL import Image, ImageDraw

def criar_icone():
    img = Image.new('RGBA', (32, 32), (0,0,0,0))
    d = ImageDraw.Draw(img)
    d.ellipse([4,4,28,28], fill=(40,167,69))
    return img

def mostrar_painel():
    root = tk.Toplevel()
    root.title("Limites IA")
    root.geometry("280x170")
    root.configure(bg="#111827")
    root.overrideredirect(True)
    root.attributes("-topmost", True)
    
    tk.Label(root, text="Limites IA", font=("Segoe UI", 12, "bold"), fg="white", bg="#111827").pack(pady=6)
    
    linhas = [
        ("Claude Code", "84% (reseta 03/11)", "#D97757"),
        ("ChatGPT", "0% (plano free)", "#10A37F"),
        ("Antigravity", "— (IDE aberto)", "#4285F4"),
    ]
    for nome, pct, cor in linhas:
        f = tk.Frame(root, bg="#1f2937", padx=8, pady=4)
        f.pack(fill=tk.X, padx=8, pady=1)
        tk.Label(f, text=nome, font=("Segoe UI", 9, "bold"), fg="white", bg="#1f2937").pack(side=tk.LEFT)
        tk.Label(f, text=pct, font=("Segoe UI", 9), fg=cor, bg="#1f2937").pack(side=tk.RIGHT)
    
    tk.Label(root, text="Atualiza a cada 60s", font=("Segoe UI", 7), fg="#9ca3af", bg="#111827").pack(pady=4)

def abrir_menu(icon, item):
    mostrar_painel()

def sair(icon, item):
    icon.stop()

icone = pystray.Icon("limit", criar_icone(), menu=pystray.Menu(
    pystray.MenuItem("Mostrar Limites", abrir_menu),
    pystray.MenuItem("Sair", sair)
))

icone.run()
