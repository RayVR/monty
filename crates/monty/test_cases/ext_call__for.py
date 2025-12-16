# mode: iter
# === External calls in for loops ===

# Ext call in loop body
total = 0
for i in range(3):
    total = add_ints(total, 1)
assert total == 3, 'ext call accumulator in loop'

# Ext call with loop variable
sum_val = 0
for i in range(4):
    sum_val = add_ints(sum_val, i)
assert sum_val == 6, 'ext call with loop var'

# Multiple ext calls per iteration
result = 0
for i in range(3):
    result = add_ints(result, add_ints(i, i))
assert result == 6, 'nested ext calls in loop'

# Building list with ext calls
items = []
for i in range(3):
    items.append(add_ints(i, 10))
assert items[0] == 10, 'ext call list build first'
assert items[1] == 11, 'ext call list build second'
assert items[2] == 12, 'ext call list build third'

# Chained ext calls in loop
acc = 0
for i in range(3):
    acc = add_ints(acc, 1) + add_ints(0, 1)
assert acc == 6, 'chained ext calls in loop body'

# Nested loops with ext calls
matrix_sum = 0
for i in range(2):
    for j in range(2):
        matrix_sum = add_ints(matrix_sum, add_ints(i, j))
assert matrix_sum == 4, 'ext calls in nested loops'
