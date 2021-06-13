from dataclasses import dataclass


@dataclass
class Cat:
    chars: str
    bit: int


cats = [
    Cat(chars=" \t\n\r", bit=0),
    Cat(chars="<>", bit=1),
    Cat(chars="&;", bit=2),
    Cat(chars="\"", bit=3),
    Cat(chars="=", bit=4),
]

low_mask = [0] * 16
high_mask = [0] * 16

for cat in cats:
    for c in cat.chars:
        low_mask[ord(c) & 0x0f] |= (1 << cat.bit)
        high_mask[ord(c) >> 4] |= (1 << cat.bit)


print(f"low_mask = {', '.join([hex(x) for x in low_mask])}")
print(f"high_mask = {', '.join([hex(x) for x in high_mask])}")

