package main

func reverse_linked_list(head *Node) *Node {
	var previous *Node = nil
	current := head
	for current != nil {
		next_node := current.next
		current.next = previous
		previous = current
		current = next_node
	}
	return previous
}
