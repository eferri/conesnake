import json
from itertools import chain

from skopt import gp_minimize
from skopt.space import Real, Integer
from skopt.utils import use_named_args

import subprocess

search_space = [
    Real(0.2, 8.0, name="temperature"),
    Real(0.00001, 0.5, name="virtual-loss"),
    Integer(1, 100000, name="rave-equiv"),
    Integer(0, 64, name="rave-moves"),
]


@use_named_args(search_space)
def run_search(**kwargs):
    cfg_args = list(chain.from_iterable((f"--{key}", str(value)) for (key, value) in kwargs.items()))

    args = ["./target-snake/release/performance"] + cfg_args

    print(f"Running with args: {args}")

    ret_str = subprocess.run(
        args=args,
        stderr=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        check=True,
        encoding="utf-8",
    )

    print(f"Results: {ret_str.stdout}")

    ret = json.loads(ret_str.stdout)

    return ret["failures"]


if __name__ == "__main__":
    res = gp_minimize(run_search, search_space, n_calls=100, n_jobs=3)

    print(res.x)
