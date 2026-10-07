import ctypes
import time

lib = ctypes.CDLL("./build/libfrglib.so")


lib.frg_create()

while lib.frg_poll():
    time.sleep(0.01)

lib.frg_close()