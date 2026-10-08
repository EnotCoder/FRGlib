import time

import frglib

with frglib.Window("FRGLib", 800, 600) as window:
    while window.poll():
        window.frame()
        time.sleep(1 / 60)
