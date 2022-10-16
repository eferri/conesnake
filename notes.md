# Todo:

- Fix timeout when node down
- Add playout strength
- View assembly, vector commands
- Propagate thread pool panic instead of crash


# Performance scaling

5950x, threads - total nodes

## Single worker

32 - 840k
28 - 836k
24 - 828k
20 - 824k
16 - 849k
14 - 862k
12 - 815k
10 - 720k
8  - 625k


## Two workers

16 - 614k
12 - 595k
8  - 574k
4  - 366k

## Four workers

8 - 408k
