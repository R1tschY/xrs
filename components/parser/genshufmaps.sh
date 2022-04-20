#!/bin/bash
echo "// name"
python3 genshufmap.py -R "a-zA-Z0-9:\-_"

echo "// whitespace + char end"
python3 genshufmap.py -R " \t\n\r" "<&\r"

echo "// XML ascii char"
python3 genshufmap.py -v -R "\t\n\r -\x7f"