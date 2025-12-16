# Test that returning a cyclic list doesn't crash (PyObject cycle detection)
a = []
a.append(a)
a
# Return=[[...]]
