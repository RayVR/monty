# mode: iter
# === Basic external function tests ===

# Simple calls
a = add_ints(10, 20)
assert a == 30, 'add_ints basic'

b = add_ints(-5, 15)
assert b == 10, 'add_ints with negative'

s = concat_strings('hello', ' world')
assert s == 'hello world', 'concat_strings basic'

x = return_value(42)
assert x == 42, 'return_value with int'

y = return_value('test')
assert y == 'test', 'return_value with str'

# === Assignment with external calls ===
result = add_ints(100, 200)
assert result == 300, 'assignment from add_ints'

name = concat_strings('foo', 'bar')
assert name == 'foobar', 'assignment from concat_strings'

# === Nested calls ===
nested = add_ints(1, add_ints(2, 3))
assert nested == 6, 'nested add_ints right'

nested2 = add_ints(add_ints(1, 2), 3)
assert nested2 == 6, 'nested add_ints left'

nested3 = add_ints(add_ints(1, 2), add_ints(3, 4))
assert nested3 == 10, 'nested add_ints both'

deep = add_ints(add_ints(add_ints(1, 2), 3), 4)
assert deep == 10, 'deeply nested add_ints'

# === Chained operations ===
chained = add_ints(1, 2) + add_ints(3, 4)
assert chained == 10, 'chained add_ints with +'

chained2 = add_ints(10, 20) - add_ints(5, 10)
assert chained2 == 15, 'chained add_ints with -'

chained3 = add_ints(2, 3) * add_ints(4, 5)
assert chained3 == 45, 'chained add_ints with *'

str_chain = concat_strings('a', 'b') + concat_strings('c', 'd')
assert str_chain == 'abcd', 'chained concat_strings'

# === External calls in assert statements ===
assert add_ints(5, 5) == 10, 'ext call in assert condition'
assert return_value(True), 'ext call returning truthy in assert'
assert concat_strings('x', 'y') == 'xy', 'concat in assert'
assert add_ints(1, add_ints(2, 3)) == 6, 'nested ext call in assert'

# === Mixed with builtins ===
length = len(concat_strings('hello', 'world'))
assert length == 10, 'len of concat result'

items = [add_ints(1, 2), add_ints(3, 4)]
assert items[0] == 3, 'ext call in list literal first'
assert items[1] == 7, 'ext call in list literal second'
