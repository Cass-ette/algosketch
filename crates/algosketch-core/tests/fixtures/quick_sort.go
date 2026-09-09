package main

func quick_sort(items []int, low int, high int) []int {
	if low < high {
		pivot := partition(items, low, high)
		quick_sort(items, low, pivot-1)
		quick_sort(items, pivot+1, high)
	}
	return items
}

func partition(items []int, low int, high int) int {
	pivot := items[high]
	i := low
	for j := low; j < high; j++ {
		if items[j] < pivot {
			temp := items[i]
			items[i] = items[j]
			items[j] = temp
			i = i + 1
		}
	}
	temp := items[i]
	items[i] = items[high]
	items[high] = temp
	return i
}
