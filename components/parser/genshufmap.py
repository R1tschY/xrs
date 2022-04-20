import argparse
import itertools
import json
import re
import sys
from dataclasses import dataclass
from functools import reduce
from operator import attrgetter, or_, itemgetter
from typing import List, Dict, TypeVar, Iterable, Callable, Tuple, Iterator

T = TypeVar("T")
U = TypeVar("U")


@dataclass
class Item:
    chr: str
    ord: int
    high: int
    low: int


def group_by_unsorted(iterable: Iterable[T], key: Callable[[T], U]) -> Iterator[Tuple[U, Iterator[T]]]:
    return itertools.groupby(sorted(iterable, key=key), key=key)


def dict_by_unsorted(iterable: Iterable[T], key: Callable[[T], U]) -> Dict[U, List[T]]:
    return {
        k: list(iterator)
        for k, iterator in group_by_unsorted(iterable, key)
    }


def bit_mask(idx_bits: Iterable[int]) -> int:
    return reduce(or_, map(lambda x: (1 << x), idx_bits), 0)


def shufti_mask(values: List[int], bit: int) -> List[int]:
    return [(1 << bit) if i in values else 0 for i in range(16)]


def or_shufti_masks(left: List[int], right: List[int]) -> List[int]:
    return [l | r for l, r in zip(left, right)]


def debug_nibble_lists(l: List[Tuple[List[int], List[int]]]) -> None:
    for i, (highs, lows) in enumerate(l):
        chars = "".join([chr(high << 4 | low) for high, low in itertools.product(highs, lows)]) \
            .encode("unicode_escape").decode("ascii")
        if highs:
            print(f"Bit {i}: {chars} (high={highs}, low={lows})", file=sys.stderr)


def build_nibble_masks(l: List[Tuple[List[int], List[int]]], cat_bit: int) -> Tuple[List[int], List[int]]:
    return (
        reduce(or_shufti_masks, [
            shufti_mask(highs, i)
            for i, (highs, _) in enumerate(l, start=cat_bit)
        ], [0] * 16),
        reduce(or_shufti_masks, [
            shufti_mask(lows, i)
            for i, (_, lows) in enumerate(l, start=cat_bit)
        ], [0] * 16))


def build_shufti_masks(cats: List[str], verbose: bool) -> Tuple[List[int], List[int], List[int]]:
    low_mask = [0] * 16
    high_mask = [0] * 16
    cat_masks = []
    nibble_lists = []

    cat_bit = 0
    for cat in cats:
        items: List[Item] = []
        for c in cat:
            items.append(Item(c, ord(c), low=ord(c) & 0x0f, high=ord(c) >> 4))

        # try to reduce on high bits
        hreduce: Dict[int, List[Item]] = dict_by_unsorted(items, attrgetter("high"))
        hmask1 = [([high], [item.low for item in items]) for high, items in hreduce.items()]
        hmask2 = [
            ([high[0] for high, _ in items], low_mask)
            for low_mask, items in group_by_unsorted(hmask1, itemgetter(1))
        ]

        # try to reduce on low bits
        lreduce: Dict[int, List[Item]] = dict_by_unsorted(items, attrgetter("low"))
        lmask1 = [([item.high for item in items], [low]) for low, items in lreduce.items()]
        lmask2 = [
            (high_mask, [low[0] for _, low in items])
            for high_mask, items in group_by_unsorted(lmask1, itemgetter(0))
        ]

        mask = hmask2 if len(hmask2) < len(lmask2) else lmask2
        cat_shufti_mask = build_nibble_masks(mask, cat_bit)
        n_cat_bits = len(mask)
        if cat_bit + n_cat_bits >= 8:
            print("error: no solution found, try to reduce characters", file=sys.stderr)
            sys.exit(1)

        cat_mask = bit_mask(range(cat_bit, cat_bit + n_cat_bits))

        high_mask = or_shufti_masks(high_mask, cat_shufti_mask[0])
        low_mask = or_shufti_masks(low_mask, cat_shufti_mask[1])
        cat_bit += n_cat_bits
        cat_masks.append(cat_mask)
        if verbose:
            nibble_lists.extend(mask)

    if verbose:
        debug_nibble_lists(nibble_lists)

    return cat_masks, high_mask, low_mask


def main():
    argparser = argparse.ArgumentParser()
    argparser.add_argument("inputs", metavar="INPUT", nargs="+")
    argparser.add_argument("--regex", "-R", action="store_true")
    argparser.add_argument("--format", "-f", type=str, default="rust")
    argparser.add_argument("--verbose", "-v", action="store_true")

    args = argparser.parse_args()
    inputs = args.inputs
    format = args.format
    verbose = args.verbose
    if args.regex:
        regexes = [re.compile(f"[{inp}]") for inp in inputs]
        inputs = [
            "".join([chr(i) for i in range(128) if regex.fullmatch(chr(i))])
            for regex in regexes
        ]

    masks, high, low = build_shufti_masks(inputs, verbose=verbose)
    if format == "rust":
        print(f"let high_nibble_mask: __m128i = _mm_setr_epi8({', '.join([hex(h) for h in high])});")
        print(f"let low_nibble_mask: __m128i = _mm_setr_epi8({', '.join([hex(h) for h in low])});")
        print(f"let category_masks: &[u8] = &[{', '.join([hex(m) for m in masks])}];")
    else:
        print(f"error: unknown format '{format}'", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
