from pathlib import Path

from lxml.etree import parse, tostring

with (Path(__file__).parent / "xmlts20130923" / "xmlconf" / "xmlconf.xml").open("r") as fp:
    tree = parse(fp)


complete = tostring(tree, encoding="utf-8", pretty_print=True)
print(complete.decode("utf-8"))

with (Path(__file__).parent / "xmlts20130923" / "xmlconf" / "xmlconf.complete.xml").open("wb") as fp:
    fp.write(complete)
