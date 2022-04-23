#!/bin/bash
echo "// name"
python3 genshufmap.py -R "a-zA-Z0-9:\-_"

echo
echo "// whitespace"
python3 genshufmap.py -R " \t\n\r"

echo
echo "// whitespace + char end"
python3 genshufmap.py -R " \t\n\r" "<&\r"

echo
echo "// XML ascii char"
python3 genshufmap.py -v -R "\t\n\r -\x7f"

echo
echo "// char content"
python3 genshufmap.py -v -R "\x09\x0A\x20-%'-;=-\x7f"