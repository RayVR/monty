# skip=cpython
# invalid format specifier with trailing characters (detected at parse time)
f'{1:10xyz}'
# ParseError=Exc: (<no-tb>) SyntaxError("Invalid format specifier '10xyz'")
