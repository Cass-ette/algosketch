def binary_search(items, target):
    low: int = 0
    high: int = len(items) - 1
    while low <= high:
        mid: int = (low + high) // 2
        if items[mid] == target:
            return mid
        elif items[mid] < target:
            low = mid + 1
        else:
            high = mid - 1
    return -1
