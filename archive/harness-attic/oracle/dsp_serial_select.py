"""Runs select_pure.py's main() with multiprocessing.Pool replaced by a serial map (the sandbox refused to create the Pool's
semaphores: OSError 122). Same code path otherwise. usage: serial_select.py PATH/select_pure.py PE DISPATCH_C OUT.json"""
import importlib.util
import multiprocessing as mp
import sys


class SerialPool:
    def __init__(self, n, initializer=None, initargs=()):
        if initializer:
            initializer(*initargs)

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False

    def map(self, f, it, chunksize=1):
        return list(map(f, it))


mp.Pool = SerialPool
path = sys.argv[1]
sys.path.insert(0, __import__("os").path.dirname(path))
spec = importlib.util.spec_from_file_location("select_pure_under_test", path)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
sys.argv = [path] + sys.argv[2:]
m.main()
