# xfail=monty
import sys

assert sys.version_info[:2] == (3, 14), f'Expected Python 3.14, got {sys.version_info[:2]}'
