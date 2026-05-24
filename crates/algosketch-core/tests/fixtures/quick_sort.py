def quick_sort(items, low, high):
    if low < high:
        pivot: int = partition(items, low, high)
        quick_sort(items, low, pivot - 1)
        quick_sort(items, pivot + 1, high)
    return items

def partition(items, low, high):
    pivot: int = items[high]
    i: int = low
    for j in range(low, high):
        if items[j] < pivot:
            temp: int = items[i]
            items[i] = items[j]
            items[j] = temp
            i = i + 1
    temp: int = items[i]
    items[i] = items[high]
    items[high] = temp
    return i
