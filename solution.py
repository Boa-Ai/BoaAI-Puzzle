#!/usr/bin/env python3
"""Solve the BoaAI 6-button lattice puzzle from all OFF to a target state."""

from __future__ import annotations

import argparse
from collections import deque
from typing import Iterable

BUTTON_COUNT = 6
COLOR_COUNT = 6
START_STATE = (0,) * BUTTON_COUNT
DISTANCE_DELTAS = {
    0: 2,  # pressed button
    1: 1,  # immediate neighbors
    2: 5,  # one step backward modulo 6
    3: 3,  # opposite button
}

COLOR_TO_INT = {
    "OFF": 0,
    "GREEN": 1,
    "BLUE": 2,
    "RED": 3,
    "PURPLE": 4,
    "WHITE": 5,
}
INT_TO_COLOR = {value: key for key, value in COLOR_TO_INT.items()}


def parse_target(raw: str) -> tuple[int, ...]:
    tokens = [part.strip().upper() for part in raw.replace("|", ",").split(",") if part.strip()]
    if len(tokens) == 1 and " " in tokens[0]:
        tokens = [part.strip().upper() for part in tokens[0].split() if part.strip()]

    if len(tokens) != BUTTON_COUNT:
        raise ValueError(f"Expected {BUTTON_COUNT} values, got {len(tokens)}")

    values: list[int] = []
    for token in tokens:
        if token in COLOR_TO_INT:
            values.append(COLOR_TO_INT[token])
            continue

        if token.isdigit():
            number = int(token)
            if 0 <= number < COLOR_COUNT:
                values.append(number)
                continue

        raise ValueError(
            f"Invalid token '{token}'. Use numbers 0-5 or colors OFF/GREEN/BLUE/RED/PURPLE/WHITE."
        )

    return tuple(values)


def press(state: tuple[int, ...], index: int) -> tuple[int, ...]:
    updated = list(state)
    for target in range(BUTTON_COUNT):
        clockwise = (target + BUTTON_COUNT - index) % BUTTON_COUNT
        counterclockwise = (index + BUTTON_COUNT - target) % BUTTON_COUNT
        distance = min(clockwise, counterclockwise)
        delta = DISTANCE_DELTAS[distance]
        updated[target] = (updated[target] + delta) % COLOR_COUNT
    return tuple(updated)


def shortest_solution(target: tuple[int, ...]) -> list[int] | None:
    if target == START_STATE:
        return []

    queue = deque([START_STATE])
    parent: dict[tuple[int, ...], tuple[tuple[int, ...], int] | None] = {START_STATE: None}

    while queue:
        state = queue.popleft()
        for button in range(BUTTON_COUNT):
            next_state = press(state, button)
            if next_state in parent:
                continue

            parent[next_state] = (state, button)
            if next_state == target:
                path: list[int] = []
                cursor = next_state
                while parent[cursor] is not None:
                    previous, pressed = parent[cursor]  # type: ignore[misc]
                    path.append(pressed)
                    cursor = previous
                path.reverse()
                return path

            queue.append(next_state)

    return None


def format_state(state: Iterable[int]) -> str:
    names = [INT_TO_COLOR[value] for value in state]
    numbers = [str(value) for value in state]
    return f"{' | '.join(names)}   ({', '.join(numbers)})"


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Compute the shortest press sequence from all OFF for the BoaAI 6-button puzzle. "
            "Target accepts numbers or color names."
        )
    )
    parser.add_argument(
        "--target",
        required=True,
        help=(
            "Comma-separated target. Example: '5,4,1,5,4,1' "
            "or 'WHITE,PURPLE,GREEN,WHITE,PURPLE,GREEN'"
        ),
    )
    args = parser.parse_args()

    try:
        target = parse_target(args.target)
    except ValueError as error:
        print(f"Input error: {error}")
        return 2

    solution = shortest_solution(target)
    if solution is None:
        print("No solution found (unexpected for this puzzle).")
        return 1

    print("Start state :", format_state(START_STATE))
    print("Target state:", format_state(target))
    print(f"Moves       : {len(solution)}")
    print("Press order : " + ", ".join(str(button + 1) for button in solution))
    print("Zero-index  : " + ", ".join(str(button) for button in solution))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
