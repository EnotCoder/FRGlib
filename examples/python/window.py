import ctypes
import time

lib = ctypes.CDLL("./build/libfrglib.so")


lib.frg_create(b"game", 500, 500, True)

while lib.frg_poll():
    time.sleep(0.01)

lib.frg_close()