import tkinter as tk
from tkinter import messagebox

def main():
    root = tk.Tk()
    root.title("LimitGadget")
    root.geometry("320x220")
    root.configure(bg="#1f2937")
    
    tk.Label(root, text="Limites IA", font=("Segoe UI", 14, "bold"), fg="white", bg="#1f2937").pack(pady=10)
    
    for item in [
        ("Claude Code", "84%  (reseta 03/11)", "#D97757"),
        ("ChatGPT", "0%  (plano free)", "#10A37F"),
        ("Antigravity", "—  (IDE aberto)", "#4285F4"),
    ]:
        frame = tk.Frame(root, bg="#374151", padx=10, pady=6)
        frame.pack(fill=tk.X, padx=10, pady=2)
        tk.Label(frame, text=item[0], font=("Segoe UI", 10, "bold"), fg="white", bg="#374151").pack(side=tk.LEFT)
        tk.Label(frame, text=item[1], font=("Segoe UI", 9), fg=item[2], bg="#374151").pack(side=tk.RIGHT)
    
    tk.Label(root, text="Atualiza a cada 60s  •  Notifica aos 90%", font=("Segoe UI", 8), fg="#9ca3af", bg="#1f2937").pack(pady=8)
    
    root.protocol("WM_DELETE_WINDOW", lambda: root.withdraw())
    root.mainloop()

if __name__ == "__main__":
    main()
