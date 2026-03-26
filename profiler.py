import time
from functools import wraps
from typing import Callable, Any

class Profiler:
    def __init__(self):
        self.func_names = []
        self.func_times = {}
        self.func_calls = {}

        self.disabled = False
    def register(self, func: Callable) -> Callable:
        if self.disabled: return func
        name = func.__name__
        if name not in self.func_names:
            self.func_names.append(name)
            self.func_times[name] = 0
            self.func_calls[name] = 0

        # Local variable access is faster than attribute access
        func_times = self.func_times
        func_calls = self.func_calls
        perf = time.perf_counter

        @wraps(func)
        def wrapper(*args, **kwargs) -> Any:
            if self.disabled: return func(*args, **kwargs)
            start = perf()
            res = func(*args, **kwargs)
            func_times[name] += perf() - start
            func_calls[name] += 1
            return res
        return wrapper

    def print_results(self, ignore_smalls=False) -> None:
        if self.disabled: return
        total_time = sum(self.func_times.values())
        if total_time == 0:
            print("No time recorded.")
            return

        for func in self.func_names:
            time_spent = self.func_times[func]
            percentage = 100 * (time_spent / total_time)
            if ignore_smalls and percentage < 0.1:
                continue

            print(f"Function \"{func}\"")
            calls = self.func_calls[func]
            if calls == 0:
                print("was never called")
            else:
                print(f"was called {calls} times, ")
                print(f"taking {time_spent:.2f} seconds in total.")
                print(f"It took {time_spent / calls:.6f} on average")
                print(f"and about {percentage:.2f}% of the time")
            print("-" * 50)
    def disable(self):
        self.disabled = True
prof = Profiler()
