import time

import frglib

with frglib.Window("FRGLib", 800, 600) as window:
    print(window)
    while window.poll():
        time.sleep(0.01)
