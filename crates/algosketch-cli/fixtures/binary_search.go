package main

func binary_search(items []int, target int) int {
	low := 0
	high := len(items) - 1
	for low <= high {
		mid := (low + high) / 2
		if items[mid] == target {
			return mid
		} else if items[mid] < target {
			low = mid + 1
		} else {
			high = mid - 1
		}
	}
	return -1
}
