# Test that returning a cyclic dict doesn't crash (PyObject cycle detection)
d = {}
d['self'] = d
d
# Return={'self': {...}}
