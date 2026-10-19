"""Stubs for sundgren._native.cli."""

import argparse

EXIT_OK: int
EXIT_FAILURE: int
EXIT_USAGE: int
EXIT_PATCH: int

def run_command(args: argparse.Namespace) -> int: ...
