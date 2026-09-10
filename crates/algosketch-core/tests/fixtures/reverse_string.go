package main

func reverse_string(text string) string {
	result := ""
	for i := len(text) - 1; i >= 0; i-- {
		result = result + text[i]
	}
	return result
}
