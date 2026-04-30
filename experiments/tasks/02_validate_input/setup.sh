#!/usr/bin/env bash
set -euo pipefail
SANDBOX="experiments/sandbox/02"
rm -rf "$SANDBOX"
mkdir -p "$SANDBOX"
cat >"$SANDBOX/validators.py" <<'PY'
def validate_email(addr: str) -> bool:
    raise NotImplementedError
PY
cat >"$SANDBOX/test_validators.py" <<'PY'
import unittest
from validators import validate_email


class ValidateEmail(unittest.TestCase):
    def test_valid(self):
        self.assertTrue(validate_email("a@b.co"))
        self.assertTrue(validate_email("user.name@domain.example"))

    def test_missing_at(self):
        self.assertFalse(validate_email("nodomain.com"))

    def test_two_ats(self):
        self.assertFalse(validate_email("a@b@c.com"))

    def test_missing_dot_after_at(self):
        self.assertFalse(validate_email("a@bcom"))

    def test_empty_local_part(self):
        self.assertFalse(validate_email("@b.co"))

    def test_whitespace(self):
        self.assertFalse(validate_email("a @b.co"))
        self.assertFalse(validate_email("a@b .co"))

    def test_too_short(self):
        self.assertFalse(validate_email("a@b"))

    def test_empty(self):
        self.assertFalse(validate_email(""))


if __name__ == "__main__":
    unittest.main()
PY
