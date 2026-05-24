def reverse_linked_list(head):
    previous: Node = None
    current: Node = head
    while current != None:
        next_node: Node = current.next
        current.next = previous
        previous = current
        current = next_node
    return previous
