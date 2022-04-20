import json
from itertools import product
from typing import List, Tuple


def bit_test(bits: int, bit: int) -> bool:
    return (bits & (1 << bit)) != 0


def debug_nibble_lists(l: List[Tuple[List[int], List[int]]]) -> None:
    for i, (highs, lows) in enumerate(l):
        chars = json.dumps("".join([chr(high << 4 | low) for high, low in product(highs, lows)]))
        if highs:
            print(f"Bit {i}: {chars[1:-1]} (high={highs}, low={lows})")


def parse_shufti_mask(high: List[int], low: List[int]) -> List[Tuple[List[int], List[int]]]:
    return [
        (
            [j for j, e in enumerate(high) if bit_test(e, i)],
            [j for j, e in enumerate(low) if bit_test(e, i)]
        ) for i in range(8)
    ]


def debug_nibble_masks(high: List[int], low: List[int]) -> None:
    debug_nibble_lists(parse_shufti_mask(high, low))


if __name__ == '__main__':
    debug_nibble_masks(
        [8, 0, 18, 4, 0, 1, 0, 1, 0, 0, 0, 3, 2, 1, 0, 0],
        [16, 0, 0, 0, 0, 0, 0, 0, 0, 8, 12, 1, 2, 9, 0, 0]
    )
