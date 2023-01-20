import os
import sys
import pprint
import asyncio
import itertools

from skopt.space import Real
from skopt import Optimizer

NUM_WORKERS = int(os.environ["NUM_WORKERS"])
BASE_PORT = 8090


async def start_snake(index, opt_args=None):
    max_boards = os.environ["MAX_BOARDS"]
    max_width = os.environ["MAX_WIDTH"]
    max_height = os.environ["MAX_HEIGHT"]
    max_snakes = os.environ["MAX_SNAKES"]
    num_threads = os.environ["NUM_THREADS"]

    snake_port = BASE_PORT + index * (NUM_WORKERS + 1)

    snake_args = [
        "--port", f"{snake_port}",
        "--max-boards", "0",
        "--max-width", max_width,
        "--max-height", max_height,
        "--max-snakes", max_snakes,
        "--num-parallel-reqs", "1",
        "--worker-node", "127.0.0.1",
        "--worker-pod", f"http://127.0.0.1:{snake_port + 1}",
        "--worker-pod", f"http://127.0.0.1:{snake_port + 2}",
        "--latency", "30",
        "relay",
    ]

    snake_args = snake_args + opt_args if opt_args else snake_args

    snake_handle = await asyncio.create_subprocess_exec(
        "./target-snake/release/conesnake",
        *snake_args,
        stderr=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
    )

    worker_args = [
        "--num-worker-threads", num_threads,
        "--max-boards", max_boards,
        "--max-width", max_width,
        "--max-height", max_height,
        "--max-snakes", max_snakes,
        "--latency", "60",
        "worker",
    ]
    worker_args = worker_args + opt_args if opt_args else worker_args

    worker_handles = []

    for i in range(NUM_WORKERS):
        worker_i_args = [
            "--port", f"{snake_port + i + 1}"
        ] + worker_args

        worker_handles += [await asyncio.create_subprocess_exec(
            "./target-snake/release/conesnake",
            *worker_i_args,
            stderr=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
        )]

    return snake_handle, worker_handles


async def start_rules(snakes):
    rules_args = [
        "--timeout", os.environ["TIMEOUT"],
        "--width", os.environ["MAX_WIDTH"],
        "--height", os.environ["MAX_HEIGHT"],
        "--gametype", "standard",
        "--map", "standard",
        "--foodSpawnChance", "15",
    ] + list(itertools.chain.from_iterable([
        "--name", s, "--url", f"http://127.0.0.1:{BASE_PORT + i*(NUM_WORKERS + 1)}"
    ] for i, s in enumerate(snakes)))

    return await asyncio.create_subprocess_exec(
        "scripts/entrypoint_rules.sh",
        *rules_args,
        stderr=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
    )


async def print_stream(name, stream, out_fd=sys.stdout):
    output = []

    while True:
        line = await stream.readline()
        if not line:
            break
        line = line.decode()
        output.append(line)
        print(f"{name: <22}|  {line}", file=out_fd, end="")
    return "".join(output)


async def run_games(**kwargs):
    opt_args = list(itertools.chain.from_iterable((
        f"--{key}", str(value)
    ) for (key, value) in kwargs.items()))

    num_games = 500
    num_opponents = 1

    wins = 0

    for i in range(num_games):

        print("\n-------------------------\n")
        print(f"game {i}/{num_games}")
        print(f"wins {wins}\n")

        wait_handles = []

        conesnake, conesnake_workers = await start_snake(0, opt_args)

        wait_handles += [("conesnake", conesnake)]
        wait_handles += [(f"conesnake-worker-{i}", w)
                         for i, w in enumerate(conesnake_workers)]

        for i in range(num_opponents):
            oponent, oponent_handles = await start_snake(i + 1)
            wait_handles += [(f"oponent-{i}", oponent)]
            wait_handles += [(f"oponent-{i}-worker-{j}", w)
                             for j, w in enumerate(oponent_handles)]

        rules = await start_rules(
            ["conesnake"] + [f"oponent-{i}" for i in range(num_opponents)]
        )

        rules_handles = [
            asyncio.create_task(print_stream("rules", rules.stdout)),
            asyncio.create_task(print_stream(
                "rules-err", rules.stderr, sys.stderr)),
        ]

        debug_handles = list(itertools.chain.from_iterable([
            asyncio.create_task(print_stream(f"{name}", h.stdout)),
            asyncio.create_task(print_stream(
                f"{name}-err", h.stderr, sys.stderr)),
        ] for name, h in wait_handles))

        running = set(rules_handles + debug_handles)
        rules_done = set()

        while True:
            done, _ = await asyncio.wait(
                running, return_when=asyncio.FIRST_COMPLETED
            )

            for d in done:
                rules_done.add(d)
                running.remove(d)

            if len(rules_done) == 2:
                break

        exit_code = await rules.wait()

        assert exit_code == 0

        for _, h in wait_handles:
            h.terminate()
        for _, h in wait_handles:
            await h.wait()

        is_win = False
        for task in rules_done:
            is_win = is_win or task.result().find("conesnake was the winner.") > 0

        if is_win:
            wins += 1

    return (num_games - wins) / num_games


async def main():
    os.environ["TARGET"] = "release"
    os.environ["RUST_BACKTRACE"] = "1"
    os.environ["RUST_LOG"] = "warn"

    pretty = pprint.PrettyPrinter(indent=4)

    dimensions = [
        Real(0.7, 6.0, name="temperature"),
    ]

    opt = Optimizer(
        dimensions=dimensions,
    )

    num_calls = 100
    calls = 0

    while calls < num_calls:
        # n_points = min(num_parallel, num_calls - calls)
        x = opt.ask()

        print("\n*************************\n")
        print("Starting evaluation jobs with args:\n")

        args = {dim.name: x[idx] for idx, dim in enumerate(dimensions)}

        pretty = pprint.PrettyPrinter(indent=4)

        print(f"{pretty.pformat(args)}\n")

        msg = ""
        msg += f"gp_minimize call {calls}/{num_calls}\n"
        msg += "args:\n"
        msg += pretty.pformat(x)

        y = await run_games(**args)

        msg += "\nloss rate:\n"
        msg += pretty.pformat(y)

        print(msg)

        opt.tell(x, y)
        calls += 1

    min_loss_idx = opt.yi.index(min(opt.yi))
    min_loss = opt.yi[min_loss_idx]

    res = {dimensions[idx].name: x for idx,
           x in enumerate(opt.Xi[min_loss_idx])}

    print(f"Solution:\n{pretty.pformat(res)}\nmin_loss: {min_loss}")


if __name__ == "__main__":
    asyncio.run(main())
