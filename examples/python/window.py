import time
import frglib

with frglib.Window("Game", 800, 600) as window:
    window.set_background(255, 255, 255, 255)
    while window.poll():
        window.frame()
        time.sleep(1 / 60)
