from skopt import Optimizer
from skopt.space import Real, Integer
import os
import sys
import pprint
import asyncio
import itertools
import argparse


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
        "--latency", "50",
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


async def run_games(num_games=250, num_opponents=1, **kwargs):
    opt_args = []
    for key, value in kwargs.items():
        if isinstance(value, bool):
            opt_args.append((f"--{key}",))
        else:
            opt_args.append((f"--{key}", str(value)))

    opt_args = list(itertools.chain.from_iterable(opt_args))

    wins = 0
    draws = 0

    wait_handles = []

    conesnake, conesnake_workers = await start_snake(0, opt_args)

    wait_handles += [("conesnake", conesnake)]
    wait_handles += [(f"conesnake-worker-{i}", w)
                     for i, w in enumerate(conesnake_workers)]

    for i in range(num_opponents):
        oponent, oponent_workers = await start_snake(i + 1)
        wait_handles += [(f"oponent-{i}", oponent)]
        wait_handles += [(f"oponent-{i}-worker-{j}", w)
                         for j, w in enumerate(oponent_workers)]

    # Print output of snake tasks
    for name, h in wait_handles:
        asyncio.create_task(print_stream(f"{name}", h.stdout)),
        asyncio.create_task(print_stream(
            f"{name}-err", h.stderr, sys.stderr)),

    for i in range(num_games):

        print("\n-------------------------\n")
        print(f"game {i + 1}/{num_games}")
        print(f"wins {wins}")
        print(f"draws {draws}")
        print(f"losses {i + 1 - wins - draws}")

        rules = await start_rules(
            ["conesnake"] + [f"oponent-{j}" for j in range(num_opponents)]
        )

        rules_output = await asyncio.gather(
            rules.wait(),
            print_stream("rules", rules.stdout),
            print_stream("rules-err", rules.stderr, sys.stderr),
        )

        exit_code = rules_output[0]
        assert exit_code == 0

        is_win = False
        is_draw = False

        for output in rules_output[1:]:
            is_win = is_win or output.find("conesnake was the winner.") > 0
            is_draw = is_draw or output.find("It was a draw.") > 0

        if is_win:
            wins += 1

        if is_draw:
            draws += 1

    for _, h in wait_handles:
        h.terminate()
    for _, h in wait_handles:
        await h.wait()

    print(f"final: wins {wins} draws {draws}")

    return (num_games - wins) / num_games


async def optimize():
    dimensions = [
        Real(0.7, 6.0, name="temperature"),
        Integer(0, 1, name="strong-playout"),
    ]

    opt = Optimizer(
        dimensions=dimensions,
    )

    pretty = pprint.PrettyPrinter(indent=4)

    num_calls = 100
    calls = 0

    while calls < num_calls:
        x = opt.ask()

        print("\n*************************\n")
        print("Starting evaluation jobs with args:\n")

        kwargs = {}
        for idx, dim in enumerate(dimensions):
            if isinstance(dim, Integer) and dim.bounds == (0, 1):
                kwargs[dim.name] = bool(x[idx])
            else:
                kwargs[dim.name] = x[idx]

        print(f"{pretty.pformat(kwargs)}\n")
        print(f"game {calls}/{num_calls}")
        print("args:")
        print(pretty.pformat(x))

        y = await run_games(**kwargs)

        print("\nloss rate:")
        print(pretty.pformat(y))

        opt.tell(x, y)
        calls += 1

    min_loss_idx = opt.yi.index(min(opt.yi))
    min_loss = opt.yi[min_loss_idx]

    res = {dimensions[idx].name: x for idx,
           x in enumerate(opt.Xi[min_loss_idx])}

    print(f"Best results:\n{pretty.pformat(res)}\nmin_loss: {min_loss}")


async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", type=str)
    args = parser.parse_args()

    os.environ["TARGET"] = "release"
    os.environ["RUST_BACKTRACE"] = "1"
    os.environ["RUST_LOG"] = "warn"

    match args.mode:
        case "optimize":
            await optimize()
        case "compare":
            await run_games(num_games=10000, compare=True)
        case _:
            raise ValueError(f"invalid mode {args.mode}")

if __name__ == "__main__":
    asyncio.run(main())
