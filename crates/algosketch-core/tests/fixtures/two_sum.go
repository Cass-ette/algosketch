package main

func two_sum(items []int, target int) int {
	for i := 0; i < len(items); i++ {
		for j := i + 1; j < len(items); j++ {
			if items[i]+items[j] == target {
				return i
			}
		}
	}
	return -1
}
