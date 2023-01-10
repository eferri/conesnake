import json
import pprint
import os
import asyncio

from itertools import chain

from skopt.space import Real, Integer
from skopt import Optimizer


async def run_search(call, num_calls, **kwargs):
    pretty = pprint.PrettyPrinter(indent=4)

    print(f"{pretty.pformat(kwargs)}\n")

    msg = "\n-------------------------\n"
    msg += f"gp_minimize call {call}/{num_calls}\n"
    msg += "args:\n"
    msg += pretty.pformat(kwargs)

    args = list(chain.from_iterable((
        f"--{key}", str(value)
    ) for (key, value) in kwargs.items()))

    proc = await asyncio.create_subprocess_exec(
        "./target-snake/release/performance",
        *args,
        stderr=asyncio.subprocess.DEVNULL,
        stdout=asyncio.subprocess.PIPE,
    )
    stdout, _ = await proc.communicate()

    ret = json.loads(stdout)

    msg += "\nresults:\n"
    msg += pretty.pformat(ret)

    call += 1

    print(msg)

    return ret["loss"]


async def main():
    pretty = pprint.PrettyPrinter(indent=4)

    dimensions = [
        Real(0.5, 32.0, name="temperature"),
    ]

    opt = Optimizer(
        dimensions=dimensions,
    )

    num_parallel = os.cpu_count() // 8

    num_calls = 100
    calls = 0

    while calls < num_calls:
        n_points = min(num_parallel, num_calls - calls)
        x_list = opt.ask(n_points=n_points)
        x_kwargs = [{dim.name: x[idx]
                     for idx, dim in enumerate(dimensions)} for x in x_list]

        print("\n*************************\n")
        print("Starting evaluation jobs with args:\n")

        y_list = await asyncio.gather(*[
            run_search(calls + idx, num_calls, **search_kwargs) for idx, search_kwargs in enumerate(x_kwargs)
        ])

        opt.tell(x_list, y_list)
        calls += num_parallel

    min_loss_idx = opt.yi.index(min(opt.yi))
    min_loss = opt.yi[min_loss_idx]

    res = {dimensions[idx].name: x for idx,
           x in enumerate(opt.Xi[min_loss_idx])}

    print(f"Solution:\n{pretty.pformat(res)}\nmin_loss: {min_loss}")


if __name__ == "__main__":
    asyncio.run(main())
