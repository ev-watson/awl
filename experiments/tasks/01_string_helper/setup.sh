#!/usr/bin/env bash
set -euo pipefail
SANDBOX="experiments/sandbox/01"
rm -rf "$SANDBOX"
mkdir -p "$SANDBOX"
cat >"$SANDBOX/textops.py" <<'PY'
def capitalize_each_line(text: str) -> str:
    raise NotImplementedError
PY
cat >"$SANDBOX/test_textops.py" <<'PY'
import unittest
from textops import capitalize_each_line


class CapitalizeEachLine(unittest.TestCase):
    def test_basic(self):
        self.assertEqual(capitalize_each_line("hello\nworld"), "Hello\nWorld")

    def test_preserves_blank_lines(self):
        self.assertEqual(capitalize_each_line("a\n\nb"), "A\n\nB")

    def test_preserves_trailing_newline(self):
        self.assertEqual(capitalize_each_line("foo\n"), "Foo\n")

    def test_empty_input(self):
        self.assertEqual(capitalize_each_line(""), "")


if __name__ == "__main__":
    unittest.main()
PY
