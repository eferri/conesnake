import matplotlib.pyplot as plt
from skopt import Optimizer
from skopt.space import Real, Integer
from skopt.plots import plot_convergence, plot_regret, plot_evaluations
import os
import sys
import time
import pprint
import asyncio
import aiohttp
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
        "--latency", "10",
        "relay",
    ]

    snake_args = snake_args + opt_args if opt_args else snake_args

    print("/target-snake/release/conesnake" + " ".join(snake_args))

    snake_handle = await asyncio.create_subprocess_exec(
        "./target-snake/release/conesnake",
        *snake_args,
        stderr=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
    )

    worker_args = [
        "--num-threads", num_threads,
        "--max-boards", max_boards,
        "--max-width", max_width,
        "--max-height", max_height,
        "--max-snakes", max_snakes,
        "--latency", "10",
        "worker",
    ]
    worker_args = worker_args + opt_args if opt_args else worker_args

    worker_handles = []

    for i in range(NUM_WORKERS):
        worker_i_args = [
            "--port", f"{snake_port + i + 1}"
        ] + worker_args

        print("/target-snake/release/conesnake" + " ".join(snake_args))

        worker_handles += [await asyncio.create_subprocess_exec(
            "./target-snake/release/conesnake",
            *worker_i_args,
            stderr=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
        )]

    return snake_handle, worker_handles, snake_port


async def start_rules(snakes, game_type="standard", map="standard"):
    rules_args = [
        "--timeout", "200",
        "--width", "11",
        "--height", "11",
        "--gametype", game_type,
        "--map", map,
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


async def get_status(session, url):
    try:
        async with session.get(url) as response:
            return response.status
    except aiohttp.client_exceptions.ClientConnectorError:
        return 500


async def run_games(num_games=500, num_opponents=2, **kwargs):
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
    snake_ports = []

    conesnake, conesnake_workers, conesnake_port = await start_snake(0, opt_args)

    snake_ports.append(conesnake_port)

    wait_handles += [("conesnake", conesnake)]
    wait_handles += [(f"conesnake-worker-{i}", w)
                     for i, w in enumerate(conesnake_workers)]

    for i in range(num_opponents):
        oponent, oponent_workers, oponent_port = await start_snake(i + 1)
        wait_handles += [(f"oponent-{i}", oponent)]
        wait_handles += [(f"oponent-{i}-worker-{j}", w)
                         for j, w in enumerate(oponent_workers)]
        snake_ports.append(oponent_port)

    # Print output of snake tasks
    for name, h in wait_handles:
        asyncio.create_task(print_stream(f"{name}", h.stdout)),
        asyncio.create_task(print_stream(
            f"{name}-err", h.stderr, sys.stderr)),

    # Wait for snakes to be ready
    while True:
        snake_futures = []
        async with aiohttp.ClientSession() as session:
            for snake_url in [f"http://127.0.0.1:{port}/ping" for port in snake_ports]:
                snake_futures.append(get_status(session, snake_url))

            statuses = await asyncio.gather(*snake_futures)

            if all([s == 200 for s in statuses]):
                break
            else:
                time.sleep(0.5)

    for i in range(num_games):
        print("\n-------------------------\n")
        print(f"game {i + 1}/{num_games}")
        print(f"wins {wins}")
        print(f"draws {draws}")
        print(f"losses {i - wins - draws}")
        print()

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

    losses = num_games - wins - draws

    print(f"final: wins {wins} draws {draws} losses {losses}")

    return (num_games - wins) / num_games


async def optimize():
    dimensions = [
        Real(1.7, 2.5, name="temperature"),
        Real(1.0, 50.0, name="win-val"),
        Real(-50.0, -1.0, name="loss-val"),
        Real(-50.0, 0.0, name="tie-val"),
        Integer(0, 1, name="strong-playout"),
        Integer(1, 100, name="min-playouts"),
    ]

    opt = Optimizer(
        dimensions=dimensions,
        base_estimator="GP",
        acq_func="gp_hedge",
        acq_optimizer="auto",
        initial_point_generator="grid",
        n_initial_points=8,
        random_state=0,
    )

    opt_result = None

    pretty = pprint.PrettyPrinter(indent=4)

    num_calls = 500

    for call in range(num_calls):
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
        print(f"evaluation {call + 1}/{num_calls}")
        print("args:")
        print(pretty.pformat(x))
        print()

        y = await run_games(**kwargs)

        print("\nloss rate:")
        print(pretty.pformat(y))

        opt_result = opt.tell(x, y)

        plot_convergence(opt_result)
        plt.savefig("convergence.png")
        plt.clf()
        plot_regret(opt_result)
        plt.savefig("regret.png")
        plt.clf()
        plot_evaluations(opt_result, dimensions=[
                         dim.name for dim in dimensions])
        plt.savefig("evaluation.png")
        plt.clf()

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
            await run_games(num_games=100, compare=True)
        case _:
            raise ValueError(f"invalid mode {args.mode}")

if __name__ == "__main__":
    asyncio.run(main())
