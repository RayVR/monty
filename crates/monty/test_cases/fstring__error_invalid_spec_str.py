# skip=cpython
# invalid format specifier for string (detected at parse time)
f'{"hello":abc}'
# ParseError=Exc: (<no-tb>) SyntaxError("Invalid format specifier 'abc'")
