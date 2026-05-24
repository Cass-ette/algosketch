def two_sum(items, target):
    for i in range(0, len(items)):
        for j in range(i + 1, len(items)):
            if items[i] + items[j] == target:
                return i
    return -1
