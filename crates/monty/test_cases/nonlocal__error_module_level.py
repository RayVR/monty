# nonlocal at module level is a syntax error
nonlocal x  # type: ignore[reportUndefinedVariable]
# ParseError=Exc: (<no-tb>) SyntaxError('nonlocal declaration not allowed at module level')
