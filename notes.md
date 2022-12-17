# Todo:
- Debug crash with +1 snake
    - Set max nodes for new profile

- Fix timeout when node down
    - Test fix with explicit connection timeout

- Add playout strength
    - test, improve

- Investigate performance without locking heuristic
- Investigate virtual loss / watch the unobserved
- Investigate integrating royale shrink into tree branching
    - Choose random shrink direction when moving down tree
    - May be complicated, high branch-factor gives us randomization even without

- Find test snakes for win rate benchmarking

- View assembly, investigate vector intrinsics

- Propagate thread pool panic instead of crash
